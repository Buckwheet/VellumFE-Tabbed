# KIRO.md — Steering Document for VellumFE-Tabbed

**Resume phrase**: "Read KIRO.md from VellumFE-Tabbed and continue"

---

## Project

VellumFE — GemStone IV terminal frontend with Warlock-style login manager.
Rust + Ratatui. Binary: `vellum-fe-tabbed`.
Repo: https://github.com/Buckwheet/VellumFE-Tabbed
Local: `~/VellumFE-Tabbed/`

**Testing machine**: User tests on a **separate Windows machine**.
Kiro can read files via WSL mount at `/mnt/c/`.
- Log: `/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/vellum-fe.log`
- Config dir: `C:\Users\rpgfi\.vellum-fe\` (WSL path: `/mnt/c/Users/rpgfi/.vellum-fe/`)

---

## Build / Deploy / Tag

```bash
# Build
cd ~/VellumFE-Tabbed && ~/.cargo/bin/cargo build --release --target x86_64-pc-windows-gnu 2>&1 | tail -3

# Copy
cp ~/VellumFE-Tabbed/target/x86_64-pc-windows-gnu/release/vellum-fe-tabbed.exe \
   "/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/vellum-fe-tabbed.exe"

# Commit + tag (replace N with next beta number)
cd ~/VellumFE-Tabbed && ~/.cargo/bin/cargo fmt && git add -A && \
  git commit -m "<message>" && git push && \
  git tag v0.1.0-beta.N && git push origin v0.1.0-beta.N
```

---

## Current State — beta.53 (HEAD: 5914ce0, tag: v0.1.0-beta.53)

Both launch-crash issues are **RESOLVED**.

### Issue 1: Ratatui buffer panic (FIXED — beta.49–51)

Windows Terminal briefly resizes new tabs to `47x1`. This caused `terminal.draw()` to panic.

- **beta.49**: Removed initial `frontend.render()` call in `runtime.rs`
- **beta.50**: Added size guard in `frontend_impl.rs::render()` — skip if `w < 20 || h < 3`
- **beta.51**: Call `self.terminal.autoresize()` before the size guard so the cached size
  is fresh before `terminal.draw()` runs internally

Current guard (`src/frontend/tui/frontend_impl.rs`):
```rust
let _ = self.terminal.autoresize();
let (w, h) = self.size();
if w < 20 || h < 3 {
    return Ok(());
}
```

### Issue 2: Window shrinks / moves on second instance launch (FIXED — beta.52–53)

Root cause: `window.toml` stored `{ x:50, y:50, width:0, height:0 }` as default.
`SetWindowPos` with `width=0, height=0` collapsed the WT window.
Position was never saved because closing a WT tab kills the process before exit cleanup runs.

- **beta.52** (`src/window_position/windows.rs`): Use `SWP_NOSIZE` flag when saved dims are zero
- **beta.53** (`src/frontend/tui/runtime.rs`):
  - Skip `set_position` entirely when `width == 0 || height == 0`
  - Save position every 30 seconds in the event loop (survives tab-close kills)

---

## Architecture — Key Files

```
src/window_position/
  windows.rs          — Win32 SetWindowPos; SWP_NOSIZE when dims are zero
  storage.rs          — load/save window.toml (~/.vellum-fe/profiles/default/window.toml)
  mod.rs              — WindowRect, WindowPositioner trait, create_positioner()

src/frontend/tui/
  runtime.rs          — startup: skip set_position if dims zero; 30s periodic position save
  frontend_impl.rs    — autoresize() + size guard before terminal.draw()
  login_wizard.rs     — ProfilePicker TUI widget
  input_handlers.rs   — handle_wizard_keys(); N/E/D gated on list mode
  mod.rs              — TuiFrontend struct

src/connection.rs     — ProfileStore, Profile struct, profiles.toml
src/credentials.rs    — OS keychain (store/get/delete password)
src/network.rs        — eaccess module, DirectConnection, LichConnection
src/config.rs         — Config struct (DO NOT TOUCH)
src/core/            — App core, game state (DO NOT TOUCH)
src/parser.rs        — GS XML parser (DO NOT TOUCH)
```

---

## Login / Profile System

### profiles.toml format (`~/.vellum-fe/profiles.toml`)

```toml
[[profiles]]
name = "Brashka (Prime)"
account = "myaccount"
character = "Brashka"
game_code = "GS3"
use_lich = false
```

### GAMES list (`login_wizard.rs`)

```rust
pub const GAMES: &[(&str, &str)] = &[
    ("GS3", "GemStone IV (Prime)"),
    ("GSX", "GemStone IV (Platinum)"),
    ("GSF", "GemStone IV (Shattered)"),
    ("DR", "DragonRealms"),
];
```

### Internal connection commands

- `//setup:connect:direct:<account>:<game_code>:<character>` — spawn DirectConnection
- `//setup:connect:lich:<host>:<port>` — spawn LichConnection

---

## Known Bugs / Next Steps

1. **Paste in edit form** — paste events arrive as rapid individual `Char` events.
   Fix: add `paste_str()` to `ProfilePicker`, handle `KeyCode::Paste(text)` in `handle_wizard_keys`.

2. **Lich subprocess launch** — currently assumes Lich is already running on configured port.

3. **`--character <name>` CLI flag** — skip picker and connect directly to named profile.

4. **Dead code cleanup** — orphaned multi-session files still present:
   ```
   src/session/
   src/session_manager.rs
   src/sessions_config.rs
   src/frontend/tui/session_picker.rs
   src/frontend/tui/session_keys.rs
   ```
   Verify with `grep -rn` before deleting.
