use anyhow::Result;
use std::time::Instant;

use super::TuiFrontend;
use crate::frontend::Frontend;
use crate::session_manager::SessionManager;
use crate::session::{ConnectionMode, SessionStatus};

/// Spawn a Lich connection that auto-reconnects on disconnect/error.
/// Retries every `retry_secs` seconds, up to `max_retries` times (0 = unlimited).
fn spawn_lich_reconnect(
    host: String,
    port: u16,
    login_key: Option<String>,
    server_tx: tokio::sync::mpsc::UnboundedSender<crate::network::ServerMessage>,
    command_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
    raw_logger: Option<crate::network::RawLogger>,
    retry_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // command_rx can only be consumed once; wrap in Option so we can move it on first use
        // and create a dummy channel for subsequent reconnects (commands go to the live session)
        let mut first_rx = Some(command_rx);
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let rx = first_rx.take().unwrap_or_else(|| {
                // Subsequent reconnects: create a throwaway receiver; the real command_tx
                // is held by the session and will be re-wired in a future Phase.
                tokio::sync::mpsc::unbounded_channel::<String>().1
            });
            match crate::network::LichConnection::start(
                &host, port, login_key.clone(), server_tx.clone(), rx, raw_logger.clone(),
            ).await {
                Ok(()) => {
                    tracing::info!("Lich connection closed cleanly (attempt {})", attempt);
                }
                Err(e) => {
                    tracing::warn!("Lich connection lost (attempt {}): {}", attempt, e);
                }
            }
            tracing::info!("Reconnecting to {}:{} in {}s...", host, port, retry_secs);
            tokio::time::sleep(std::time::Duration::from_secs(retry_secs)).await;
        }
    })
}

/// Run the TUI frontend with the given configuration.
/// This is the main entry point for TUI mode.
pub fn run(
    config: crate::config::Config,
    character: Option<String>,
    direct: Option<crate::network::DirectConnectConfig>,
    setup_palette: bool,
    login_key: Option<String>,
) -> Result<()> {
    // Use tokio runtime for async network I/O
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run(config, character, direct, setup_palette, login_key))
}

/// Async TUI main loop with network support
async fn async_run(
    config: crate::config::Config,
    character: Option<String>,
    direct: Option<crate::network::DirectConnectConfig>,
    setup_palette: bool,
    login_key: Option<String>,
) -> Result<()> {
    use crate::core::AppCore;
    use crate::network::{DirectConnection, LichConnection, ServerMessage};
    use tokio::sync::mpsc;

    // Create channels for network communication
    let (server_tx, mut server_rx) = mpsc::unbounded_channel::<ServerMessage>();
    let (command_tx, command_rx) = mpsc::unbounded_channel::<String>();

    // Store connection info
    let host = config.connection.host.clone();
    let port = config.connection.port;

    // Set global color mode BEFORE creating frontend or any widgets
    // This ensures ALL color parsing respects the mode from config
    let raw_logger = match crate::network::RawLogger::new(&config) {
        Ok(logger) => logger,
        Err(e) => {
            tracing::error!("Failed to initialize raw logger: {}", e);
            None
        }
    };

    // Create core application state
    let mut app_core = AppCore::new(config)?;

    super::colors::set_global_color_mode(app_core.config.ui.color_mode);

    // Initialize palette lookup for Slot mode
    // This builds the hex→slot mapping from color_palette entries
    if app_core.config.ui.color_mode == crate::config::ColorMode::Slot {
        super::colors::init_palette_lookup(&app_core.config.colors.color_palette);
    }

    // Create TUI frontend
    let mut frontend = TuiFrontend::new()?;

    // Restore window position for this character (if saved)
    if let Some(positioner) = crate::window_position::create_positioner() {
        if let Ok(Some(saved)) = crate::window_position::load(character.as_deref()) {
            use crate::window_position::WindowPositionerExt;
            let rect = if positioner.is_visible(&saved.window) {
                saved.window
            } else {
                // Clamp to visible area if monitors changed
                match positioner.clamp_to_screen(&saved.window) {
                    Ok(clamped) => clamped,
                    Err(_) => saved.window,
                }
            };
            if let Err(e) = positioner.set_position(&rect) {
                tracing::debug!("Failed to restore window position: {}", e);
            }
        }
    }

    // Ensure frontend theme cache matches whatever layout/theme AppCore activated
    let initial_theme_id = app_core.config.active_theme.clone();
    let initial_theme = app_core.config.get_theme();
    frontend.update_theme_cache(initial_theme_id, initial_theme);

    // Initialize command input widget BEFORE any rendering
    // This ensures it exists when we start routing keys to it
    frontend.ensure_command_input_exists("command_input");

    // Setup palette if requested via --setup-palette flag
    if setup_palette {
        if let Err(e) = frontend.execute_setpalette(&app_core) {
            tracing::warn!("Failed to setup palette: {}", e);
        } else {
            tracing::info!("Terminal palette loaded from color_palette");
        }
    }

    // Get terminal size and initialize windows
    let (width, height) = frontend.size();
    app_core.init_windows(width, height);

    // Initial render to create widgets (needed before loading history)
    frontend.render(&mut app_core)?;

    // Load command history (must be after widgets are created)
    if let Err(e) = frontend.command_input_load_history("command_input", character.as_deref()) {
        tracing::warn!("Failed to load command history: {}", e);
    }

    // Play startup music if enabled (with optional delay)
    if app_core.config.sound.startup_music {
        if let Some(ref player) = app_core.sound_player {
            let delay_ms = app_core.config.sound.startup_music_delay_ms;
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            if let Err(e) = player.play_from_sounds_dir("wizard_music", None) {
                tracing::debug!("Startup music not available: {}", e);
            }
        }
    }

    if direct.is_none() {
        app_core.seed_default_quickbars_if_empty();
        if app_core
            .ui_state
            .get_window_by_type(crate::data::window::WidgetType::Spells, None)
            .is_some()
        {
            let command = "_spell _spell_update_links\n".to_string();
            app_core.message_processor.skip_next_spells_clear();
            app_core
                .perf_stats
                .record_bytes_sent((command.len() + 1) as u64);
            let _ = command_tx.send(command);
        }
    }

    // Spawn network connection task (with auto-reconnect for Lich)
    let _network_handle = match direct {
        Some(cfg) => tokio::spawn(async move {
            if let Err(e) = DirectConnection::start(cfg, server_tx, command_rx, raw_logger).await {
                tracing::error!(error = ?e, "Network connection error");
            }
        }),
        None => {
            let host_clone = host.clone();
            let login_key_clone = login_key.clone();
            spawn_lich_reconnect(host_clone, port, login_key_clone, server_tx, command_rx, raw_logger, 5)
        }
    };

    // Session manager — owns all sessions
    let mut session_manager = SessionManager::new();

    // Load persisted session list
    let mut sessions_config = crate::sessions_config::SessionsConfig::load().unwrap_or_default();

    // Seed with the initial session derived from CLI args
    let initial_mode = ConnectionMode::LichProxy { host: host.clone(), port };
    let initial_label = character.clone().unwrap_or_else(|| format!("{}:{}", host, port));
    let initial_id = session_manager.add(initial_label.clone(), initial_mode);
    if let Some(s) = session_manager.get_mut(initial_id) {
        s.status = SessionStatus::Connected;
        s.command_tx = Some(command_tx.clone());
    }

    // If no sessions are saved yet, show the picker
    if sessions_config.sessions.is_empty() {
        frontend.session_picker = Some(
            crate::frontend::tui::session_picker::SessionPicker::new(&sessions_config)
        );
    }

    // Sync tab bar labels into frontend
    let sync_tabs = |mgr: &SessionManager, fe: &mut TuiFrontend| {
        fe.session_labels = mgr.all().iter().map(|s| {
            (s.label.clone(), mgr.active().map_or(false, |a| a.id == s.id), s.is_connected(), s.unread_count)
        }).collect();
    };
    sync_tabs(&session_manager, &mut frontend);

    // Track time for periodic countdown updates
    let mut last_countdown_update = std::time::Instant::now();

    // Main event loop
    while app_core.running {
        // Poll for frontend events (keyboard, mouse, resize)
        let events = frontend.poll_events()?;
        app_core
            .perf_stats
            .record_event_queue_depth(events.len() as u64);

        // Poll TTS callback events for auto-play
        app_core.poll_tts_events();

        // Process frontend events
        for event in events {
            let event_start = Instant::now();
            // Handle events that need frontend access directly
            match &event {
                crate::frontend::FrontendEvent::Mouse(mouse_event) => {
                    // Phase 4.1: Delegate to TuiFrontend::handle_mouse_event
                    let (handled, command) = frontend.handle_mouse_event(
                        mouse_event,
                        &mut app_core,
                        crate::frontend::tui::menu_actions::handle_menu_action,
                    )?;

                    if let Some(cmd) = command {
                        app_core.perf_stats.record_bytes_sent((cmd.len() + 1) as u64);
                        let _ = command_tx.send(cmd);
                    }

                    if handled {
                        continue;
                    }
                }
                crate::frontend::FrontendEvent::Key { code: _code, modifiers: _modifiers } => {
                    // Key events are handled in handle_event()
                    // No early intercepts - let the 3-layer routing handle everything
                }
                _ => {}
            }

            if let Some(command) = handle_event(&mut app_core, &mut frontend, event)? {
                // Intercept wizard commands
                if command.starts_with("//wizard:") {
                    handle_wizard_command(&command, &mut frontend, &mut session_manager, &mut sessions_config, &command_tx);
                    sync_tabs(&session_manager, &mut frontend);
                // Intercept picker commands
                } else if command.starts_with("//picker:") {
                    handle_picker_command(
                        &command,
                        &mut frontend,
                        &mut session_manager,
                        &mut sessions_config,
                        &command_tx,
                    );
                    sync_tabs(&session_manager, &mut frontend);
                // Intercept session commands before sending to game server
                } else if crate::frontend::tui::session_keys::is_session_cmd(&command) {
                    use crate::frontend::tui::session_keys::SessionCmd;
                    if let Some(cmd) = SessionCmd::parse(&command) {
                        match cmd {
                            SessionCmd::SwitchToIndex(i) => session_manager.set_active_by_index(i),
                            SessionCmd::Next => session_manager.next(),
                            SessionCmd::Prev => session_manager.prev(),
                            SessionCmd::ToggleCompact => frontend.compact_tabs = !frontend.compact_tabs,
                            SessionCmd::New | SessionCmd::Close => {} // Phase 2
                        }
                        sync_tabs(&session_manager, &mut frontend);
                    }
                } else {
                    app_core.perf_stats.record_bytes_sent((command.len() + 1) as u64);
                    let _ = command_tx.send(command);
                }
            }

            let duration = event_start.elapsed();
            app_core.perf_stats.record_event_process_time(duration);

            // Process pending window additions after event handling (for .testline)
            let (term_width, term_height) = frontend.size();
            app_core.process_pending_window_additions(term_width, term_height);
        }

        // Poll for server messages (non-blocking)
        while let Ok(msg) = server_rx.try_recv() {
            match msg {
                ServerMessage::Text(line) => {
                    app_core
                        .perf_stats
                        .record_bytes_received((line.len() + 1) as u64);
                    let parse_start = Instant::now();
                    // Process incoming server data through parser
                    if let Err(e) = app_core.process_server_data(&line) {
                        tracing::error!("Error processing server data: {}", e);
                    }
                    let parse_duration = parse_start.elapsed();
                    app_core.perf_stats.record_parse(parse_duration);

                    // Adjust content-driven window sizes (e.g., Betrayer auto-resize)
                    app_core.adjust_content_driven_windows();

                    // Play queued sounds from highlight processing
                    for sound in app_core.game_state.drain_sound_queue() {
                        if let Some(ref player) = app_core.sound_player {
                            if let Err(e) = player.play_from_sounds_dir(&sound.file, sound.volume) {
                                tracing::warn!("Failed to play sound '{}': {}", sound.file, e);
                            }
                        }
                    }

                    // Container discovery: auto-create window for new containers
                    if app_core.ui_state.container_discovery_mode {
                        if let Some((id, title)) =
                            app_core.message_processor.newly_registered_container.take()
                        {
                            tracing::info!(
                                "Container discovery: creating window for '{}' (id={})",
                                title,
                                id
                            );
                            let (term_width, term_height) = frontend.size();
                            app_core.create_ephemeral_container_window(
                                &title,
                                term_width,
                                term_height,
                            );
                        }
                    } else {
                        // Clear any pending signal if discovery mode is off
                        app_core.message_processor.newly_registered_container = None;
                    }

                    // Process pending window additions from openDialog events
                    let (term_width, term_height) = frontend.size();
                    app_core.process_pending_window_additions(term_width, term_height);
                }
                ServerMessage::Connected => {
                    tracing::info!("Connected to game server");
                    app_core.game_state.connected = true;
                    app_core.needs_render = true;
                }
                ServerMessage::Disconnected => {
                    tracing::info!("Disconnected from game server");
                    app_core.game_state.connected = false;
                    app_core.needs_render = true;
                }
            }
        }

        // Force render every second for countdown widgets
        if last_countdown_update.elapsed().as_secs() >= 1 {
            app_core.needs_render = true;
            last_countdown_update = std::time::Instant::now();
        }

        // Sample system/process metrics (rate-limited internally)
        app_core.perf_stats.sample_sysinfo();

        // Reset widget caches if layout was reloaded
        if app_core.ui_state.needs_widget_reset {
            frontend.widget_manager.clear();
            app_core.ui_state.needs_widget_reset = false;
            tracing::debug!("Widget caches cleared after layout reload");
        }

        // Reset specific widgets (e.g., when widget type changes)
        if !app_core.ui_state.widgets_to_reset.is_empty() {
            for name in app_core.ui_state.widgets_to_reset.drain(..) {
                frontend.widget_manager.remove_widget_from_all_caches(&name);
                tracing::debug!("Reset widget cache for '{}' (type change)", name);
            }
        }

        // Render if needed
        if app_core.needs_render {
            frontend.render(&mut app_core)?;
            app_core.needs_render = false;
        }

        // No sleep needed - event::poll() timeout already limits frame rate to ~60 FPS
    }

    // Save command history
    if let Err(e) = frontend.command_input_save_history("command_input", character.as_deref()) {
        tracing::warn!("Failed to save command history: {}", e);
    }

    // Save window position for this character
    if let Some(positioner) = crate::window_position::create_positioner() {
        if let Ok(rect) = positioner.get_position() {
            if let Ok(screens) = positioner.get_screen_bounds() {
                let config = crate::window_position::WindowPositionConfig {
                    window: rect,
                    monitors: screens,
                };
                if let Err(e) = crate::window_position::save(character.as_deref(), &config) {
                    tracing::warn!("Failed to save window position: {}", e);
                }
            }
        }
    }

    // Cleanup
    frontend.cleanup()?;

    Ok(())
}

/// Handle a frontend event
/// Returns Some(command) if a command should be sent to the server
fn handle_event(
    app_core: &mut crate::core::AppCore,
    frontend: &mut TuiFrontend,
    event: crate::frontend::FrontendEvent,
) -> Result<Option<String>> {
    use crate::frontend::FrontendEvent;

    match event {
        FrontendEvent::Key { code, modifiers } => {
            // Phase 4.2: Delegate all keyboard handling to TuiFrontend::handle_key_event()
            return frontend.handle_key_event(
                code,
                modifiers,
                app_core,
                crate::frontend::tui::menu_actions::handle_menu_action,
            );
        }
        FrontendEvent::Resize { width, height } => {
            // DISABLED: Automatic resize on terminal resize (manual .resize command only)
            tracing::info!(
                "Terminal resized to {}x{} (auto-resize disabled, use .resize command)",
                width,
                height
            );
        }
        _ => {}
    }

    Ok(None)
}

/// Handle a `//picker:*` command produced by the session picker input handler.
fn handle_picker_command(
    command: &str,
    frontend: &mut TuiFrontend,
    session_manager: &mut SessionManager,
    sessions_config: &mut crate::sessions_config::SessionsConfig,
    command_tx: &tokio::sync::mpsc::UnboundedSender<String>,
) {
    use crate::frontend::tui::session_picker::PickerAction;

    // Take the action out of the picker so we can act on it
    let action = frontend.session_picker.as_mut().and_then(|p| p.action.take());

    match command {
        "//picker:quit" => {
            // No sessions and user pressed Escape — close picker, let app exit naturally
            frontend.session_picker = None;
        }
        "//picker:open_wizard" => {
            // Open the Direct login wizard
            if let Some(picker) = &mut frontend.session_picker {
                picker.focus = crate::frontend::tui::session_picker::PickerFocus::List;
                picker.form = None;
            }
            frontend.login_wizard = Some(crate::frontend::tui::login_wizard::LoginWizard::new());
        }
        "//picker:add" => {
            if let Some(PickerAction::AddSession(entry)) = action {
                // Add to session manager
                let mode = crate::session::ConnectionMode::LichProxy {
                    host: entry.host.clone().unwrap_or_else(|| "localhost".to_string()),
                    port: entry.port.unwrap_or(8000),
                };
                let id = session_manager.add(entry.label.clone(), mode);
                if let Some(s) = session_manager.get_mut(id) {
                    s.command_tx = Some(command_tx.clone());
                }
                // Persist
                sessions_config.add(entry);
                let _ = sessions_config.save();
                // Update picker list and close form
                if let Some(picker) = &mut frontend.session_picker {
                    picker.sessions = sessions_config.sessions.clone();
                    picker.focus = crate::frontend::tui::session_picker::PickerFocus::List;
                    picker.form = None;
                }
            }
        }
        cmd if cmd.starts_with("//picker:connect:") => {
            if let Ok(idx) = cmd["//picker:connect:".len()..].parse::<usize>() {
                // Switch to that session and dismiss picker
                session_manager.set_active_by_index(idx);
                frontend.session_picker = None;
            }
        }
        cmd if cmd.starts_with("//picker:remove:") => {
            if let Ok(idx) = cmd["//picker:remove:".len()..].parse::<usize>() {
                if let Some(entry) = sessions_config.sessions.get(idx) {
                    let label = entry.label.clone();
                    sessions_config.remove(&label);
                    let _ = sessions_config.save();
                }
                // Update picker list
                if let Some(picker) = &mut frontend.session_picker {
                    picker.sessions = sessions_config.sessions.clone();
                    if picker.selected >= picker.sessions.len() {
                        picker.selected = picker.sessions.len().saturating_sub(1);
                    }
                }
            }
        }
        _ => {}
    }
}


/// Handle `//wizard:*` commands from the login wizard.
fn handle_wizard_command(
    command: &str,
    frontend: &mut TuiFrontend,
    session_manager: &mut SessionManager,
    sessions_config: &mut crate::sessions_config::SessionsConfig,
    command_tx: &tokio::sync::mpsc::UnboundedSender<String>,
) {
    match command {
        "//wizard:cancel" => {
            frontend.login_wizard = None;
        }
        "//wizard:fetch_chars" => {
            // Fetch character list from eAccess in a blocking thread, then set on wizard
            if let Some(wizard) = &frontend.login_wizard {
                let account = wizard.account.clone();
                let password = wizard.password.clone();
                let game_code = wizard.selected_game_code().to_string();
                let data_dir = crate::config::Config::base_dir().unwrap_or_default();
                match std::thread::spawn(move || {
                    crate::network::fetch_characters_for_account(&account, &password, &game_code, &data_dir)
                }).join() {
                    Ok(Ok(chars)) => {
                        if let Some(w) = &mut frontend.login_wizard {
                            w.set_characters(chars);
                        }
                    }
                    Ok(Err(e)) => {
                        if let Some(w) = &mut frontend.login_wizard {
                            w.set_error(format!("Auth failed: {}", e));
                        }
                    }
                    Err(_) => {
                        if let Some(w) = &mut frontend.login_wizard {
                            w.set_error("Connection error".to_string());
                        }
                    }
                }
            }
        }
        cmd if cmd.starts_with("//wizard:connect:") => {
            // Format: //wizard:connect:account:password:game_code:character
            let parts: Vec<&str> = cmd["//wizard:connect:".len()..].splitn(4, ':').collect();
            if parts.len() == 4 {
                let (account, password, game_code, character) =
                    (parts[0], parts[1], parts[2], parts[3]);
                // Add as Direct session
                let mode = crate::session::ConnectionMode::Direct {
                    account: account.to_string(),
                    password: password.to_string(),
                    character: character.to_string(),
                    game_code: game_code.to_string(),
                };
                let id = session_manager.add(character.to_string(), mode);
                if let Some(s) = session_manager.get_mut(id) {
                    s.command_tx = Some(command_tx.clone());
                }
                // Save to sessions.toml (no password stored)
                let entry = crate::sessions_config::SessionEntry {
                    label: character.to_string(),
                    mode: crate::sessions_config::SessionModeConfig::Direct,
                    host: None,
                    port: None,
                    account: Some(account.to_string()),
                    character: Some(character.to_string()),
                    game_code: Some(game_code.to_string()),
                    auto_connect: false,
                };
                sessions_config.add(entry);
                let _ = sessions_config.save();
                // Close wizard and picker
                frontend.login_wizard = None;
                frontend.session_picker = None;
            }
        }
        _ => {}
    }
}
