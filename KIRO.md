# KIRO.md — Steering Document for VellumFE-Tabbed

This file is the first thing Kiro reads each session. It contains everything needed to
resume work immediately without re-reading source files.

---

## Project

Multi-session GemStone IV terminal frontend. Rust + Ratatui. Up to 15 simultaneous sessions.
Binary: `vellum-fe-tabbed`. Repo: https://github.com/Buckwheet/VellumFE-Tabbed
Local: `~/VellumFE-Tabbed/`

**Rule**: Before modifying any file, create a `.bak` backup first.

**IMPORTANT — Testing machine**: User builds and tests on a **separate Windows machine**.
Kiro CAN read files from it via WSL mount at `/mnt/c/`. Always check there first before
asking the user to paste files.
- Log file: `/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/vellum-fe.log`
- Config dir: `C:\Users\rpgfi\.vellum-fe\` (NOT accessible via /mnt/c — different machine)
- Project dir on Windows: `C:\Users\rpgfi\Documents\GSIV Development\VellumFE-Tabbed\`
  accessible as: `/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/`
- sidebar.toml source of truth: `/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/sidebar.toml`

---

## Current State (Session 30)

`cargo build` clean. Three fixes shipped across sessions 29–30:

### Fix 1: sidebar.toml parse failure — `v0.1.0-beta.36` (`aba4a8c`)
- Root cause: `ActiveEffectsWidgetData.category` had no `#[serde(default)]` and no alias
  for `effect_category` (the field name used in the user's saved TOML)
- Fix: added `#[serde(alias = "effect_category", default)]` to `category` in `ActiveEffectsWidgetData`
- File: `src/config.rs`
- User must copy `sidebar_fixed.toml` (repo root) to `C:\Users\rpgfi\.vellum-fe\layouts\sidebar.toml`

### Fix 2: Ctrl+C force-quit — `v0.1.0-beta.36` (`aba4a8c`)
- User couldn't exit app during/after failed auth — no obvious escape path
- Fix: added Ctrl+C as universal force-quit at top of `handle_normal_mode_keys`,
  fires before picker/wizard routing
- File: `src/frontend/tui/input_handlers.rs`

### Fix 3: Mouse capture disabled by default — `v0.1.0-beta.37` (`58a37b1`)
- Root cause: `EnableMouseCapture` in `TuiFrontend::new()` caused Windows terminal to steal
  mouse focus from all other windows on startup
- Fix: added `mouse_capture: bool` field to `UiConfig` (default `false`); `TuiFrontend::new()`
  now takes `mouse_capture: bool` and conditionally calls `EnableMouseCapture`/`DisableMouseCapture`
- Files: `src/config.rs`, `src/frontend/tui/mod.rs`, `src/frontend/tui/frontend_impl.rs`,
  `src/frontend/tui/runtime.rs`
- To re-enable: add `mouse_capture = true` under `[ui]` in config

### Current next steps
User needs to:
1. Pull latest / rebuild on Windows (beta.37 tag)
2. Copy `sidebar_fixed.toml` → `C:\Users\rpgfi\.vellum-fe\layouts\sidebar.toml`
3. Confirm app opens without freezing other windows (beta.37 fix)
4. Try `.layout sidebar` — should load now (beta.36 fix)
5. Confirm Brashka/Makerol tabs visible and game text flows

---

## Key Files

```
src/frontend/tui/session_picker.rs   — password prompt added (beta.34)
src/frontend/tui/input_handlers.rs   — ConnectWithPassword action → //picker:connect_pw:
src/frontend/tui/runtime.rs          — //picker:connect_pw: handler
src/sessions_config.rs               — PartialEq added to SessionModeConfig
src/config.rs                        — serde aliases for legacy widget types
src/core/app_core/layout.rs          — layout load error now logs full chain ({:?})
```

---

## Password Prompt Flow (beta.34)

1. User selects Direct session in picker → `confirm()` → `PickerFocus::PasswordPrompt`
2. User types password → `type_char()` appends to `password_input`
3. User hits Enter → `ConnectWithPassword(idx, pw)` action
4. `input_handlers.rs` converts to `//picker:connect_pw:{idx}\x00{password}`
5. `runtime.rs` handler: calls `credentials::store_password(account, pw)`,
   sets `*password = pw.to_string()` on session mode, then `spawn_session_network`

---

## Layout Widget Type Names (current valid values)

From `src/config.rs` `WindowDef` enum (serde rename/alias):

| TOML value | Notes |
|---|---|
| `text` | plain text window |
| `tabbedtext` | tabbed text (alias: `tabbed`) |
| `command_input` | command input bar |
| `progress` | progress bar |
| `compass` | compass widget |
| `hand` | hand display (aliases: `lefthand`, `righthand`, `spellhand`) |
| `injury_doll` | injury doll |
| `countdown` | countdown timer |
| `dashboard` | status dashboard |
| `active_effects` | buffs/debuffs/cooldowns |
| `players` | player list |
| `targets` | target list |

**Note**: Old `entity` type has no alias — must be manually changed to `players` or `targets`.

---

## sidebar.toml Status

- `sidebar_fixed.toml` in repo root has all corrections applied
- User copied it to `C:\Users\rpgfi\.vellum-fe\layouts\sidebar.toml`
- Parse is still failing — error detail now visible after `aa29c1d` rebuild
- Once parse succeeds, layout should load cleanly

---

## After Layout Confirmed Working

Remove debug scaffolding from `network.rs`:
- `eaccess_raw_debug` function
- `raw_debug` function
- `fetch_characters` debug log line

Then tag `v0.2.0`.

---

## Backup Files (do not delete until v0.2.0 tagged)

```
src/network.rs.bak, .bak2, .bak5
src/frontend/tui/runtime.rs.bak, .bak2, .bak3, .bak4
src/frontend/tui/frontend_impl.rs.bak3
src/frontend/tui/input_handlers.rs.bak
src/main.rs.bak
src/core/app_core/state.rs.bak
src/frontend/tui/tab_bar.rs.bak
src/config.rs.bak6
src/frontend/tui/session_picker.rs.bak
```

---

## Architecture

### Per-Session Isolation

Each session owns:

| Field | Type | Purpose |
|-------|------|---------|
| `server_tx/server_rx` | `mpsc::UnboundedChannel<ServerMessage>` | Network → main loop |
| `command_tx/command_rx` | `mpsc::UnboundedChannel<String>` | Main loop → network |
| `AppCore` | `HashMap<SessionId, AppCore>` in runtime | Parser, game state, UI, config |
| `unread` | `Arc<AtomicUsize>` | Badge counter (lock-free) |
| `active_session_id` | `Arc<AtomicUsize>` | Shared across all sessions |

### Key Source Files

```
src/session/mod.rs              Session struct, ConnectionMode, SessionStatus
src/session_manager.rs          SessionManager — Vec<Session>, active_id, shared atomic
src/sessions_config.rs          sessions.toml load/save
src/credentials.rs              OS keychain (keyring crate)
src/frontend/tui/runtime.rs     Main event loop, spawn_session_network, app_cores map
src/frontend/tui/mod.rs         TuiFrontend struct, session_labels tuple
src/frontend/tui/tab_bar.rs     TabBar widget, TabEntry struct
src/frontend/tui/frontend_impl.rs  render(), tab_entries construction
src/frontend/tui/session_picker.rs Session picker TUI
src/frontend/tui/login_wizard.rs   Direct eAccess login wizard
src/core/app_core/state.rs      AppCore struct definition
src/core/app_core/layout.rs     Layout load/save
src/config.rs                   Config::load_with_options(character, port)
src/network.rs                  LichConnection, DirectConnection, ServerMessage
```

### Internal Command Protocol (runtime.rs)

```
//picker:connect:N          switch to session N, close picker
//picker:connect_pw:N\x00PW connect session N with password PW
//picker:remove:N           remove session N from config
//picker:add                save Lich session entry
//picker:open_wizard         open login wizard
//picker:quit               close picker
//wizard:fetch_chars         blocking fetch from eAccess
//wizard:connect:acct:pw:game:char  add Direct session
//wizard:cancel             close wizard
//session:switch:N          set_active_by_index(N)
//session:next/prev         next()/prev()
//session:compact           toggle compact_tabs
//session:broadcast         set broadcast_next = true
//session:sound             toggle sound_enabled on active session
//session:tts               toggle tts_enabled on active session
```

### ConnectionMode

```rust
pub enum ConnectionMode {
    LichProxy { host: String, port: u16, login_key: Option<String> },
    Direct { account: String, password: String, character: String, game_code: String },
}
```

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

## Config Paths

```
~/.config/vellum-fe-tabbed/sessions.toml       session list
~/.vellum-fe/<character>/config.toml           per-character config
~/.vellum-fe/default/config.toml               global config
~/.vellum-fe/layouts/<name>.toml               layout files
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
