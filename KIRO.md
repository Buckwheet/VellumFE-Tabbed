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

## Current State (Session 22 — commit `146e629`, tag `v0.2.0-beta.25`)

`cargo check` clean. **v0.2.0-beta.25 released** — awaiting user test of full login flow.

### Session 22 — Deep dive on Warlock SGE flow + fixes (current)

Compared Warlock's `SgeClientImpl.kt` against our `network.rs`. Found and fixed three bugs:

**beta.24** — `fix(wizard): switch active session to new Direct session after connect`
- Root cause: `session_manager.add()` only auto-activates if no sessions exist. Since a Lich
  session (session 0) always exists, the new Direct session was added but never switched to.
  User saw the game screen UI but it was still showing the disconnected Lich session.
- Fix: added `session_manager.set_active(id)` + `do_session_switch(...)` in `//wizard:connect:` handler.

**beta.25** — `fix(eaccess): match Warlock SGE flow — remove F/P commands, fix parse_launch_response`
- Warlock's authenticate flow: `K → A → G\t{game_code} → C → L\t{char_code}\tSTORM`
- Our code was sending `F\t{game_code}` and `P\t{game_code}` which Warlock does NOT send.
  The `F` command returned `X\tPROBLEM` which may have broken eAccess state machine.
- `parse_launch_response` had a broken double-`strip_prefix` — second call fell back to
  the original string (with `L\t` prefix), causing all `key=value` parsing to fail silently.
- Fixed both `authenticate()` and `fetch_characters()`. Removed all beta.23 debug log lines.

**NEXT**: User tests beta.25 — go through wizard, select character, confirm game feed appears.
If it works → update KIRO.md, tag `v0.2.0` stable.

### Session 21 — eAccess authenticate debug

- beta.21: Fixed game code `GS3` → `GS4` in `login_wizard.rs` GAMES array and `network.rs`
- beta.22: Fixed `fetch_characters` missing `G\t{game_code}` and `P\t{game_code}` before `C`
  - Characters confirmed: `C\t2\t16\t1\t1\tW_HOGGD_000\tBrashka\tW_HOGGZ_W000\tMejora`
- beta.23: Added debug logging to `authenticate` F/G/P/C/L responses (removed in beta.25)

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
3. ~~**CI test failures**~~ — DONE (Session 12)
4. ~~**macOS package step binary name**~~ — DONE (Session 13)
5. ~~**Release job permissions**~~ — DONE (Session 13)
6. ~~**Pre-commit hooks**~~ — DONE (Session 14)
7. ~~**First-run blank screen**~~ — DONE (Session 15)
8. ~~**Windows double-click crash**~~ — DONE (Session 17)
9. ~~**Invalid app type crash**~~ — DONE (Session 18)
10. ~~**Blank character select screen**~~ — DONE (Session 19)
11. ~~**Empty character list (GS3→GS4)**~~ — DONE (Session 20/21)
12. ~~**fetch_characters missing G+P commands**~~ — DONE (beta.22)
13. ~~**authenticate failure after character select**~~ — DONE (beta.24 + beta.25)
14. ~~**Session not switched after wizard connect**~~ — DONE (beta.24)
15. **Confirm end-to-end working** — awaiting user test of beta.25
16. **Promote to v0.2.0 stable** once binary confirmed working end-to-end
17. **Bak file cleanup** — deferred until first working release binary confirmed
18. **Clippy tech debt** — 283 pre-existing warnings; address incrementally

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
