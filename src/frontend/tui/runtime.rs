use anyhow::Result;
use std::time::Instant;

use super::TuiFrontend;
use crate::frontend::Frontend;

/// Spawn a Lich connection with auto-reconnect.
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
        let mut first_rx = Some(command_rx);
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let rx = first_rx
                .take()
                .unwrap_or_else(|| tokio::sync::mpsc::unbounded_channel::<String>().1);
            match crate::network::LichConnection::start(
                &host,
                port,
                login_key.clone(),
                server_tx.clone(),
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

/// Run the TUI frontend.
pub fn run(
    config: crate::config::Config,
    character: Option<String>,
    direct: Option<crate::network::DirectConnectConfig>,
    setup_palette: bool,
    login_key: Option<String>,
) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run(
        config,
        character,
        direct,
        setup_palette,
        login_key,
    ))
}

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

    let raw_logger = match crate::network::RawLogger::new(&config) {
        Ok(logger) => logger,
        Err(e) => {
            tracing::error!("Failed to initialize raw logger: {}", e);
            None
        }
    };

    let mut app_core = AppCore::new(config.clone())?;
    super::colors::set_global_color_mode(app_core.config.ui.color_mode);
    if app_core.config.ui.color_mode == crate::config::ColorMode::Slot {
        super::colors::init_palette_lookup(&app_core.config.colors.color_palette);
    }

    let mut frontend = TuiFrontend::new(app_core.config.ui.mouse_capture)?;

    // Restore window position
    if let Some(positioner) = crate::window_position::create_positioner() {
        if let Ok(Some(saved)) = crate::window_position::load(character.as_deref()) {
            // Skip restore if no valid size was ever saved
            if saved.window.width > 0 && saved.window.height > 0 {
                use crate::window_position::WindowPositionerExt;
                let rect = if positioner.is_visible(&saved.window) {
                    saved.window
                } else {
                    positioner
                        .clamp_to_screen(&saved.window)
                        .unwrap_or(saved.window)
                };
                let _ = positioner.set_position(&rect);
            }
        }
    }

    let initial_theme_id = app_core.config.active_theme.clone();
    let initial_theme = app_core.config.get_theme();
    frontend.update_theme_cache(initial_theme_id, initial_theme);
    frontend.ensure_command_input_exists("command_input");

    if setup_palette {
        if let Err(e) = frontend.execute_setpalette(&app_core) {
            tracing::warn!("Failed to setup palette: {}", e);
        }
    }

    let (width, height) = frontend.size();
    app_core.init_windows(width, height);
    // Skip initial render — terminal buffer may not match actual size yet.
    // The event loop will render on the first tick after the terminal settles.

    if let Err(e) = frontend.command_input_load_history("command_input", character.as_deref()) {
        tracing::warn!("Failed to load command history: {}", e);
    }

    if app_core.config.sound.startup_music {
        if let Some(ref player) = app_core.sound_player {
            let delay_ms = app_core.config.sound.startup_music_delay_ms;
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            let _ = player.play_from_sounds_dir("wizard_music", None);
        }
    }

    // Determine connection mode from CLI args or connection.toml
    let (server_tx, server_rx) = mpsc::unbounded_channel::<ServerMessage>();
    let (mut command_tx, command_rx) = mpsc::unbounded_channel::<String>();

    // Resolve connection: CLI args take priority, then connection.toml, then show setup
    let conn = if let Some(ref cfg) = direct {
        // Direct connection from CLI --direct flags
        Some(ConnectionMode::Direct {
            account: cfg.account.clone(),
            password: cfg.password.clone(),
            character: cfg.character.clone(),
            game_code: cfg.game_code.clone(),
        })
    } else if character.is_some() || login_key.is_some() {
        // Lich proxy from CLI args
        Some(ConnectionMode::Lich {
            host: config.connection.host.clone(),
            port: config.connection.port,
            login_key: login_key.clone(),
        })
    } else {
        None
    };

    // Spawn network task based on resolved connection
    let mut pending_command_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>> = None;
    let mut pending_raw_logger: Option<crate::network::RawLogger> = None;
    let connected = match conn {
        Some(ConnectionMode::Lich {
            ref host,
            port,
            ref login_key,
        }) => {
            spawn_lich_reconnect(
                host.clone(),
                port,
                login_key.clone(),
                server_tx.clone(),
                command_rx,
                raw_logger,
                5,
            );
            true
        }
        Some(ConnectionMode::Direct {
            ref account,
            ref password,
            ref character,
            ref game_code,
        }) => {
            let cfg = crate::network::DirectConnectConfig {
                account: account.clone(),
                password: password.clone(),
                character: character.clone(),
                game_code: game_code.clone(),
                data_dir: crate::config::Config::base_dir().unwrap_or_default(),
            };
            let st = server_tx.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::network::DirectConnection::start(cfg, st, command_rx, raw_logger).await
                {
                    tracing::error!("Direct connection error: {:#}", e);
                }
            });
            true
        }
        None => {
            // No connection configured — show profile picker
            tracing::info!("No profiles configured. Showing profile picker.");
            frontend.show_setup_screen = true;
            let profiles: Vec<super::login_wizard::Profile> =
                crate::connection::ProfileStore::load()
                    .unwrap_or_default()
                    .profiles
                    .into_iter()
                    .map(|p| super::login_wizard::Profile {
                        name: p.name,
                        account: p.account,
                        character: p.character,
                        game_code: p.game_code,
                        use_lich: p.use_lich,
                        lich_host: p.lich_host,
                        lich_port: p.lich_port,
                        lich_command: p.lich_command,
                    })
                    .collect();
            frontend.login_wizard = Some(super::login_wizard::ProfilePicker::new(profiles));
            pending_command_rx = Some(command_rx);
            pending_raw_logger = raw_logger;
            false
        }
    };

    if !connected {
        app_core.seed_default_quickbars_if_empty();
    }

    // If Direct connection needs a password prompt, show it
    if let Some(ref cfg) = direct {
        if cfg.password.is_empty() {
            frontend.show_password_prompt = true;
        }
    }

    let mut server_rx = server_rx;
    let mut last_countdown_update = std::time::Instant::now();
    let mut last_position_save = std::time::Instant::now();
    let mut running = true;

    while running {
        if !app_core.running {
            running = false;
            break;
        }

        let events = frontend.poll_events()?;
        app_core
            .perf_stats
            .record_event_queue_depth(events.len() as u64);
        app_core.poll_tts_events();

        for event in events {
            let event_start = Instant::now();
            match &event {
                crate::frontend::FrontendEvent::Mouse(mouse_event) => {
                    let (handled, command) = frontend.handle_mouse_event(
                        mouse_event,
                        &mut app_core,
                        crate::frontend::tui::menu_actions::handle_menu_action,
                    )?;
                    if let Some(cmd) = command {
                        app_core
                            .perf_stats
                            .record_bytes_sent((cmd.len() + 1) as u64);
                        let _ = command_tx.send(cmd);
                    }
                    if handled {
                        continue;
                    }
                }
                _ => {}
            }

            if let Some(command) = handle_event(&mut app_core, &mut frontend, event)? {
                // Handle setup/password prompt commands
                if command.starts_with("//setup:") {
                    if command.starts_with("//setup:connect:direct:") {
                        // Parse: //setup:connect:direct:<account>:<game_code>:<character>
                        let parts: Vec<&str> = command["//setup:connect:direct:".len()..]
                            .splitn(3, ':')
                            .collect();
                        if parts.len() == 3 {
                            let account = parts[0].to_string();
                            let game_code = parts[1].to_string();
                            let character = parts[2].to_string();
                            let password =
                                crate::credentials::get_password(&account).unwrap_or_default();
                            let cfg = crate::network::DirectConnectConfig {
                                account,
                                password,
                                character,
                                game_code,
                                data_dir: crate::config::Config::base_dir().unwrap_or_default(),
                            };
                            let rx = pending_command_rx.take().unwrap_or_else(|| {
                                let (new_tx, new_rx) =
                                    tokio::sync::mpsc::unbounded_channel::<String>();
                                let _ = std::mem::replace(&mut command_tx, new_tx);
                                new_rx
                            });
                            let rl = pending_raw_logger.take();
                            let st = server_tx.clone();
                            tokio::spawn(async move {
                                if let Err(e) =
                                    crate::network::DirectConnection::start(cfg, st, rx, rl).await
                                {
                                    tracing::error!("Direct connection error: {:#}", e);
                                }
                            });
                        }
                    } else if command.starts_with("//setup:connect:lich:") {
                        // Parse: //setup:connect:lich:<account>\x1F<game_code>\x1F<character>\x1F<lich_host>\x1F<lich_port>\x1F<lich_command>
                        let payload = &command["//setup:connect:lich:".len()..];
                        let parts: Vec<&str> = payload.splitn(6, '\x1F').collect();
                        if parts.len() >= 5 {
                            let account = parts[0].to_string();
                            let game_code = parts[1].to_string();
                            let character = parts[2].to_string();
                            let lich_host = parts[3].to_string();
                            // 0 means "use game port" (Warlock default behavior)
                            let lich_port_override = parts[4].parse::<u16>().unwrap_or(0);
                            let launch_cmd = parts.get(5).copied().unwrap_or("").to_string();
                            let rx = pending_command_rx.take().unwrap_or_else(|| {
                                let (new_tx, new_rx) =
                                    tokio::sync::mpsc::unbounded_channel::<String>();
                                let _ = std::mem::replace(&mut command_tx, new_tx);
                                new_rx
                            });
                            let rl = pending_raw_logger.take();
                            let st = server_tx.clone();
                            tokio::spawn(async move {
                                // Do eAccess auth to get real game host:port:key
                                let password =
                                    crate::credentials::get_password(&account).unwrap_or_default();
                                let data_dir =
                                    crate::config::Config::base_dir().unwrap_or_default();
                                let (game_host, game_port, key) =
                                    match tokio::task::spawn_blocking(move || {
                                        crate::network::authenticate_and_launch(
                                            &account, &password, &character, &game_code, &data_dir,
                                        )
                                    })
                                    .await
                                    {
                                        Ok(Ok(ticket)) => ticket,
                                        Ok(Err(e)) => {
                                            tracing::error!("Lich auth failed: {:#}", e);
                                            return;
                                        }
                                        Err(e) => {
                                            tracing::error!("Lich auth task failed: {:#}", e);
                                            return;
                                        }
                                    };
                                // Substitute real host:port into launch command
                                if !launch_cmd.is_empty() {
                                    let cmd = launch_cmd
                                        .replace("{host}", &game_host)
                                        .replace("{port}", &game_port.to_string());
                                    tracing::info!("Launching Lich: {}", cmd);
                                    #[cfg(target_os = "windows")]
                                    let child = {
                                        use std::os::windows::process::CommandExt;
                                        const DETACHED_PROCESS: u32 = 0x00000008;
                                        std::process::Command::new("cmd")
                                            .args(["/C", &cmd])
                                            .creation_flags(DETACHED_PROCESS)
                                            .spawn()
                                    };
                                    #[cfg(not(target_os = "windows"))]
                                    let child = std::process::Command::new("sh")
                                        .args(["-c", &cmd])
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .spawn();
                                    match child {
                                        Ok(_) => tracing::info!("Lich process launched"),
                                        Err(e) => tracing::warn!("Failed to launch Lich: {}", e),
                                    }
                                }
                                // Use game port if no override specified (Warlock default)
                                let connect_port = if lich_port_override == 0 {
                                    game_port
                                } else {
                                    lich_port_override
                                };
                                tracing::info!(
                                    "Connecting to Lich at {}:{}",
                                    lich_host,
                                    connect_port
                                );
                                // Connect to Lich's local listener, sending the auth key
                                spawn_lich_reconnect(
                                    lich_host,
                                    connect_port,
                                    Some(key),
                                    st,
                                    rx,
                                    rl,
                                    5,
                                );
                            });
                        }
                    } else {
                        handle_setup_command(&command, &mut frontend, &mut app_core);
                    }
                } else {
                    app_core
                        .perf_stats
                        .record_bytes_sent((command.len() + 1) as u64);
                    let _ = command_tx.send(command);
                }
            }

            let duration = event_start.elapsed();
            app_core.perf_stats.record_event_process_time(duration);

            let (term_width, term_height) = frontend.size();
            app_core.process_pending_window_additions(term_width, term_height);
        }

        // Poll server messages
        while let Ok(msg) = server_rx.try_recv() {
            match msg {
                ServerMessage::Text(line) => {
                    app_core
                        .perf_stats
                        .record_bytes_received((line.len() + 1) as u64);
                    let parse_start = Instant::now();
                    if let Err(e) = app_core.process_server_data(&line) {
                        tracing::error!("Error processing server data: {}", e);
                    }
                    app_core.perf_stats.record_parse(parse_start.elapsed());
                    app_core.adjust_content_driven_windows();
                    for sound in app_core.game_state.drain_sound_queue() {
                        if let Some(ref player) = app_core.sound_player {
                            if let Err(e) = player.play_from_sounds_dir(&sound.file, sound.volume) {
                                tracing::warn!("Failed to play sound '{}': {}", sound.file, e);
                            }
                        }
                    }
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
                        app_core.message_processor.newly_registered_container = None;
                    }
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

        if last_countdown_update.elapsed().as_secs() >= 1 {
            app_core.needs_render = true;
            last_countdown_update = std::time::Instant::now();
        }

        if last_position_save.elapsed().as_secs() >= 30 {
            if let Some(positioner) = crate::window_position::create_positioner() {
                if let (Ok(rect), Ok(screens)) =
                    (positioner.get_position(), positioner.get_screen_bounds())
                {
                    let cfg = crate::window_position::WindowPositionConfig {
                        window: rect,
                        monitors: screens,
                    };
                    let _ = crate::window_position::save(character.as_deref(), &cfg);
                }
            }
            last_position_save = std::time::Instant::now();
        }

        app_core.perf_stats.sample_sysinfo();

        if app_core.ui_state.needs_widget_reset {
            frontend.widget_manager.clear();
            app_core.ui_state.needs_widget_reset = false;
        }

        if !app_core.ui_state.widgets_to_reset.is_empty() {
            for name in app_core.ui_state.widgets_to_reset.drain(..) {
                frontend.widget_manager.remove_widget_from_all_caches(&name);
            }
        }

        if app_core.needs_render {
            frontend.render(&mut app_core)?;
            app_core.needs_render = false;
        }
    }

    if let Err(e) = frontend.command_input_save_history("command_input", character.as_deref()) {
        tracing::warn!("Failed to save command history: {}", e);
    }

    if let Some(positioner) = crate::window_position::create_positioner() {
        if let Ok(rect) = positioner.get_position() {
            if let Ok(screens) = positioner.get_screen_bounds() {
                let cfg = crate::window_position::WindowPositionConfig {
                    window: rect,
                    monitors: screens,
                };
                if let Err(e) = crate::window_position::save(character.as_deref(), &cfg) {
                    tracing::warn!("Failed to save window position: {}", e);
                }
            }
        }
    }

    frontend.cleanup()?;
    Ok(())
}

/// Internal connection mode (runtime only, not persisted)
enum ConnectionMode {
    Lich {
        host: String,
        port: u16,
        login_key: Option<String>,
    },
    Direct {
        account: String,
        password: String,
        character: String,
        game_code: String,
    },
}

fn handle_event(
    app_core: &mut crate::core::AppCore,
    frontend: &mut TuiFrontend,
    event: crate::frontend::FrontendEvent,
) -> Result<Option<String>> {
    use crate::frontend::FrontendEvent;
    match event {
        FrontendEvent::Key { code, modifiers } => frontend.handle_key_event(
            code,
            modifiers,
            app_core,
            crate::frontend::tui::menu_actions::handle_menu_action,
        ),
        FrontendEvent::Paste { text } => {
            if let Some(picker) = frontend.login_wizard.as_mut() {
                picker.paste_text(&text);
                app_core.needs_render = true;
            }
            Ok(None)
        }
        FrontendEvent::Resize { width, height } => {
            tracing::info!(
                "Terminal resized to {}x{} (use .resize to apply)",
                width,
                height
            );
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn handle_setup_command(
    command: &str,
    frontend: &mut TuiFrontend,
    _app_core: &mut crate::core::AppCore,
) {
    if command == "//setup:dismiss" {
        frontend.show_setup_screen = false;
        frontend.show_password_prompt = false;
    }
}
