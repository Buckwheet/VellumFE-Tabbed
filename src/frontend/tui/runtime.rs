use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::TuiFrontend;
use crate::frontend::Frontend;
use crate::session::{ConnectionMode, SessionId, SessionStatus};
use crate::session_manager::SessionManager;

/// Spawn a Lich connection that auto-reconnects on disconnect/error.
/// `unread` is incremented for each text line received (used for badge on inactive sessions).
fn spawn_lich_reconnect(
    host: String,
    port: u16,
    login_key: Option<String>,
    server_tx: tokio::sync::mpsc::UnboundedSender<crate::network::ServerMessage>,
    command_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
    raw_logger: Option<crate::network::RawLogger>,
    retry_secs: u64,
    unread: Arc<AtomicUsize>,
    is_active: Arc<AtomicUsize>, // 1 = active session, 0 = background
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut first_rx = Some(command_rx);
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let rx = first_rx
                .take()
                .unwrap_or_else(|| tokio::sync::mpsc::unbounded_channel::<String>().1);
            // Wrap server_tx to intercept Text messages for unread counting
            let (wrapped_tx, mut wrapped_rx) =
                tokio::sync::mpsc::unbounded_channel::<crate::network::ServerMessage>();
            let unread_clone = unread.clone();
            let is_active_clone = is_active.clone();
            let real_tx = server_tx.clone();
            tokio::spawn(async move {
                while let Some(msg) = wrapped_rx.recv().await {
                    if let crate::network::ServerMessage::Text(_) = &msg {
                        if is_active_clone.load(Ordering::Relaxed) == 0 {
                            unread_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    let _ = real_tx.send(msg);
                }
            });
            match crate::network::LichConnection::start(
                &host,
                port,
                login_key.clone(),
                wrapped_tx,
                rx,
                raw_logger.clone(),
            )
            .await
            {
                Ok(()) => tracing::info!("Lich connection closed cleanly (attempt {})", attempt),
                Err(e) => tracing::warn!("Lich connection lost (attempt {}): {}", attempt, e),
            }
            tracing::info!("Reconnecting to {}:{} in {}s...", host, port, retry_secs);
            tokio::time::sleep(std::time::Duration::from_secs(retry_secs)).await;
        }
    })
}

/// Swap WidgetManagers on session switch: save outgoing, restore incoming.
fn do_session_switch(
    prev_id: Option<crate::session::SessionId>,
    new_id: crate::session::SessionId,
    frontend: &mut crate::frontend::tui::TuiFrontend,
    widget_managers: &mut HashMap<
        crate::session::SessionId,
        crate::frontend::tui::widget_manager::WidgetManager,
    >,
) {
    if prev_id == Some(new_id) {
        return;
    }
    // Save outgoing session's widget state
    if let Some(prev) = prev_id {
        let outgoing = frontend
            .swap_widget_manager(crate::frontend::tui::widget_manager::WidgetManager::new());
        widget_managers.insert(prev, outgoing);
    }
    // Restore incoming session's widget state (or use the fresh one just swapped in)
    if let Some(incoming) = widget_managers.remove(&new_id) {
        let _ = frontend.swap_widget_manager(incoming);
    }
}

/// Spawn the network task for a session using its stored server_tx.
/// Returns the command_tx so the caller can store it on the session.
fn spawn_session_network(
    session: &mut crate::session::Session,
    raw_logger: Option<crate::network::RawLogger>,
) -> Option<tokio::sync::mpsc::UnboundedSender<String>> {
    let server_tx = session.server_tx.clone()?;
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let unread = session.unread.clone();
    let active_id = session.active_session_id.clone();
    match &session.mode {
        ConnectionMode::LichProxy {
            host,
            port,
            login_key,
        } => {
            spawn_lich_reconnect(
                host.clone(),
                *port,
                login_key.clone(),
                server_tx,
                command_rx,
                raw_logger,
                5,
                unread,
                active_id,
            );
        }
        ConnectionMode::Direct {
            account,
            password,
            character,
            game_code,
        } => {
            let cfg = crate::network::DirectConnectConfig {
                account: account.clone(),
                password: password.clone(),
                character: character.clone(),
                game_code: game_code.clone(),
                data_dir: crate::config::Config::base_dir().unwrap_or_default(),
            };
            tokio::spawn(async move {
                if let Err(e) =
                    crate::network::DirectConnection::start(cfg, server_tx, command_rx, raw_logger)
                        .await
                {
                    tracing::error!("Direct connection error: {:#}", e);
                }
            });
        }
    }
    session.status = SessionStatus::Connecting;
    Some(command_tx)
}

/// Create an AppCore for a session given its connection mode.
fn create_app_core_for_session(
    mode: &ConnectionMode,
    base_config: &crate::config::Config,
) -> Result<crate::core::AppCore> {
    use crate::core::AppCore;
    let character = match mode {
        ConnectionMode::Direct { character, .. } => Some(character.as_str()),
        ConnectionMode::LichProxy { .. } => None,
    };
    let port = base_config.connection.port;
    let config = crate::config::Config::load_with_options(character, port)
        .unwrap_or_else(|_| base_config.clone());
    AppCore::new(config)
}

/// Run the TUI frontend with the given configuration.
pub fn run(
    config: crate::config::Config,
    character: Option<String>,
    direct: Option<crate::network::DirectConnectConfig>,
    setup_palette: bool,
    login_key: Option<String>,
) -> Result<()> {
    // Use tokio runtime for async network I/O
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run(
        config,
        character,
        direct,
        setup_palette,
        login_key,
    ))
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
    use crate::network::ServerMessage;
    use tokio::sync::mpsc;

    // Per-session server_rx map — keyed by SessionId
    let mut session_rxs: HashMap<SessionId, mpsc::UnboundedReceiver<ServerMessage>> =
        HashMap::new();
    // Per-session AppCore map — keyed by SessionId
    let mut app_cores: HashMap<SessionId, AppCore> = HashMap::new();
    // Per-session WidgetManager map — swapped in/out of frontend on session switch
    let mut widget_managers: HashMap<
        SessionId,
        crate::frontend::tui::widget_manager::WidgetManager,
    > = HashMap::new();

    // Dummy command channel (replaced per-session by spawn_session_network)
    let (command_tx, _command_rx_unused) = mpsc::unbounded_channel::<String>();

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

    // Create initial AppCore from CLI config (used for startup and as base for new sessions)
    let mut initial_app_core = AppCore::new(config.clone())?;

    super::colors::set_global_color_mode(initial_app_core.config.ui.color_mode);

    // Initialize palette lookup for Slot mode
    // This builds the hex→slot mapping from color_palette entries
    if initial_app_core.config.ui.color_mode == crate::config::ColorMode::Slot {
        super::colors::init_palette_lookup(&initial_app_core.config.colors.color_palette);
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
    let initial_theme_id = initial_app_core.config.active_theme.clone();
    let initial_theme = initial_app_core.config.get_theme();
    frontend.update_theme_cache(initial_theme_id, initial_theme);

    // Initialize command input widget BEFORE any rendering
    // This ensures it exists when we start routing keys to it
    frontend.ensure_command_input_exists("command_input");

    // Setup palette if requested via --setup-palette flag
    if setup_palette {
        if let Err(e) = frontend.execute_setpalette(&initial_app_core) {
            tracing::warn!("Failed to setup palette: {}", e);
        } else {
            tracing::info!("Terminal palette loaded from color_palette");
        }
    }

    // Get terminal size and initialize windows
    let (width, height) = frontend.size();
    initial_app_core.init_windows(width, height);

    // Initial render to create widgets (needed before loading history)
    frontend.render(&mut initial_app_core)?;

    // Load command history (must be after widgets are created)
    if let Err(e) = frontend.command_input_load_history("command_input", character.as_deref()) {
        tracing::warn!("Failed to load command history: {}", e);
    }

    // Play startup music if enabled (with optional delay)
    if initial_app_core.config.sound.startup_music {
        if let Some(ref player) = initial_app_core.sound_player {
            let delay_ms = initial_app_core.config.sound.startup_music_delay_ms;
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            if let Err(e) = player.play_from_sounds_dir("wizard_music", None) {
                tracing::debug!("Startup music not available: {}", e);
            }
        }
    }

    if direct.is_none() {
        initial_app_core.seed_default_quickbars_if_empty();
        if initial_app_core
            .ui_state
            .get_window_by_type(crate::data::window::WidgetType::Spells, None)
            .is_some()
        {
            let command = "_spell _spell_update_links\n".to_string();
            initial_app_core.message_processor.skip_next_spells_clear();
            initial_app_core
                .perf_stats
                .record_bytes_sent((command.len() + 1) as u64);
            let _ = command_tx.send(command);
        }
    }

    // Spawn network connection task (with auto-reconnect for Lich)
    // The initial session's network task is spawned after session_manager is created below.
    // We keep direct config for use after session_manager setup.
    let _direct_cfg = direct;

    // Session manager — owns all sessions
    let mut session_manager = SessionManager::new();

    // Load persisted session list
    let mut sessions_config = crate::sessions_config::SessionsConfig::load().unwrap_or_default();

    // Auto-connect sessions that have auto_connect = true
    for entry in sessions_config.sessions.iter().filter(|e| e.auto_connect) {
        match &entry.mode {
            crate::sessions_config::SessionModeConfig::Lich => {
                if let (Some(h), Some(p)) = (&entry.host, entry.port) {
                    let mode = ConnectionMode::LichProxy {
                        host: h.clone(),
                        port: p,
                        login_key: None,
                    };
                    let id = session_manager.add(entry.label.clone(), mode.clone());
                    if let Some(s) = session_manager.get_mut(id) {
                        if let Some(rx) = s.server_rx.take() {
                            session_rxs.insert(id, rx);
                        }
                        if let Some(tx) = spawn_session_network(s, raw_logger.clone()) {
                            s.command_tx = Some(tx);
                        }
                    }
                    if let Ok(ac) = create_app_core_for_session(&mode, &config) {
                        app_cores.insert(id, ac);
                    }
                    widget_managers.insert(
                        id,
                        crate::frontend::tui::widget_manager::WidgetManager::new(),
                    );
                }
            }
            crate::sessions_config::SessionModeConfig::Direct => {
                if let (Some(acct), Some(ch), Some(gc)) =
                    (&entry.account, &entry.character, &entry.game_code)
                {
                    let password = crate::credentials::get_password(acct).unwrap_or_default();
                    let mode = ConnectionMode::Direct {
                        account: acct.clone(),
                        password,
                        character: ch.clone(),
                        game_code: gc.clone(),
                    };
                    let id = session_manager.add(entry.label.clone(), mode.clone());
                    if let Some(s) = session_manager.get_mut(id) {
                        if let Some(rx) = s.server_rx.take() {
                            session_rxs.insert(id, rx);
                        }
                        if let Some(tx) = spawn_session_network(s, raw_logger.clone()) {
                            s.command_tx = Some(tx);
                        }
                    }
                    if let Ok(ac) = create_app_core_for_session(&mode, &config) {
                        app_cores.insert(id, ac);
                    }
                    widget_managers.insert(
                        id,
                        crate::frontend::tui::widget_manager::WidgetManager::new(),
                    );
                }
            }
        }
    }

    // Seed with the initial session derived from CLI args
    let initial_mode = if let Some(ref cfg) = _direct_cfg {
        ConnectionMode::Direct {
            account: cfg.account.clone(),
            password: cfg.password.clone(),
            character: cfg.character.clone(),
            game_code: cfg.game_code.clone(),
        }
    } else {
        ConnectionMode::LichProxy {
            host: host.clone(),
            port,
            login_key: login_key.clone(),
        }
    };
    let initial_label = character
        .clone()
        .unwrap_or_else(|| format!("{}:{}", host, port));
    let initial_id = session_manager.add(initial_label.clone(), initial_mode);
    if let Some(s) = session_manager.get_mut(initial_id) {
        if let Some(rx) = s.server_rx.take() {
            session_rxs.insert(initial_id, rx);
        }
        // Only connect if we have explicit CLI args or saved sessions (not first run)
        let should_connect =
            _direct_cfg.is_some() || character.is_some() || !sessions_config.sessions.is_empty();
        if should_connect {
            if let Some(tx) = spawn_session_network(s, raw_logger.clone()) {
                s.command_tx = Some(tx);
            }
        }
    }
    // Move the initial AppCore into the map
    app_cores.insert(initial_id, initial_app_core);
    // Initial session gets the current frontend widget_manager (already populated by startup sync)
    // We don't pre-insert here — it will be saved on first switch away from this session.

    // If no sessions are saved yet, show the picker
    let any_auto_connected = sessions_config.sessions.iter().any(|e| e.auto_connect);
    if sessions_config.sessions.is_empty() {
        // First run — go straight to the Direct login wizard
        frontend.login_wizard = Some(crate::frontend::tui::login_wizard::LoginWizard::new());
    } else if !any_auto_connected {
        // Has sessions but none auto-connect — show picker so user can choose
        frontend.session_picker = Some(crate::frontend::tui::session_picker::SessionPicker::new(
            &sessions_config,
        ));
    }

    // Sync tab bar labels into frontend
    let sync_tabs = |mgr: &SessionManager, fe: &mut TuiFrontend| {
        fe.session_labels = mgr
            .all()
            .iter()
            .map(|s| {
                let sym = match &s.status {
                    SessionStatus::Connected => "●".to_string(),
                    SessionStatus::Connecting => "…".to_string(),
                    SessionStatus::Reconnecting => "↻".to_string(),
                    SessionStatus::Error(_) => "!".to_string(),
                    SessionStatus::Disconnected => "○".to_string(),
                };
                (
                    s.label.clone(),
                    mgr.active().map_or(false, |a| a.id == s.id),
                    sym,
                    s.unread_count,
                    s.sound_enabled,
                    s.tts_enabled,
                )
            })
            .collect();
    };
    sync_tabs(&session_manager, &mut frontend);

    // Force a render after startup so picker/wizard/tabs are visible immediately
    if let Some(ac) = app_cores.get_mut(&initial_id) {
        ac.needs_render = true;
    }

    // Track time for periodic countdown updates
    let mut last_countdown_update = std::time::Instant::now();
    // When true, next game command is broadcast to all sessions (Ctrl+B)
    let mut broadcast_next = false;
    let mut running = true;

    // Main event loop
    while running {
        // Get active session's AppCore (or skip if none)
        // SAFETY: app_cores outlives the loop; we only access one entry per iteration
        let active_sid = session_manager.active().map(|s| s.id);
        let mut app_core: &mut crate::core::AppCore = if let Some(id) = active_sid {
            if let Some(ac) = app_cores.get_mut(&id) {
                // SAFETY: we hold no other references into app_cores during this iteration
                unsafe { &mut *(ac as *mut _) }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(16));
            continue;
        };

        if !app_core.running {
            running = false;
            break;
        }

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
                        app_core
                            .perf_stats
                            .record_bytes_sent((cmd.len() + 1) as u64);
                        if let Some(tx) =
                            session_manager.active().and_then(|s| s.command_tx.as_ref())
                        {
                            let _ = tx.send(cmd);
                        }
                    }

                    if handled {
                        continue;
                    }
                }
                crate::frontend::FrontendEvent::Key {
                    code: _code,
                    modifiers: _modifiers,
                } => {
                    // Key events are handled in handle_event()
                    // No early intercepts - let the 3-layer routing handle everything
                }
                _ => {}
            }

            if let Some(command) = handle_event(&mut app_core, &mut frontend, event)? {
                // Intercept wizard commands
                if command.starts_with("//wizard:") {
                    handle_wizard_command(
                        &command,
                        &mut frontend,
                        &mut session_manager,
                        &mut sessions_config,
                        &command_tx,
                        &mut session_rxs,
                        raw_logger.clone(),
                        &mut app_cores,
                        &config,
                        &mut widget_managers,
                    );
                    sync_tabs(&session_manager, &mut frontend);
                    app_core.needs_render = true;
                // Intercept picker commands
                } else if command.starts_with("//picker:") {
                    handle_picker_command(
                        &command,
                        &mut frontend,
                        &mut session_manager,
                        &mut sessions_config,
                        &command_tx,
                        &mut session_rxs,
                        raw_logger.clone(),
                        &mut app_cores,
                        &config,
                        &mut widget_managers,
                    );
                    sync_tabs(&session_manager, &mut frontend);
                // Intercept session commands before sending to game server
                } else if crate::frontend::tui::session_keys::is_session_cmd(&command) {
                    use crate::frontend::tui::session_keys::SessionCmd;
                    if let Some(cmd) = SessionCmd::parse(&command) {
                        let prev_sid = session_manager.active().map(|s| s.id);
                        match cmd {
                            SessionCmd::SwitchToIndex(i) => {
                                session_manager.set_active_by_index(i);
                                let new_sid = session_manager.active().map(|s| s.id);
                                if let Some(nid) = new_sid {
                                    do_session_switch(
                                        prev_sid,
                                        nid,
                                        &mut frontend,
                                        &mut widget_managers,
                                    );
                                }
                            }
                            SessionCmd::Next => {
                                session_manager.next();
                                let new_sid = session_manager.active().map(|s| s.id);
                                if let Some(nid) = new_sid {
                                    do_session_switch(
                                        prev_sid,
                                        nid,
                                        &mut frontend,
                                        &mut widget_managers,
                                    );
                                }
                            }
                            SessionCmd::Prev => {
                                session_manager.prev();
                                let new_sid = session_manager.active().map(|s| s.id);
                                if let Some(nid) = new_sid {
                                    do_session_switch(
                                        prev_sid,
                                        nid,
                                        &mut frontend,
                                        &mut widget_managers,
                                    );
                                }
                            }
                            SessionCmd::ToggleCompact => {
                                frontend.compact_tabs = !frontend.compact_tabs
                            }
                            SessionCmd::Broadcast => broadcast_next = true,
                            SessionCmd::ToggleSound => {
                                if let Some(s) = session_manager.active_mut() {
                                    s.sound_enabled = !s.sound_enabled;
                                }
                            }
                            SessionCmd::ToggleTts => {
                                if let Some(s) = session_manager.active_mut() {
                                    s.tts_enabled = !s.tts_enabled;
                                }
                            }
                            SessionCmd::New | SessionCmd::Close => {} // Phase 2
                        }
                        sync_tabs(&session_manager, &mut frontend);
                    }
                } else {
                    app_core
                        .perf_stats
                        .record_bytes_sent((command.len() + 1) as u64);
                    if broadcast_next {
                        session_manager.broadcast(&command);
                        broadcast_next = false;
                    } else if let Some(tx) =
                        session_manager.active().and_then(|s| s.command_tx.as_ref())
                    {
                        let _ = tx.send(command);
                    }
                }
            }

            let duration = event_start.elapsed();
            app_core.perf_stats.record_event_process_time(duration);

            // Process pending window additions after event handling (for .testline)
            let (term_width, term_height) = frontend.size();
            app_core.process_pending_window_additions(term_width, term_height);
        }

        // Poll for server messages (non-blocking) — use active session's receiver
        let active_id = session_manager.active().map(|s| s.id);
        if let Some(aid) = active_id {
            if let Some(rx) = session_rxs.get_mut(&aid) {
                while let Ok(msg) = rx.try_recv() {
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
                                    if let Err(e) =
                                        player.play_from_sounds_dir(&sound.file, sound.volume)
                                    {
                                        tracing::warn!(
                                            "Failed to play sound '{}': {}",
                                            sound.file,
                                            e
                                        );
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
                } // while try_recv
            } // if let Some(rx)
        } // if let Some(aid)

        // Force render every second for countdown widgets
        if last_countdown_update.elapsed().as_secs() >= 1 {
            app_core.needs_render = true;
            last_countdown_update = std::time::Instant::now();
            // Sync unread badges from atomic counters (background sessions)
            session_manager.sync_unread_all();
            sync_tabs(&session_manager, &mut frontend);
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
            frontend.render(app_core)?;
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
    session_rxs: &mut HashMap<
        SessionId,
        tokio::sync::mpsc::UnboundedReceiver<crate::network::ServerMessage>,
    >,
    raw_logger: Option<crate::network::RawLogger>,
    app_cores: &mut HashMap<SessionId, crate::core::AppCore>,
    base_config: &crate::config::Config,
    widget_managers: &mut HashMap<SessionId, crate::frontend::tui::widget_manager::WidgetManager>,
) {
    use crate::frontend::tui::session_picker::PickerAction;

    // Take the action out of the picker so we can act on it
    let action = frontend
        .session_picker
        .as_mut()
        .and_then(|p| p.action.take());

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
                    host: entry
                        .host
                        .clone()
                        .unwrap_or_else(|| "localhost".to_string()),
                    port: entry.port.unwrap_or(8000),
                    login_key: None,
                };
                let id = session_manager.add(entry.label.clone(), mode.clone());
                if let Some(s) = session_manager.get_mut(id) {
                    if let Some(rx) = s.server_rx.take() {
                        session_rxs.insert(id, rx);
                    }
                    if let Some(tx) = spawn_session_network(s, raw_logger.clone()) {
                        s.command_tx = Some(tx);
                    } else {
                        s.command_tx = Some(command_tx.clone());
                    }
                }
                if let Ok(ac) = create_app_core_for_session(&mode, base_config) {
                    app_cores.insert(id, ac);
                }
                widget_managers.insert(
                    id,
                    crate::frontend::tui::widget_manager::WidgetManager::new(),
                );
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
                let prev_sid = session_manager.active().map(|s| s.id);
                session_manager.set_active_by_index(idx);
                let new_sid = session_manager.active().map(|s| s.id);
                if let Some(nid) = new_sid {
                    do_session_switch(prev_sid, nid, frontend, widget_managers);
                    // If this is a Direct session that was never connected, spawn it now
                    if let Some(s) = session_manager.get_mut(nid) {
                        if s.command_tx.is_none() {
                            if let ConnectionMode::Direct {
                                ref account,
                                ref mut password,
                                ..
                            } = s.mode
                            {
                                // Retrieve password from keychain if not already set
                                if password.is_empty() {
                                    if let Some(pw) = crate::credentials::get_password(account) {
                                        *password = pw;
                                    }
                                }
                            }
                            if let Some(tx) = spawn_session_network(s, raw_logger.clone()) {
                                s.command_tx = Some(tx);
                            }
                        }
                    }
                }
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
    session_rxs: &mut HashMap<
        SessionId,
        tokio::sync::mpsc::UnboundedReceiver<crate::network::ServerMessage>,
    >,
    raw_logger: Option<crate::network::RawLogger>,
    app_cores: &mut HashMap<SessionId, crate::core::AppCore>,
    base_config: &crate::config::Config,
    widget_managers: &mut HashMap<SessionId, crate::frontend::tui::widget_manager::WidgetManager>,
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
                    crate::network::fetch_characters_for_account(
                        &account, &password, &game_code, &data_dir,
                    )
                })
                .join()
                {
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
                let id = session_manager.add(character.to_string(), mode.clone());
                if let Some(s) = session_manager.get_mut(id) {
                    if let Some(rx) = s.server_rx.take() {
                        session_rxs.insert(id, rx);
                    }
                    if let Some(tx) = spawn_session_network(s, raw_logger.clone()) {
                        s.command_tx = Some(tx);
                    } else {
                        s.command_tx = Some(command_tx.clone());
                    }
                }
                if let Ok(ac) = create_app_core_for_session(&mode, base_config) {
                    app_cores.insert(id, ac);
                }
                widget_managers.insert(
                    id,
                    crate::frontend::tui::widget_manager::WidgetManager::new(),
                );
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
                // Store password in OS keychain (keyed by account name)
                crate::credentials::store_password(account, password);
                // Remove any placeholder Lich session that was never connected
                // (the initial session created at startup when no CLI args given)
                let placeholder_ids: Vec<_> = session_manager
                    .all()
                    .iter()
                    .filter(|s| {
                        s.id != id
                            && s.command_tx.is_none()
                            && matches!(s.mode, crate::session::ConnectionMode::LichProxy { .. })
                    })
                    .map(|s| s.id)
                    .collect();
                for pid in placeholder_ids {
                    session_manager.remove(pid);
                    app_cores.remove(&pid);
                    widget_managers.remove(&pid);
                    session_rxs.remove(&pid);
                }
                // Switch to the new session
                let prev_sid = session_manager.active().map(|s| s.id);
                session_manager.set_active(id);
                do_session_switch(prev_sid, id, frontend, widget_managers);
                // Close wizard and picker
                frontend.login_wizard = None;
                frontend.session_picker = None;
            }
        }
        _ => {}
    }
}
