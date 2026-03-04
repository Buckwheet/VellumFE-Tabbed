//! Input handling for Search and Normal modes
//!
//! These methods handle keyboard input routing based on the current input mode.
//! Extracted from mod.rs to reduce file size and improve organization.

use crate::frontend::tui::menu_actions;
use anyhow::Result;

/// Input handling methods (impl extension for TuiFrontend)
impl super::TuiFrontend {
    pub(super) fn handle_search_mode_keys(
        &mut self,
        code: crate::frontend::KeyCode,
        modifiers: crate::frontend::KeyModifiers,
        app_core: &mut crate::core::AppCore,
    ) -> Result<Option<String>> {
        use crate::frontend::KeyCode;

        // Handle Ctrl+PageUp/PageDown for cycling through search results
        if modifiers.ctrl {
            match code {
                KeyCode::PageUp => {
                    let focused_name = app_core.get_focused_window_name();
                    if self.prev_search_match(&focused_name) {
                        tracing::debug!("Jumped to previous search match in '{}'", focused_name);
                    } else {
                        tracing::debug!("No more search matches in '{}'", focused_name);
                    }
                    app_core.needs_render = true;
                    return Ok(None);
                }
                KeyCode::PageDown => {
                    let focused_name = app_core.get_focused_window_name();
                    if self.next_search_match(&focused_name) {
                        tracing::debug!("Jumped to next search match in '{}'", focused_name);
                    } else {
                        tracing::debug!("No more search matches in '{}'", focused_name);
                    }
                    app_core.needs_render = true;
                    return Ok(None);
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Enter => {
                let pattern = app_core.ui_state.search_input.clone();
                if !pattern.is_empty() {
                    let window_name = app_core.get_focused_window_name();
                    match self.execute_search(&window_name, &pattern) {
                        Ok(count) => {
                            if count > 0 {
                                tracing::info!("Found {} matches for '{}'", count, pattern);
                            } else {
                                tracing::info!("No matches found for '{}'", pattern);
                            }
                            app_core.needs_render = true;
                        }
                        Err(e) => {
                            tracing::warn!("Invalid search regex '{}': {}", pattern, e);
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                let pos = app_core.ui_state.search_cursor;
                app_core.ui_state.search_input.insert(pos, c);
                app_core.ui_state.search_cursor += 1;
                app_core.needs_render = true;
            }
            KeyCode::Backspace => {
                if app_core.ui_state.search_cursor > 0 {
                    app_core.ui_state.search_cursor -= 1;
                    app_core
                        .ui_state
                        .search_input
                        .remove(app_core.ui_state.search_cursor);
                    app_core.needs_render = true;
                }
            }
            KeyCode::Left => {
                if app_core.ui_state.search_cursor > 0 {
                    app_core.ui_state.search_cursor -= 1;
                    app_core.needs_render = true;
                }
            }
            KeyCode::Right => {
                if app_core.ui_state.search_cursor < app_core.ui_state.search_input.len() {
                    app_core.ui_state.search_cursor += 1;
                    app_core.needs_render = true;
                }
            }
            KeyCode::Home => {
                app_core.ui_state.search_cursor = 0;
                app_core.needs_render = true;
            }
            KeyCode::End => {
                app_core.ui_state.search_cursor = app_core.ui_state.search_input.len();
                app_core.needs_render = true;
            }
            KeyCode::Esc => {
                // Exit search mode
                app_core.ui_state.input_mode = crate::data::InputMode::Normal;
                app_core.ui_state.search_input.clear();
                app_core.ui_state.search_cursor = 0;
                app_core.needs_render = true;
                tracing::debug!("Exited search mode");
            }
            _ => {}
        }
        Ok(None)
    }

    /// Handle Normal mode keyboard events (extracted from main.rs Phase 4.2)
    pub(super) fn handle_normal_mode_keys(
        &mut self,
        code: crate::frontend::KeyCode,
        modifiers: crate::frontend::KeyModifiers,
        app_core: &mut crate::core::AppCore,
    ) -> Result<Option<String>> {
        use crate::data::window::WidgetType;
        use crate::frontend::KeyCode;

        // Ctrl+C always quits, regardless of state
        if modifiers.ctrl && matches!(code, KeyCode::Char('c') | KeyCode::Char('C')) {
            app_core.quit();
            return Ok(None);
        }

        // If login wizard is active, route all keys to it
        if self.login_wizard.is_some() {
            return self.handle_wizard_keys(code, modifiers, app_core);
        }

        // If setup screen is active, dismiss on any key
        if self.show_setup_screen || self.show_password_prompt {
            return Ok(Some("//setup:dismiss".to_string()));
        }

        let focused_name = app_core.get_focused_window_name();
        if let Some(window) = app_core.ui_state.get_window(&focused_name) {
            if window.widget_type == WidgetType::Quickbar {
                match code {
                    KeyCode::Left => {
                        if let Some(widget) =
                            self.widget_manager.quickbar_widgets.get_mut(&focused_name)
                        {
                            widget.move_selection(-1);
                            app_core.needs_render = true;
                            return Ok(None);
                        }
                    }
                    KeyCode::Right => {
                        if let Some(widget) =
                            self.widget_manager.quickbar_widgets.get_mut(&focused_name)
                        {
                            widget.move_selection(1);
                            app_core.needs_render = true;
                            return Ok(None);
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(widget) =
                            self.widget_manager.quickbar_widgets.get_mut(&focused_name)
                        {
                            if let Some(action) = widget.activate_selected() {
                                return Ok(self.handle_quickbar_action(
                                    action,
                                    &focused_name,
                                    app_core,
                                ));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app_core.needs_render = true;
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }

        if matches!(code, KeyCode::BackTab) {
            app_core.cycle_focused_window_reverse();
            app_core.needs_render = true;
            return Ok(None);
        }

        // Handle Enter key - always submit command
        if matches!(code, KeyCode::Enter) {
            if let Some(command) = self.command_input_submit("command_input") {
                return self.handle_command_submission(command, app_core);
            }
        } else {
            // Check for keybinds first - normalize to lowercase for consistent matching
            let normalized_code = match code {
                KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                other => other,
            };
            let key_event = crate::frontend::common::KeyEvent {
                code: normalized_code,
                modifiers,
            };
            if let Some(action) = app_core.keybind_map.get(&key_event).cloned() {
                let is_command_input_action = matches!(&action,
                    crate::config::KeyBindAction::Action(s) if matches!(s.as_str(),
                        "cursor_left" | "cursor_right" | "cursor_word_left" | "cursor_word_right" |
                        "cursor_home" | "cursor_end" | "cursor_backspace" | "cursor_delete" |
                        "previous_command" | "next_command" | "send_last_command" | "send_second_last_command"
                    )
                );

                let is_tab_action = matches!(&action,
                    crate::config::KeyBindAction::Action(s) if matches!(s.as_str(),
                        "next_tab" | "prev_tab" | "next_unread_tab"
                    )
                );

                // Check for switch_current_window (Tab key) - smart behavior:
                // - If command input has text starting with '.', do tab completion
                // - Otherwise, cycle focused window for scrolling
                let is_switch_window_action = matches!(&action,
                    crate::config::KeyBindAction::Action(s) if s.as_str() == "switch_current_window"
                );

                // Check for search actions - must be handled by frontend
                let is_search_action = matches!(&action,
                    crate::config::KeyBindAction::Action(s) if matches!(s.as_str(),
                        "start_search" | "next_search_match" | "prev_search_match" | "clear_search"
                    )
                );

                // Check for scroll actions - must be handled by frontend (TuiFrontend.scroll_window)
                let is_scroll_action = matches!(&action,
                    crate::config::KeyBindAction::Action(s) if matches!(s.as_str(),
                        "scroll_current_window_up_one" | "scroll_current_window_down_one" |
                        "scroll_current_window_up_page" | "scroll_current_window_down_page" |
                        "scroll_current_window_home" | "scroll_current_window_end"
                    )
                );

                if is_search_action {
                    // Handle search actions
                    if let crate::config::KeyBindAction::Action(action_str) = &action {
                        match action_str.as_str() {
                            "start_search" => {
                                // Enter search mode
                                app_core.ui_state.input_mode = crate::data::InputMode::Search;
                                app_core.ui_state.search_input.clear();
                                app_core.ui_state.search_cursor = 0;
                                tracing::debug!("Entered search mode");
                            }
                            "next_search_match" => {
                                let focused_name = app_core.get_focused_window_name();
                                if self.next_search_match(&focused_name) {
                                    tracing::debug!(
                                        "Jumped to next search match in '{}'",
                                        focused_name
                                    );
                                } else {
                                    tracing::debug!("No more search matches in '{}'", focused_name);
                                }
                            }
                            "prev_search_match" => {
                                let focused_name = app_core.get_focused_window_name();
                                if self.prev_search_match(&focused_name) {
                                    tracing::debug!(
                                        "Jumped to previous search match in '{}'",
                                        focused_name
                                    );
                                } else {
                                    tracing::debug!("No more search matches in '{}'", focused_name);
                                }
                            }
                            "clear_search" => {
                                self.clear_all_searches();
                                tracing::debug!("Cleared all searches");
                            }
                            _ => {}
                        }
                    }
                    app_core.needs_render = true;
                } else if is_scroll_action {
                    // Get the focused window name and scroll it via frontend
                    let focused_name = app_core.get_focused_window_name();
                    if let crate::config::KeyBindAction::Action(action_str) = &action {
                        match action_str.as_str() {
                            "scroll_current_window_up_one" => {
                                self.scroll_window(&focused_name, 1);
                                tracing::debug!(
                                    "Scrolled '{}' up 1 line via frontend",
                                    focused_name
                                );
                            }
                            "scroll_current_window_down_one" => {
                                self.scroll_window(&focused_name, -1);
                                tracing::debug!(
                                    "Scrolled '{}' down 1 line via frontend",
                                    focused_name
                                );
                            }
                            "scroll_current_window_up_page" => {
                                self.scroll_window(&focused_name, 20);
                                tracing::info!(
                                    "Scrolled '{}' up 20 lines via frontend",
                                    focused_name
                                );
                            }
                            "scroll_current_window_down_page" => {
                                self.scroll_window(&focused_name, -20);
                                tracing::info!(
                                    "Scrolled '{}' down 20 lines via frontend",
                                    focused_name
                                );
                            }
                            "scroll_current_window_home" => {
                                // Scroll to top - use a large number
                                self.scroll_window(&focused_name, 100000);
                                tracing::debug!("Scrolled '{}' to top via frontend", focused_name);
                            }
                            "scroll_current_window_end" => {
                                // Scroll to bottom - use a large negative number
                                self.scroll_window(&focused_name, -100000);
                                tracing::debug!(
                                    "Scrolled '{}' to bottom via frontend",
                                    focused_name
                                );
                            }
                            _ => {}
                        }
                    }
                    app_core.needs_render = true;
                } else if is_switch_window_action {
                    // Check if command input has text that should trigger tab completion
                    let should_complete = self
                        .widget_manager
                        .command_inputs
                        .get("command_input")
                        .and_then(|cmd| cmd.get_input())
                        .map(|text| text.starts_with('.'))
                        .unwrap_or(false);

                    if should_complete {
                        // Do tab completion for dot commands
                        let available_commands = app_core.get_available_commands();
                        let available_window_names = app_core.get_window_names();
                        use crate::frontend::tui::crossterm_bridge;
                        let ct_code = crossterm_bridge::to_crossterm_keycode(code);
                        let ct_mods = crossterm_bridge::to_crossterm_modifiers(modifiers);
                        self.command_input_key(
                            "command_input",
                            ct_code,
                            ct_mods,
                            &available_commands,
                            &available_window_names,
                        );
                    } else {
                        app_core.cycle_focused_window();
                    }
                    app_core.needs_render = true;
                } else if is_tab_action {
                    if let crate::config::KeyBindAction::Action(action_str) = &action {
                        match action_str.as_str() {
                            "next_tab" => {
                                self.next_tab_all();
                                self.sync_tabbed_active_state(app_core);
                                tracing::info!("Switched to next tab in all tabbed windows");
                            }
                            "prev_tab" => {
                                self.prev_tab_all();
                                self.sync_tabbed_active_state(app_core);
                                tracing::info!("Switched to previous tab in all tabbed windows");
                            }
                            "next_unread_tab" => {
                                if !self.go_to_next_unread_tab() {
                                    app_core.add_system_message("No tabs with new messages");
                                }
                                self.sync_tabbed_active_state(app_core);
                                tracing::info!("Next unread tab navigation triggered");
                            }
                            _ => {}
                        }
                    }
                    app_core.needs_render = true;
                } else if is_command_input_action {
                    let available_commands = app_core.get_available_commands();
                    let available_window_names = app_core.get_window_names();
                    use crate::frontend::tui::crossterm_bridge;
                    let ct_code = crossterm_bridge::to_crossterm_keycode(code);
                    let ct_mods = crossterm_bridge::to_crossterm_modifiers(modifiers);
                    self.command_input_key(
                        "command_input",
                        ct_code,
                        ct_mods,
                        &available_commands,
                        &available_window_names,
                    );
                    app_core.needs_render = true;
                } else {
                    match app_core.execute_keybind_action(&action) {
                        Ok(commands) => {
                            if let Some(cmd) = commands.into_iter().next() {
                                app_core.needs_render = true;
                                return Ok(Some(cmd));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Keybind action failed: {}", e);
                        }
                    }
                    app_core.needs_render = true;
                }
            } else {
                // No keybind - route to CommandInput for typing
                let available_commands = app_core.get_available_commands();
                let available_window_names = app_core.get_window_names();
                use crate::frontend::tui::crossterm_bridge;
                let ct_code = crossterm_bridge::to_crossterm_keycode(code);
                let ct_mods = crossterm_bridge::to_crossterm_modifiers(modifiers);
                self.command_input_key(
                    "command_input",
                    ct_code,
                    ct_mods,
                    &available_commands,
                    &available_window_names,
                );
                app_core.needs_render = true;
            }
        }
        Ok(None)
    }

    /// Handle command submission from CommandInput (extracted from main.rs Phase 4.2)
    pub(super) fn handle_command_submission(
        &mut self,
        command: String,
        app_core: &mut crate::core::AppCore,
    ) -> Result<Option<String>> {
        tracing::debug!("handle_command_submission: start '{}'", command);
        if command.starts_with(".savelayout ") || command == ".savelayout" {
            let name = command
                .strip_prefix(".savelayout ")
                .unwrap_or("default")
                .trim();
            let (width, height) = self.size();
            app_core.save_layout(name, width, height);
            app_core.needs_render = true;
        } else if command.starts_with(".loadlayout ") || command == ".loadlayout" {
            let name = command
                .strip_prefix(".loadlayout ")
                .unwrap_or("default")
                .trim();
            let (width, height) = self.size();
            if let Some((theme_id, theme)) = app_core.load_layout(name, width, height) {
                self.update_theme_cache(theme_id, theme);
            }
            app_core.needs_render = true;
        } else if command == ".resize" {
            let (width, height) = self.size();
            app_core.resize_windows(width, height);
            app_core.needs_render = true;
        } else {
            let to_send = app_core.send_command(command)?;
            tracing::debug!(
                "handle_command_submission: send_command returned '{}'",
                to_send
            );
            if to_send.starts_with("action:") {
                // Handle internal UI actions locally instead of sending to the game
                menu_actions::handle_menu_action(app_core, self, &to_send)?;
                app_core.needs_render = true;
                tracing::debug!("handle_command_submission: handled action '{}'", to_send);
                return Ok(None);
            }

            if to_send.is_empty() {
                app_core.needs_render = true;
                tracing::debug!("handle_command_submission: no-op command");
                return Ok(None);
            }

            app_core.needs_render = true;
            tracing::debug!("handle_command_submission: queued for network");
            return Ok(Some(to_send));
        }
        tracing::debug!("handle_command_submission: end");
        Ok(None)
    }

    fn handle_quickbar_action(
        &mut self,
        action: super::quickbar::QuickbarAction,
        window_name: &str,
        app_core: &mut crate::core::AppCore,
    ) -> Option<String> {
        match action {
            super::quickbar::QuickbarAction::OpenSwitcher => {
                if let Some(window) = app_core.ui_state.get_window(window_name) {
                    self.open_quickbar_switcher(app_core, window.position.clone());
                    app_core.needs_render = true;
                }
                None
            }
            super::quickbar::QuickbarAction::ExecuteCommand(command) => Some(command),
            super::quickbar::QuickbarAction::MenuRequest { exist, noun } => {
                let click_pos = app_core
                    .ui_state
                    .get_window(window_name)
                    .map(|w| (w.position.x, w.position.y))
                    .unwrap_or((0, 0));
                Some(app_core.request_menu(exist, noun, click_pos))
            }
        }
    }

    /// Handle keyboard input when the login wizard is active.
    pub(super) fn handle_wizard_keys(
        &mut self,
        code: crate::frontend::KeyCode,
        modifiers: crate::frontend::KeyModifiers,
        app_core: &mut crate::core::AppCore,
    ) -> Result<Option<String>> {
        use super::login_wizard::PickerResult;
        use crate::frontend::KeyCode;

        let picker = match self.login_wizard.as_mut() {
            Some(p) => p,
            None => return Ok(None),
        };

        match code {
            KeyCode::Esc => {
                picker.back();
            }
            KeyCode::Tab => {
                picker.tab();
            }
            KeyCode::Up => {
                picker.move_up();
            }
            KeyCode::Down => {
                picker.move_down();
            }
            KeyCode::Backspace => {
                picker.backspace();
            }
            KeyCode::Char('n') | KeyCode::Char('N') if !modifiers.ctrl && !modifiers.alt => {
                // Only trigger N/E/D in list mode (type_char handles edit mode)
                if picker.profiles().is_empty()
                    || matches!(code, KeyCode::Char('n') | KeyCode::Char('N'))
                {
                    // Try new_profile first; if in edit mode, type_char handles it
                    picker.new_profile();
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') if !modifiers.ctrl && !modifiers.alt => {
                picker.edit_selected();
            }
            KeyCode::Char('d') | KeyCode::Char('D') if !modifiers.ctrl && !modifiers.alt => {
                picker.delete_selected();
                // Save updated profiles
                let profiles: Vec<crate::connection::Profile> = picker
                    .profiles()
                    .iter()
                    .map(|p| crate::connection::Profile {
                        name: p.name.clone(),
                        account: p.account.clone(),
                        character: p.character.clone(),
                        game_code: p.game_code.clone(),
                        use_lich: p.use_lich,
                        lich_host: p.lich_host.clone(),
                        lich_port: p.lich_port,
                    })
                    .collect();
                let store = crate::connection::ProfileStore { profiles };
                if let Err(e) = store.save() {
                    tracing::warn!("Failed to save profiles: {}", e);
                }
            }
            KeyCode::Char(c) if !modifiers.ctrl && !modifiers.alt => {
                picker.type_char(c);
            }
            KeyCode::Enter => {
                let needs_fetch = picker.confirm();
                if needs_fetch {
                    if let Some((account, password, game_code)) = self
                        .login_wizard
                        .as_ref()
                        .and_then(|p| p.get_fetch_params())
                    {
                        let data_dir = crate::config::Config::base_dir().unwrap_or_default();
                        match crate::network::fetch_characters_for_account(
                            &account, &password, &game_code, &data_dir,
                        ) {
                            Ok(chars) => {
                                if let Some(p) = self.login_wizard.as_mut() {
                                    p.set_characters(chars);
                                }
                            }
                            Err(e) => {
                                if let Some(p) = self.login_wizard.as_mut() {
                                    p.set_error(format!("Login failed: {}", e));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        app_core.needs_render = true;

        // Check if picker produced a result
        let result = self.login_wizard.as_ref().and_then(|p| p.result.clone());
        match result {
            Some(PickerResult::Quit) => {
                self.login_wizard = None;
                self.show_setup_screen = false;
                app_core.quit();
            }
            Some(PickerResult::Connect(profile)) => {
                self.show_setup_screen = false;
                // Save profile to profiles.toml
                let conn_profile = crate::connection::Profile {
                    name: profile.name.clone(),
                    account: profile.account.clone(),
                    character: profile.character.clone(),
                    game_code: profile.game_code.clone(),
                    use_lich: profile.use_lich,
                    lich_host: profile.lich_host.clone(),
                    lich_port: profile.lich_port,
                };
                // Save all profiles (picker may have been edited)
                let profiles: Vec<crate::connection::Profile> = self
                    .login_wizard
                    .as_ref()
                    .map(|p| {
                        p.profiles()
                            .iter()
                            .map(|pr| crate::connection::Profile {
                                name: pr.name.clone(),
                                account: pr.account.clone(),
                                character: pr.character.clone(),
                                game_code: pr.game_code.clone(),
                                use_lich: pr.use_lich,
                                lich_host: pr.lich_host.clone(),
                                lich_port: pr.lich_port,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut store = crate::connection::ProfileStore { profiles };
                store.add_or_update(conn_profile);
                if let Err(e) = store.save() {
                    tracing::warn!("Failed to save profiles: {}", e);
                }
                // Store password in keychain
                if let Some(ref p) = self.login_wizard {
                    if let Some((account, password, _)) = p.get_fetch_params() {
                        crate::credentials::store_password(&account, &password);
                    }
                }
                self.login_wizard = None;
                // Signal runtime to connect
                if profile.use_lich {
                    return Ok(Some(format!(
                        "//setup:connect:lich:{}:{}",
                        profile.lich_host(),
                        profile.lich_port()
                    )));
                } else {
                    return Ok(Some(format!(
                        "//setup:connect:direct:{}:{}:{}",
                        profile.account, profile.game_code, profile.character
                    )));
                }
            }
            None => {}
        }

        Ok(None)
    }
}
