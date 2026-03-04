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
- Profile picker TUI shown on every launch (Warlock-style), last-used pre-selected
- Per-profile toggle: Direct (eAccess) vs Lich proxy

---

## Current State — beta.43 (HEAD: bdd3bf3, tag: v0.1.0-beta.43)

### What's working

- Multi-profile `profiles.toml` replaces single `connection.toml`
- `ProfilePicker` TUI widget (Warlock-style list + edit form)
- **Picker always shows on launch** — fixed in beta.43 (was auto-connecting with profiles[0])
- N/E/D hotkeys gated on list mode — fixed in beta.42
  - In edit mode, N/E/D type as characters into the active field
- **Game selector fixed** — beta.43: Up/Down navigates fields, Left/Right cycles game
  - Removed game cycling from move_up/move_down; added `cycle_game(forward)` method
  - Left/Right in handle_wizard_keys calls picker.cycle_game()
- **sidebar.toml CRLF fixed** — beta.43: literal `\r\n` text in line 392 replaced with real newlines
- Password stored in OS keychain via `credentials.rs`
- Character fetch from eAccess on Enter in Character field
- Direct and Lich connection paths both wired up

### Architecture

```
src/connection.rs          — ProfileStore, Profile struct, profiles.toml
src/credentials.rs         — OS keychain (store/get/delete password)
src/network.rs             — eaccess module (authenticate, fetch_characters),
                             DirectConnection, LichConnection
src/frontend/tui/
  login_wizard.rs          — ProfilePicker TUI widget; cycle_game() added beta.43
  runtime.rs               — startup: always shows picker (else { None } — beta.43);
                             event loop: handle //setup:connect:* commands
  input_handlers.rs        — handle_wizard_keys(); N/E/D gated; Left/Right → cycle_game
  frontend_impl.rs         — renders ProfilePicker overlay
  mod.rs                   — TuiFrontend.login_wizard: Option<ProfilePicker>
defaults/globals/layouts/
  sidebar.toml             — fixed CRLF corruption (beta.43)
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

---

## Known Bugs

1. **Paste in edit form** — paste events arrive as rapid individual `Char` events.
   Now that N/E/D are mode-gated, uppercase chars in pasted passwords won't be intercepted,
   so paste may work better in practice. But a proper fix would be:
   - Add `pub fn paste_str(&mut self, s: &str)` to `ProfilePicker` (iterates chars → type_char)
   - Handle `KeyCode::Paste(text)` in `handle_wizard_keys` (crossterm bracketed paste)
   - May need to enable bracketed paste mode in terminal setup

---

## Next Steps

1. **User re-tests beta.43 on Windows** — verify picker shows on every launch, game selector works
2. **Fix paste** (bug above) if still broken
3. **Lich subprocess launch** — currently assumes Lich is already running on configured port
4. **`--character <name>` CLI flag** — skip picker and connect directly to named profile

---

## Files NOT to touch (core game engine)

- src/parser.rs
- src/theme.rs
- src/config.rs
- src/core/ (all files)
