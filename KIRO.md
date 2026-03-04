# KIRO.md — Steering Document for VellumFE

This file is the first thing Kiro reads each session. It contains everything needed to
resume work immediately without re-reading source files.

---

## Project

Single-session GemStone IV terminal frontend. Rust + Ratatui.
Binary: `vellum-fe-tabbed` (rename to `vellum-fe` pending).
Repo: https://github.com/Buckwheet/VellumFE-Tabbed
Local: `~/VellumFE-Tabbed/`

**Rule**: Before modifying any file, create a `.bak` backup first.

**IMPORTANT — Testing machine**: User builds and tests on a **separate Windows machine**.
Kiro CAN read files from it via WSL mount at `/mnt/c/`. Always check there first before
asking the user to paste files.
- Log file: `/mnt/c/Users/rpgfi/Documents/GSIV Development/VellumFE-Tabbed/vellum-fe.log`
- Config dir: `C:\Users\rpgfi\.vellum-fe\` (NOT accessible via /mnt/c — different machine)

---

## Current State (beta.39)

### What was done (beta.39 — `59f03da`)

Full single-session rewrite. All multi-session/tabbed infrastructure removed:

**Removed modules:**
- `src/session_manager.rs` — multi-session manager
- `src/sessions_config.rs` — sessions.toml
- `src/session/mod.rs` — Session struct
- `src/frontend/tui/tab_bar.rs` — tab bar widget (still compiled, now dead code)
- `src/frontend/tui/session_picker.rs` — session picker TUI (still compiled, dead code)
- `src/frontend/tui/session_keys.rs` — session key routing (removed from mod.rs)
- `src/frontend/tui/login_wizard.rs` — eAccess wizard (still compiled, dead code)

**Added:**
- `src/connection.rs` — `ConnectionConfig` enum (Lich/Direct), backed by `~/.vellum-fe/connection.toml`

**Changed:**
- `src/main.rs` — stripped all session_manager/sessions_config references
- `src/frontend/tui/runtime.rs` — single AppCore, single server_rx, no session switching
- `src/frontend/tui/frontend_impl.rs` — tab bar render removed, full-screen content area
- `src/frontend/tui/input_handlers.rs` — all `//picker:*` / `//session:*` routing removed
- `src/frontend/tui/mod.rs` — `session_picker`/`login_wizard` fields replaced with
  `show_setup_screen: bool` and `show_password_prompt: bool`
- `src/lib.rs` — removed session/session_manager/sessions_config, added connection

**Password fix:**
- `runtime.rs` reads `connection.toml` at startup
- For Direct mode: calls `credentials::get_password(&account)` from OS keychain
- If password missing → sets `show_password_prompt = true` (currently just dismisses on keypress)
- User only types password once per machine

**Connection resolution order (runtime.rs):**
1. CLI `--direct` flags
2. CLI `--character` / `--key` (Lich proxy)
3. `~/.vellum-fe/connection.toml`
4. None → `show_setup_screen = true`

---

## connection.toml format

```toml
# Lich proxy
[connection]
mode = "lich"
host = "127.0.0.1"
port = 8000

# OR Direct login
[connection]
mode = "direct"
account = "myaccount"
character = "Brashka"
game_code = "GS3"
```

File location: `~/.vellum-fe/connection.toml`
Password is NOT stored here — it lives in the OS keychain (Windows Credential Manager).

---

## LichProxy Setup

1. Start Lich and log in your character normally.
2. Find the proxy port: in Lich run `;e puts $frontend_port` (default: 8000).
3. Create `~/.vellum-fe/connection.toml`:
   ```toml
   [connection]
   mode = "lich"
   host = "127.0.0.1"
   port = 8000
   ```
4. Launch Vellum — it connects immediately, no login prompt.

---

## Next Steps

1. User downloads beta.39, tests that it connects via Lich proxy
2. Confirm password-from-keychain works for Direct mode (no re-entry prompt)
3. If setup screen / password prompt UX needs polish, implement proper TUI forms
4. Rename binary from `vellum-fe-tabbed` → `vellum-fe` in Cargo.toml
5. Remove debug scaffolding from `network.rs` (eaccess_raw_debug, raw_debug, fetch_characters debug log)
6. Tag `v0.2.0` after confirmed working

---

## Key Files

```
src/connection.rs                    ConnectionConfig, load/save connection.toml
src/frontend/tui/runtime.rs          Main event loop, single-session, connection resolution
src/frontend/tui/mod.rs              TuiFrontend struct (show_setup_screen, show_password_prompt)
src/frontend/tui/frontend_impl.rs    render() — full-screen, no tab bar
src/frontend/tui/input_handlers.rs   handle_normal_mode_keys — no picker/session routing
src/credentials.rs                   get_password / store_password (OS keychain)
src/network.rs                       LichConnection, DirectConnection, ServerMessage
src/config.rs                        Config::load_with_options, layout, highlights, themes
```

---

## Architecture

Single session owns:
- `server_rx: mpsc::UnboundedReceiver<ServerMessage>` — network → main loop
- `command_tx: mpsc::UnboundedSender<String>` — main loop → network
- `app_core: AppCore` — parser, game state, UI, config (single instance, not a HashMap)

---

## Config Paths

```
~/.vellum-fe/connection.toml           connection mode (new)
~/.vellum-fe/<character>/config.toml   per-character config
~/.vellum-fe/default/config.toml       global config
~/.vellum-fe/layouts/<name>.toml       layout files
```

---

## How to Resume

Tell Kiro: **"Read KIRO.md from VellumFE-Tabbed and continue"**
