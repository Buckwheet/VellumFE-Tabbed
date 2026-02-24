# KIRO.md — Steering Document for VellumFE-Tabbed

This file is the first thing Kiro reads each session. It contains everything needed to
resume work immediately without re-reading source files.

---

## Project

Multi-session GemStone IV terminal frontend. Rust + Ratatui. Up to 15 simultaneous sessions.
Binary: `vellum-fe-tabbed`. Repo: https://github.com/Buckwheet/VellumFE-Tabbed
Local: `~/VellumFE-Tabbed/`

**Rule**: Before modifying any file, create a `.bak` backup first.

---

## Current State (Session 13 — commit `ce90515`)

`cargo check` clean. `cargo test` passes. beta.14 build in progress on GitHub Actions.

Two bugs fixed this session:
1. `vellum_fe` → `vellum_fe_tabbed` in `tests/ui_integration.rs`, `tests/parser_integration.rs`, `src/theme.rs` doctests (CI was failing with E0433)
2. `beta-release.yml` macOS package steps were copying `target/release/vellum-fe` (old binary name) instead of `target/release/vellum-fe-tabbed`

---

## Architecture

### Per-Session Isolation (fully implemented)

Each session owns:

| Field | Type | Purpose |
|-------|------|---------|
| `server_tx/server_rx` | `mpsc::UnboundedChannel<ServerMessage>` | Network → main loop |
| `command_tx/command_rx` | `mpsc::UnboundedChannel<String>` | Main loop → network |
| `AppCore` | `HashMap<SessionId, AppCore>` in runtime | Parser, game state, UI, config |
| `unread` | `Arc<AtomicUsize>` | Badge counter (lock-free) |
| `active_session_id` | `Arc<AtomicUsize>` | Shared across all sessions |

### Key Files

```
src/session/mod.rs              Session struct, ConnectionMode, SessionStatus
src/session_manager.rs          SessionManager — Vec<Session>, active_id, shared atomic
src/sessions_config.rs          sessions.toml load/save
src/credentials.rs              OS keychain (keyring crate)
src/frontend/tui/runtime.rs     Main event loop, spawn_session_network, app_cores map
src/frontend/tui/mod.rs         TuiFrontend struct, session_labels tuple
src/frontend/tui/tab_bar.rs     TabBar widget, TabEntry struct
src/frontend/tui/frontend_impl.rs  render(), tab_entries construction
src/frontend/tui/session_keys.rs   SessionCmd enum, parse(), key string helpers
src/frontend/tui/input_handlers.rs Keyboard shortcut wiring
src/frontend/tui/session_picker.rs Session picker TUI
src/frontend/tui/login_wizard.rs   Direct eAccess login wizard
src/core/app_core/state.rs      AppCore struct definition
src/config.rs                   Config::load_with_options(character, port)
src/network.rs                  LichConnection, DirectConnection, ServerMessage
```

### session_labels tuple (6-tuple)

```rust
// (label, is_active, status_symbol, unread_count, sound_enabled, tts_enabled)
pub session_labels: Vec<(String, bool, String, usize, bool, bool)>,
```

Status symbols: `●` connected · `○` disconnected · `…` connecting · `↻` reconnecting · `!` error

### TabEntry

```rust
pub struct TabEntry<'a> {
    pub label: &'a str,
    pub is_active: bool,
    pub status: &'a str,
    pub unread: usize,
    pub sound_enabled: bool,   // shows 🔇 when false
    pub tts_enabled: bool,     // shows 🔕 when false
}
```

### ConnectionMode

```rust
pub enum ConnectionMode {
    LichProxy { host: String, port: u16, login_key: Option<String> },
    Direct { account: String, password: String, character: String, game_code: String },
}
```

### SessionCmd (session_keys.rs)

```rust
pub enum SessionCmd {
    SwitchToIndex(usize), Next, Prev, New, Close,
    ToggleCompact, Broadcast, ToggleSound, ToggleTts,
}
```

Keyboard shortcuts:
- `Ctrl+1..9` → switch session
- `Ctrl+Tab` / `Ctrl+Shift+Tab` → next/prev
- `Ctrl+B` → broadcast next command to all sessions
- `Ctrl+Shift+C` → toggle compact tab bar
- `Ctrl+Shift+S` → toggle sound for active session
- `Ctrl+Shift+T` → toggle TTS for active session

### Internal Command Protocol (runtime.rs)

```
//picker:connect:N     switch to session N, close picker
//picker:remove:N      remove session N from config
//picker:add           save Lich session entry
//picker:open_wizard   open login wizard
//picker:quit          close picker
//wizard:fetch_chars   blocking fetch from eAccess
//wizard:connect:acct:pw:game:char  add Direct session
//wizard:cancel        close wizard
//session:switch:N     set_active_by_index(N)
//session:next/prev    next()/prev()
//session:compact      toggle compact_tabs
//session:broadcast    set broadcast_next = true
//session:sound        toggle sound_enabled on active session
//session:tts          toggle tts_enabled on active session
```

### Lich Script Protocol

```xml
<vellumfe cmd="highlight.add" pattern="Buckwheet" fg="#ff00ff" bold="true" persist="true"/>
<vellumfe cmd="highlight.remove" pattern="Buckwheet"/>
<vellumfe cmd="highlight.clear" category="Friends"/>
<vellumfe cmd="squelch.add" pattern="A cool breeze"/>
```

### Main Loop Pattern (runtime.rs)

```rust
// Get active session's AppCore each iteration
let active_sid = session_manager.active().map(|s| s.id);
let app_core: &mut AppCore = /* raw ptr from app_cores.get_mut(&id) */;

// Poll active session's server_rx
if let Some(rx) = session_rxs.get_mut(&active_id) {
    while let Ok(msg) = rx.try_recv() { ... }
}

// Route commands to active session's command_tx
session_manager.active().and_then(|s| s.command_tx.as_ref()).map(|tx| tx.send(cmd));
```

### Adding a New Session (picker/wizard)

1. `session_manager.add(label, mode)` → returns `SessionId`
2. Take `s.server_rx` → insert into `session_rxs`
3. `spawn_session_network(s, raw_logger)` → sets `s.command_tx`, spawns network task
4. `create_app_core_for_session(&mode, &config)` → insert into `app_cores`
5. `widget_managers.insert(id, WidgetManager::new())` — fresh widget state for new session

### Session Switch (do_session_switch helper)

```rust
fn do_session_switch(prev_id, new_id, frontend, widget_managers) {
    // Save outgoing: swap frontend.widget_manager → widget_managers[prev_id]
    // Restore incoming: widget_managers.remove(new_id) → swap into frontend
    // If no saved state for incoming, fresh WidgetManager stays in frontend
}
```

Call at all switch sites: `SessionCmd::SwitchToIndex/Next/Prev`, `//picker:connect:`

### HighlightPattern construction (no Default impl)

```rust
crate::config::HighlightPattern {
    pattern: "...".to_string(),
    fg: None, bg: None, bold: false, color_entire_line: false,
    fast_parse: false, sound: None, sound_volume: None, category: None,
    squelch: false, silent_prompt: false, redirect_to: None,
    redirect_mode: Default::default(), replace: None, stream: None,
    window: None, compiled_regex: None,
}
```

---

## Remaining Work

Priority order:

1. ~~**Session switch UI state save/restore**~~ — DONE (Session 10)
2. ~~**TTS state in tab bar**~~ — DONE (Session 11)
3. ~~**CI test failures**~~ — DONE (Session 12) — `vellum_fe` → `vellum_fe_tabbed` in tests + doctests
4. ~~**macOS package step binary name**~~ — DONE (Session 13) — `beta-release.yml` was copying `vellum-fe` not `vellum-fe-tabbed`
5. **Verify beta.14 build passes** on all platforms (Linux, Windows, macOS arm64/x64/universal)
5. **Test the binary** — run against a real GemStone account, verify Lich proxy and Direct login
6. **Promote to v0.2.0 stable** once binary is confirmed working
7. **Bak file cleanup** — deferred until first working release binary is shipped on GitHub. Bak files are safety nets until then.
8. **Phase 5.3 — Session grouping UI** — deferred, complex, low priority

---

## Config Paths

```
~/.config/vellum-fe-tabbed/sessions.toml       session list
~/.vellum-fe/<character>/config.toml           per-character config
~/.vellum-fe/default/config.toml               global config
```

Config loading: `Config::load_with_options(character: Option<&str>, port: u16)`

---

## Cargo.toml Notable Dependencies

```toml
keyring = "2"       # OS keychain
ratatui = "..."     # TUI rendering
tokio = { features = ["full"] }
```

---

## How to Resume

Tell Kiro: **"Read KIRO.md from VellumFE-Tabbed and continue"**

Kiro reads this file, checks `cargo check`, reads only the specific files needed for the
next task, and starts coding immediately.
