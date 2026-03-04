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
- Lich is optional — users who don't want it just use Direct

---

## Current State (beta.41)

### Architecture

```
src/connection.rs          — ProfileStore, Profile struct, profiles.toml
src/credentials.rs         — OS keychain (store/get/delete password)
src/network.rs             — eaccess module (authenticate, fetch_characters),
                             DirectConnection, LichConnection
src/frontend/tui/
  login_wizard.rs          — ProfilePicker TUI widget (Warlock-style)
  runtime.rs               — startup: load profiles, show picker if needed;
                             event loop: handle //setup:connect:* commands
  input_handlers.rs        — handle_wizard_keys() routes to ProfilePicker
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

[[profiles]]
name = "Brashka via Lich"
account = "myaccount"
character = "Brashka"
game_code = "GS3"
use_lich = true
lich_host = "127.0.0.1"
lich_port = 8000
```

### ProfilePicker flow

1. Launch with no profiles → picker shows "No profiles saved. Press N to add one."
2. Launch with profiles → list of saved characters, arrow keys to select, Enter to connect
3. N = new profile form, E = edit selected, D = delete selected
4. Edit form: Name → Account → Password → Game (cycle) → Character (Enter fetches from eAccess) → Use Lich → (if Lich) Host/Port
5. On Connect: saves profiles.toml, stores password in keychain, emits `//setup:connect:direct:` or `//setup:connect:lich:` command
6. Runtime handles those commands and spawns the appropriate connection

### Connection commands (internal)

- `//setup:connect:direct:<account>:<game_code>:<character>` — spawn DirectConnection
- `//setup:connect:lich:<host>:<port>` — spawn LichConnection (Lich must already be running)

### Key bindings in picker (list mode)

- Up/Down — move selection
- Enter — connect with selected profile
- N — new profile
- E — edit selected
- D — delete selected
- Esc — quit app

### Key bindings in picker (edit mode)

- Tab/Enter — advance to next field
- Up/Down — move field / cycle game selector / toggle Lich
- Backspace — delete char
- Esc — back to list
- Enter on Character field (empty) — fetch characters from eAccess

---

## Known Bugs / Next Session Fixes

1. **Paste in edit form is broken** — pasting into a field (e.g. password) erases previously
   typed content in earlier fields and scatters the pasted text into the wrong field.
   Root cause: paste events are likely being delivered as individual key chars and not
   scoped to the currently focused field. Fix: intercept paste events in `handle_wizard_keys`
   and route them only to the active field via a new `paste_str(&mut self, s: &str)` method
   on `ProfilePicker`.

2. **Paste support needed** — users should be able to paste into any field in the edit form,
   especially the password field. The picker needs to handle `KeyCode::Paste(text)` /
   crossterm paste events and insert the text into the active field only.

---

## What's NOT done yet (next session)

1. **Lich subprocess launch** — currently assumes Lich is already running on the configured port.
   Future: when `use_lich = true`, spawn `lich.rb --login <account> --game <game_code> --char <char>`
   as a subprocess, wait for it to open its proxy port, then connect.

2. **Profile picker shown on every launch** — currently only shown when no profiles exist.
   Consider: always show picker on launch (like Warlock), with last-used profile pre-selected.
   Add a `--character <name>` CLI flag to skip the picker.

3. **Windows testing** — build and test on Windows machine.

4. **Tag v0.1.0-beta.41** after confirmed working.

---

## Files NOT to touch (core game engine — working, don't break)

- src/parser.rs (4475 lines)
- src/theme.rs (4250 lines)
- src/config.rs (6766 lines)
- src/core/ (all files)
- src/frontend/tui/ (all files except the 5 listed above)
