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

## Current State (Session 26 — commit `ebf22e1`, tag `v0.2.0-beta.33`)

`cargo build` clean. **v0.2.0-beta.33 released** — picker now pre-populates sessions from config at startup; no phantom sessions; tab bar should show real labels.

### Session 26 — Fix picker index mismatch (beta.33)

**Root cause of beta.32 bug**: The empty-placeholder Direct session created at startup was
session_manager index 0. When user selected Brashka (picker index 0), `set_active_by_index(0)`
activated the placeholder → auth with empty credentials → `Authentication failed for account : ?`

**Fix**: Instead of creating a placeholder, pre-populate `session_manager` from `sessions_config`
at startup (without connecting). Picker index N now maps directly to session_manager index N.
When user selects from picker, the session already exists with correct credentials — just spawn
the network connection.

**Also removed**: The placeholder cleanup block from `//picker:connect:` handler (no longer needed).

**Dedup still works**: `//wizard:connect:` handler finds existing session by character name and
reuses it (updating password), so Ctrl+N wizard connecting Brashka reuses the pre-populated entry.

**NEXT**:
1. User rebuilds on Windows and tests: should see "Brashka" and "Makerol" tabs in tab bar on startup
2. Open picker, select Brashka → should connect with correct credentials
3. If tab bar still invisible, investigate `frontend_impl.rs` layout (tab bar height allocation)
4. Once two sessions confirmed working: remove debug scaffolding from `network.rs`
   (`eaccess_raw_debug`, `raw_debug`, `fetch_characters` debug log line), tag `v0.2.0`

---

## Previous State (Session 24 — commit `4b47994`, tag `v0.2.0-beta.31`)

`cargo check` clean. **v0.2.0-beta.31 released** — eAccess game code bug fixed. Ready for user test.

### Session 24 — Root cause found: wrong eAccess game code

**beta.30** — `fix: single eAccess call; {:#} error logging`
- Extracted shared `login_and_select_game` helper (K→A→G) in `network.rs`
- Changed error logging from `{}` to `{:#}` in `runtime.rs` Direct connection error handler
- This revealed the real error: `Launch failed: PROBLEM`

**beta.31** — `fix: deduplicate sessions on wizard connect; add eaccess-test binary; fix GS3 game code`
- Fixed duplicate sessions bug: `//wizard:connect:` handler now checks for existing session
  with same character name before calling `session_manager.add()` — reuses existing session
- Added `eaccess-test` binary (`src/bin/eaccess_test.rs`) — standalone Windows exe for
  testing full eAccess flow (K→A→G→C→L) without running the full app
- Added `eaccess_raw_debug` public function in `network.rs` that prints every raw send/recv
- **Root cause confirmed via raw debug**: `G\tGS4` → `X\tPROBLEM` (eAccess rejects `GS4`)
  - Correct game code for GemStone IV Prime is `GS3` (confirmed by Lich log and `.sal` file)
  - Fixed `GAMES` array in `login_wizard.rs`: `"GS4"` → `"GS3"`
  - Fixed `game_name_to_code()` in `network.rs`: `"prime" | "gs4" | "gs3" => "GS3"`

**NEXT**:
1. User edits `C:\Users\rpgfi\.vellum-fe\sessions.toml` — change `game_code = "GS4"` to
   `game_code = "GS3"` on Brashka entry, delete duplicate Brashka entries (leave only one)
2. `cargo build` in WSL, launch app, connect Brashka — confirm game text flows
3. Once confirmed: remove `fetch_characters raw response` debug log from `network.rs`,
   remove `eaccess_raw_debug` / `raw_debug` functions (debug scaffolding), tag `v0.2.0`

### Session 23 — Fix: Direct session from picker never connected

**beta.28** — Ctrl+N opens login wizard (`input_handlers.rs`)
**beta.29** — spawn Direct network on picker connect for saved sessions (`runtime.rs`)

### Session 22 — Deep dive on Warlock SGE flow + fixes

**beta.24** — session not switched after wizard connect (set_active + do_session_switch)
**beta.25** — removed bogus F/P commands; fixed parse_launch_response double-strip_prefix
**beta.26** — fixed auth response check (KEY vs username)
**beta.27** — local timestamps in log; remove dead Lich placeholder tab after wizard connect

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
13. ~~**authenticate failure after character select**~~ — DONE (beta.24–26)
14. ~~**Session not switched after wizard connect**~~ — DONE (beta.24)
15. ~~**Auth response check wrong (KEY vs username)**~~ — DONE (beta.26)
16. ~~**Dead Lich placeholder tab after wizard connect**~~ — DONE (beta.27)
17. ~~**Log timestamps in UTC instead of local time**~~ — DONE (beta.27)
18. **Confirm end-to-end working** — user must edit sessions.toml (GS4→GS3), build, test
19. **Promote to v0.2.0 stable** once game text confirmed flowing
    - Remove `fetch_characters raw response` debug log from `network.rs`
    - Remove `eaccess_raw_debug` and `raw_debug` functions from `network.rs`
20. **Bak file cleanup** — deferred until first working release binary confirmed
21. **Clippy tech debt** — 283 pre-existing warnings; address incrementally

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
