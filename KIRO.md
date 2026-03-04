# KIRO.md — Steering Document for VellumFE

This file is the first thing Kiro reads each session. It contains everything needed to
resume work immediately without re-reading source files.

**Resume phrase**: "Read KIRO.md from VellumFE-Tabbed and continue"

---

## Project

VellumFE — GemStone IV terminal frontend with Warlock-style login manager.
Rust + Ratatui. Binary: `vellum-fe-tabbed`.
Repo: https://github.com/Buckwheet/VellumFE-Tabbed
Local: `~/VellumFE-Tabbed/`

**Rule**: Before modifying any file, create a `.bak` backup first.

**IMPORTANT — Testing machine**: User builds and tests on a **separate Windows machine**.
Kiro CAN read files from it via WSL mount at `/mnt/c/`. Always check there first before
asking the user to paste files.
- Log file: `/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/vellum-fe.log`
- Config dir: `C:\Users\rpgfi\.vellum-fe\` (NOT accessible via /mnt/c — different machine)

---

## Goal

Mimic Warlock's login service for VellumFE:
- Save named character profiles (account + character + game + optional Lich proxy)
- Passwords stored in OS keychain, never on disk
- Profile picker TUI shown on every launch (Warlock-style)
- Per-profile toggle: Direct (eAccess) vs Lich proxy

This is a **single-session** frontend. Multi-session/tab-bar features were removed.

---

## Current State — beta.43 (HEAD: bdd3bf3, tag: v0.1.0-beta.43)

### What's working

- Multi-profile `profiles.toml` replaces single `connection.toml`
- `ProfilePicker` TUI widget (Warlock-style list + edit form)
- **Picker always shows on launch** — fixed in beta.43
- N/E/D hotkeys gated on list mode — fixed in beta.42
  - In edit mode, N/E/D type as characters into the active field
- **Game selector** — Up/Down navigates fields, Left/Right cycles game
- **sidebar.toml CRLF fixed** — beta.43
- Password stored in OS keychain via `credentials.rs`
- Character fetch from eAccess on Enter in Character field
- Direct and Lich connection paths both wired up

### Architecture

```
src/connection.rs          — ProfileStore, Profile struct, profiles.toml
src/credentials.rs         — OS keychain (store/get/delete password)
src/network.rs             — eaccess module, DirectConnection, LichConnection
src/frontend/tui/
  login_wizard.rs          — ProfilePicker TUI widget
  runtime.rs               — startup: always shows picker; event loop
  input_handlers.rs        — handle_wizard_keys(); N/E/D gated; Left/Right → cycle_game
  frontend_impl.rs         — renders ProfilePicker overlay
  mod.rs                   — TuiFrontend.login_wizard: Option<ProfilePicker>
defaults/globals/layouts/
  sidebar.toml             — CRLF corruption fixed (beta.43)
```

### profiles.toml format (~/.vellum-fe/profiles.toml)

```toml
[[profiles]]
name = "Brashka (Prime)"
account = "myaccount"
character = "Brashka"
game_code = "GS3"
use_lich = false
```

### Connection commands (internal)

- `//setup:connect:direct:<account>:<game_code>:<character>` — spawn DirectConnection
- `//setup:connect:lich:<host>:<port>` — spawn LichConnection

### GAMES list (login_wizard.rs)

```rust
pub const GAMES: &[(&str, &str)] = &[
    ("GS3", "GemStone IV (Prime)"),
    ("GSX", "GemStone IV (Platinum)"),
    ("GSF", "GemStone IV (Shattered)"),
    ("DR", "DragonRealms"),
];
```

---

## Known Bugs

1. **Paste in edit form** — paste events arrive as rapid individual `Char` events.
   Proper fix:
   - Add `pub fn paste_str(&mut self, s: &str)` to `ProfilePicker`
   - Handle `KeyCode::Paste(text)` in `handle_wizard_keys`
   - May need to enable bracketed paste mode in terminal setup

---

## Next Steps

1. **User re-tests beta.43 on Windows** — verify picker shows on every launch, game selector works
2. **Purge dead code** — remove orphaned multi-session files (see below)
3. **Fix paste** if still broken
4. **Lich subprocess launch** — currently assumes Lich is already running on configured port
5. **`--character <name>` CLI flag** — skip picker and connect directly to named profile

---

## Dead Code to Remove (orphaned multi-session files)

These files are no longer referenced by anything in the active codebase:

```
src/session/                        — per-session state (old multi-session arch)
src/session_manager.rs              — SessionManager (old multi-session arch)
src/sessions_config.rs              — sessions.toml loader (replaced by profiles.toml)
src/frontend/tui/session_picker.rs  — old session picker TUI
src/frontend/tui/session_keys.rs    — old session keyboard handling
src/frontend/tui/input_handlers.rs.bak
src/frontend/tui/login_wizard.rs.bak
src/frontend/tui/runtime.rs.bak
defaults/globals/layouts/sidebar.toml.bak
```

Verify each with `grep -rn "<symbol>" src/` before deleting.

---

## Files NOT to touch (core game engine)

- src/parser.rs
- src/theme.rs
- src/config.rs
- src/core/ (all files)
