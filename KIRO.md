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
- On launch with no profiles: show profile picker TUI
- Per-profile toggle: Direct (eAccess) vs Lich proxy

---

## Current State — beta.42 (HEAD: edd4873)

### What's working

- Multi-profile `profiles.toml` replaces single `connection.toml`
- `ProfilePicker` TUI widget (Warlock-style list + edit form)
- N/E/D hotkeys now correctly gated on list mode — **fixed in beta.42**
  - In edit mode, N/E/D type as characters into the active field
  - `pub fn is_list_mode(&self) -> bool` added to `ProfilePicker`
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
  login_wizard.rs          — ProfilePicker TUI widget; is_list_mode() added
  runtime.rs               — startup: load profiles, show picker if needed;
                             event loop: handle //setup:connect:* commands
  input_handlers.rs        — handle_wizard_keys(); N/E/D gated on is_list_mode()
  frontend_impl.rs         — renders ProfilePicker overlay
  mod.rs                   — TuiFrontend.login_wizard: Option<ProfilePicker>
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

1. **User re-tests beta.42 on Windows** — verify N/D/E type correctly in password field
2. **Fix paste** (bug above) if still broken after beta.42
3. **Lich subprocess launch** — currently assumes Lich is already running on configured port
4. **Always-show picker** — show picker on every launch (like Warlock), last-used pre-selected;
   add `--character <name>` CLI flag to skip

---

## Files NOT to touch (core game engine)

- src/parser.rs
- src/theme.rs
- src/config.rs
- src/core/ (all files)
