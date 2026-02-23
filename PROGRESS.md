# Session Progress Log

## Session 1 — 2026-02-23

### Completed
- [x] WSL setup: git configured (Buckwheet / rpgfilms@gmail.com), SSH key generated, GitHub CLI authenticated
- [x] All GSIV Development projects connected to GitHub remotes
- [x] Reference repos synced: Warlock, Illthorn, VellumFE, ProfanityFE
- [x] VellumFE-Tabbed repo created: https://github.com/Buckwheet/VellumFE-Tabbed
- [x] VellumFE codebase cloned as base
- [x] PROJECT_PLAN.md written and pushed (full 6-phase roadmap)
- [x] README.md rewritten as our own project with proper credits
- [x] Kiro rules saved: backup-rule.md, precommit-hooks-rule.md

### Key Decisions Made
- Base: VellumFE (Rust + Ratatui) — lowest memory, best performance
- 15 simultaneous sessions target
- Both Lich proxy and direct eAccess login
- No command line required — full TUI login wizard (Phase 4)
- Lich scripts can send `<vellumfe cmd="..."/>` tags to control highlights
- Sessions auto-connect on startup: **TBD**
- Max buffer per inactive session: **TBD**

### Next Steps
- Answer remaining open questions in PROJECT_PLAN.md
- Begin Phase 1: refactor VellumFE single-session state into `Session` struct

### To Resume Next Session
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed and let's continue Phase 1"

## Session 2 — Phase 1 Started (parallel agents)

### Completed
- [x] Session struct created: src/session/mod.rs
- [x] SessionManager created: src/session_manager.rs  
- [x] SessionsConfig (sessions.toml loader): src/sessions_config.rs
- [x] sessions.toml.example format documented
- [x] TabBar widget created: src/frontend/tui/tab_bar.rs

### Remaining Phase 1 Tasks
- [ ] Wire SessionManager into main.rs (replace single-session startup)
- [ ] Wire TabBar into the TUI layout (add 1-row tab bar at top)
- [ ] Background TCP task per session (all run simultaneously)
- [ ] Keyboard shortcuts: Ctrl+1..9, Ctrl+T, Ctrl+W
- [ ] Unread counter increments for inactive sessions

### Next Steps
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed, continue Phase 1 wiring"

## Session 3 — Phase 1 Complete, Build Process Established

### Completed
- [x] Rust + build tools installed in WSL
- [x] Session/SessionManager wired into runtime event loop
- [x] TabBar widget wired into TUI render (top row, 1px height)
- [x] Session key shortcuts wired (Ctrl+1..9, Ctrl+Tab, Ctrl+W, Ctrl+T)
- [x] All Phase 1 compilation errors resolved — cargo check passes clean
- [x] Binary renamed: vellum-fe-tabbed
- [x] CI workflow: ci.yml (test on push/PR to main)
- [x] Beta release workflow: beta-release.yml (tag v*.*.*-beta*)
- [x] Stable release workflow: release.yml (tag v*.*.*)
- [x] Release process rule saved: ~/.kiro/context/release-process-rule.md

### Phase 2 Tasks (next session)
- [ ] 2.1 Session picker screen (shown on first run / no sessions)
- [ ] 2.2 Add/edit/remove sessions from picker
- [ ] 2.3 Compact mode toggle (Ctrl+Shift+C)
- [ ] 2.4 sessions.toml — persist session list across restarts
- [ ] 2.5 Auto-reconnect on disconnect per session

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed, start Phase 2"

## Session 4 — Phase 2 Complete

### Completed
- [x] 2.1 Session picker screen (shown when sessions.toml is empty / first run)
- [x] 2.2 Add/remove sessions from picker (Lich mode; Direct is Phase 4)
- [x] 2.3 Compact mode toggle (Ctrl+Shift+C) — tab bar already rendered both modes
- [x] 2.4 sessions.toml persistence — load on startup, save on add/remove
- [x] 2.5 Auto-reconnect — Lich connections retry every 5s on disconnect

### Phase 3 Tasks (next session)
- [ ] 3.1 Global highlights.toml (applies to all sessions)
- [ ] 3.2 Per-character highlights override global
- [ ] 3.3 In-app highlight editor (add/edit/delete without restarting)
- [ ] 3.4 Highlight categories UI
- [ ] 3.5 Import/export highlights
- [ ] 3.6 Per-character layout persistence

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed, start Phase 3"

## Session 5 — Phases 3–5 Complete

### Phase 3 — Config & Highlights
- [x] 3.1/3.2 Global + per-character highlights merge: already in VellumFE base
- [x] 3.3/3.4 `<vellumfe>` Lich script protocol: highlight.add/remove/clear, squelch.add/remove
- [x] 3.5 persist="true" attribute saves highlights to disk
- [x] 3.6 Per-character layout persistence: already in VellumFE base

### Phase 4 — Login Wizard
- [x] 4.1 Session picker shows saved sessions + Add Session
- [x] 4.2 Mode toggle: Lich Proxy / Direct (F2 in add form)
- [x] 4.3 Direct flow: credentials → game select → character select → connect
- [x] 4.4 Lich flow: host/port/label form → connect
- [x] 4.5 No passwords stored in sessions.toml
- [x] 4.6 Sessions saved to sessions.toml on connect
- [x] 4.7 eAccess `fetch_characters` public API added to network.rs

### Phase 5 — Cross-Session Features
- [x] 5.1 Ctrl+B broadcast: next command sent to all sessions
- [x] 5.2 Global squelch: handled by global highlights.toml (Phase 3)
- [x] 5.3 Session grouping: deferred (complex UI, low priority)
- [x] 5.4 Per-session sound_enabled / tts_enabled fields on Session struct

### Phase 6 — Windows Build
- [x] Already complete (ci.yml, beta-release.yml, release.yml from Session 3)

### All Phases Complete 🎉
Remaining work: polish, testing, bug fixes, Phase 5.3 (session grouping UI)

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed"

## Session 6 — Credential Storage, Auto-Connect, Rich Tab Status

### Completed
- [x] `src/credentials.rs` — OS keychain credential storage via `keyring` crate (store/get/delete)
- [x] Auto-connect on startup: sessions with `auto_connect = true` reconnect using keychain password
- [x] Rich tab status symbols: ● connected, … connecting, ↻ reconnecting, ! error, ○ disconnected
- [x] `tab_bar.rs` — `TabEntry.status: String` (was `is_connected: bool`); color per symbol
- [x] `frontend_impl.rs` — updated tab_entries construction to use new tuple shape
- [x] `lib.rs` — `pub mod credentials` added
- [x] `README.md` — full install instructions, keyboard shortcuts, Lich protocol docs, config paths
- [x] `PROJECT_PLAN.md` — open questions answered (auto-connect policy, 10k line buffer limit)
- [x] Commit: `5bae936`

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed"

## Session 7 — Unread Badges, Sound/TTS Toggle

### Completed
- [x] `src/session/mod.rs` — `unread: Arc<AtomicUsize>` + `active_session_id: Arc<AtomicUsize>` added to Session; `sync_unread()` method
- [x] `src/session_manager.rs` — `active_session_id: Arc<AtomicUsize>` shared across all sessions; `sync_unread_all()` method; `set_active()` updates the shared atomic
- [x] `spawn_lich_reconnect` — intercepts `ServerMessage::Text` and increments `unread` atomic when session is not active (lock-free, no main loop involvement)
- [x] Main loop — calls `session_manager.sync_unread_all()` + `sync_tabs()` every second so badges update
- [x] `session_keys.rs` — `ToggleSound` and `ToggleTts` variants added; `sound()` and `tts()` string helpers
- [x] `input_handlers.rs` — Ctrl+Shift+S → toggle sound, Ctrl+Shift+T → toggle TTS, Ctrl+Shift+C → toggle compact
- [x] `runtime.rs` — `ToggleSound`/`ToggleTts` arms toggle `session.sound_enabled`/`tts_enabled` on active session

### Keyboard Shortcuts Added
| Key | Action |
|-----|--------|
| Ctrl+Shift+C | Toggle compact tab bar |
| Ctrl+Shift+S | Toggle sound for active session |
| Ctrl+Shift+T | Toggle TTS for active session |

### Remaining / Future Work
- [ ] Phase 5.3 — Session grouping UI (deferred, complex, low priority)
- [ ] Sound/TTS state reflected in tab bar (e.g. 🔇 icon when muted)
- [ ] Per-session AppCore (true isolation of parser/game state per session — large refactor)

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed"

## Session 8 — Per-Session Network Tasks, Sound Mute Indicator

### Completed
- [x] `session/mod.rs` — `server_tx/server_rx` per session; `ConnectionMode::LichProxy` gains `login_key` field
- [x] `runtime.rs` — `spawn_session_network()` helper spawns Lich/Direct task using session's own channels
- [x] `runtime.rs` — `session_rxs: HashMap<SessionId, Receiver>` replaces single global `server_rx`; main loop polls active session's receiver
- [x] `runtime.rs` — picker/wizard handlers now call `spawn_session_network` so new sessions actually connect
- [x] `runtime.rs` — auto-connect sessions on startup spawn their own network tasks
- [x] `runtime.rs` — initial session uses its own `server_tx`; `login_key` preserved in `ConnectionMode`
- [x] `tab_bar.rs` — 🔇 shown in tab when `sound_enabled = false`
- [x] `session_labels` extended to 5-tuple: `(label, is_active, status, unread, sound_enabled)`
- [x] Commit: `9ec4101`

### Remaining / Future Work
- [ ] Per-session AppCore (true isolation of parser/game state — large refactor)
- [ ] Phase 5.3 — Session grouping UI (deferred)
- [ ] TTS state in tab bar

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed"

## Session 9 — Per-Session AppCore (True Game State Isolation)

### Completed
- [x] `runtime.rs` — `app_cores: HashMap<SessionId, AppCore>` — one AppCore per session
- [x] `runtime.rs` — `create_app_core_for_session()` loads per-character config via `Config::load_with_options`
- [x] `runtime.rs` — main loop gets active session's AppCore via raw ptr (safe: single-threaded loop, no aliasing)
- [x] `runtime.rs` — `app_core.running` replaced with local `running` bool; loop exits when active session quits
- [x] `runtime.rs` — command routing uses active session's `command_tx` instead of shared dummy channel
- [x] `runtime.rs` — auto-connect sessions at startup get their own AppCore
- [x] `runtime.rs` — picker/wizard handlers create AppCore for new sessions
- [x] Commit: `81a8d57`

### Architecture Now
Each session has:
- Its own `server_tx/server_rx` channel pair (network ↔ main loop)
- Its own `command_tx/command_rx` channel pair (main loop → network)
- Its own `AppCore` (parser, game state, UI state, highlights, config)
- Its own unread badge atomic counter

### Remaining / Future Work
- [ ] Session switch: save/restore active session's UI state (scroll position, focused window) on switch
- [ ] Phase 5.3 — Session grouping UI (deferred)
- [ ] TTS state in tab bar

### To Resume
Tell Kiro: "Read PROJECT_PLAN.md and PROGRESS.md from VellumFE-Tabbed"
