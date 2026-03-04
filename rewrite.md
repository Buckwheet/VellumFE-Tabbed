# VellumFE Rewrite Plan

## Goal

Strip the project down to a single-session GemStone IV terminal frontend.
Remove all multi-session/tabbed infrastructure. Keep the Direct login path
(eAccess auth → character select → game). Add LichProxy support with proper
documentation. Fix the saved-session password re-entry bug.

---

## What Gets Removed

### Files to delete entirely
- `src/session_manager.rs` — multi-session manager
- `src/sessions_config.rs` — sessions.toml load/save
- `src/session_cache.rs` — session cache
- `src/session/mod.rs` — Session struct with unread counters, status enum
- `src/frontend/tui/tab_bar.rs` — tab bar widget
- `src/frontend/tui/session_picker.rs` — session picker TUI
- `src/frontend/tui/session_keys.rs` — per-session key routing
- `src/frontend/tui/login_wizard.rs` — eAccess login wizard (replaced by simpler startup flow)

### Code to remove from remaining files

**`src/main.rs`**
- `SessionManager` construction and population
- `sessions_config` loading
- All `//picker:*` and `//session:*` command routing
- Multi-session startup loop

**`src/frontend/tui/runtime.rs`**
- `app_cores: HashMap<SessionId, AppCore>` → single `app_core: AppCore`
- `spawn_session_network` multi-session logic → single connection spawn
- All `//picker:*` / `//session:*` / `//wizard:*` internal command handlers
- `active_session_id` atomic, `broadcast_next` flag
- Tab switching keybinds (Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1..9)

**`src/frontend/tui/mod.rs`**
- `session_labels` tuple field
- `sessions` Vec field
- Anything referencing `SessionManager` or `TabBar`

**`src/frontend/tui/frontend_impl.rs`**
- `tab_entries` construction
- `TabBar` render call
- Per-session render routing

**`src/frontend/tui/input_handlers.rs`**
- All `//picker:*` command generation
- `ConnectWithPassword` action
- Session switch keybind handlers

**`src/config.rs`**
- `SessionModeConfig` enum (Direct/LichProxy variants) — replace with two simple
  top-level structs in a new `src/connection.rs`

---

## What Gets Kept

- All widget code (`text_window`, `progress_bar`, `compass`, `hand`, `injury_doll`,
  `dashboard`, `active_effects`, `targets`, `players`, `countdown`, etc.)
- Parser (`src/parser.rs`)
- Network layer (`src/network.rs`) — keep `DirectConnection` and `LichConnection`,
  remove multi-session channel plumbing
- Config system (`src/config.rs`) — keep layout, highlight, keybind, theme config
- `AppCore` / `MessageProcessor` — unchanged, just no longer in a HashMap
- Theme, sound, TTS, credentials, migrate — unchanged

---

## What Gets Added / Changed

### 1. Single-session startup

Replace the session picker with a simple startup sequence:

```
On launch:
  1. Read ~/.vellum-fe/connection.toml
  2. If connection_mode = "direct":
       - Read saved account/character
       - If password missing from keychain → show inline password prompt
       - Connect via DirectConnection
  3. If connection_mode = "lich":
       - Read host/port from connection.toml
       - Connect via LichConnection immediately (no auth needed)
  4. If connection.toml missing → show first-run setup screen
```

### 2. Fix: saved-session password re-entry bug

**Root cause**: `credentials::store_password` is called with the account name as the
keyring service key. On Windows, the keyring crate uses the Windows Credential Manager.
The bug is that the password is stored but not retrieved correctly on next launch —
`credentials::get_password` is either not called at startup, or the account key doesn't
match what was stored.

**Fix**:
- In `connection.toml`, store `account` (username) but never store the password in the file
- At startup, call `credentials::get_password(&account)` to retrieve from OS keychain
- If `Ok(pw)` → connect silently
- If `Err(_)` → show a single password input field, store on success
- This means the user only ever types their password once per machine

### 3. connection.toml format

New file at `~/.vellum-fe/connection.toml`:

```toml
# Direct login (eAccess)
[connection]
mode = "direct"
account = "myaccount"
character = "Brashka"
game_code = "GS3"

# OR: LichProxy
[connection]
mode = "lich"
host = "127.0.0.1"
port = 8000
```

### 4. LichProxy documentation (inline in app + README)

See "LichProxy Setup" section below.

---

## LichProxy Setup

Vellum can connect to GemStone IV through
[Lich](https://lichproject.org/) the same way Warlock does —
by connecting to Lich's local proxy port instead of directly to the game servers.

### Prerequisites

- Lich installed and configured (see https://lichproject.org/)
- Lich running with a character logged in

### How Lich proxy works

When Lich is running, it opens a local TCP port (default `8000`) that acts as a
pass-through proxy to the game. Any frontend that connects to that port receives
the same game data stream as if it connected directly. Lich scripts intercept and
inject data on this stream.

Warlock connects to `127.0.0.1:8000` by default. Vellum does the same.

### Steps

1. Start Lich and log in your character as normal (via the Lich launcher or CLI).

2. Find the proxy port. In Lich, run:
   ```
   ;e puts $frontend_port
   ```
   The default is `8000`. If you run multiple characters, each gets a different port
   (8000, 8001, 8002, ...).

3. Edit `~/.vellum-fe/connection.toml`:
   ```toml
   [connection]
   mode = "lich"
   host = "127.0.0.1"
   port = 8000
   ```

4. Launch Vellum. It will connect to Lich's proxy port immediately — no login prompt.

### Notes

- Vellum does **not** need your SGE account credentials when using LichProxy mode.
- Lich must already be running and logged in before you start Vellum.
- If Lich is on a different machine (e.g., a VPS), change `host` to that machine's IP.
- The proxy port is a plain TCP socket — no TLS, no auth handshake. Vellum sends
  nothing on connect; it just starts reading the game stream.

---

## Implementation Order

1. **Delete** the files listed above
2. **Rewrite `src/main.rs`** — single AppCore, single connection, startup flow
3. **Rewrite `src/frontend/tui/runtime.rs`** — remove session map, remove picker/wizard
   command handlers, simplify event loop to single session
4. **Rewrite `src/frontend/tui/mod.rs`** — remove session_labels, sessions Vec, TabBar
5. **Rewrite `src/frontend/tui/frontend_impl.rs`** — remove tab bar render, single-session render
6. **Rewrite `src/frontend/tui/input_handlers.rs`** — remove picker/session commands
7. **Add `src/connection.rs`** — `ConnectionConfig` enum (Direct/Lich), load/save
   `connection.toml`, password retrieval logic
8. **Add first-run setup screen** — minimal TUI form shown when `connection.toml` missing
9. **Fix password retrieval** — call `credentials::get_password` at startup, only prompt
   if missing
10. **Update README** with LichProxy setup steps (copy from above)
11. **Rename binary** from `vellum-fe-tabbed` to `vellum-fe` in `Cargo.toml`
12. `cargo build` → test → tag `v0.3.0`

---

## Out of Scope for This Rewrite

- No new widgets
- No new config options beyond `connection.toml`
- No GUI frontend
- No changes to parser, theme, highlight, or layout systems
- No test additions
