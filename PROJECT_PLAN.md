# VellumFE-Tabbed: Project Plan

## Vision

A high-performance, multi-session GemStone IV frontend built on VellumFE's Rust/Ratatui
foundation. Users can run up to 15 simultaneous sessions in a single window, switching
between them via a session picker or tabbed layout. Zero Electron, zero JVM — compiled
Rust binary only.

---

## Technology Decision

| Client      | Stack                  | RAM (est.) | Multi-session | Config persistence |
|-------------|------------------------|------------|---------------|--------------------|
| VellumFE    | Rust + Ratatui (TUI)   | ~10-30MB   | ❌ (1 session) | ✅ TOML per-char   |
| ProfanityFE | Ruby + Curses          | ~20MB      | ❌             | ✅ basic           |
| Illthorn    | Electron + TypeScript  | ~300MB+    | ✅ built-in    | ✅ electron-store  |
| Warlock     | Kotlin + Compose       | ~200MB+    | ❌             | ✅ Room DB         |

**Base: VellumFE** — lowest memory footprint by far. We port Illthorn's multi-session
architecture concept into Rust. At 15 sessions, VellumFE-Tabbed should use ~150-450MB
total vs ~4.5GB+ for Electron-based alternatives.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                  VellumFE-Tabbed                     │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │           Session Manager                    │   │
│  │  [Session 1] [Session 2] ... [Session 15]    │   │
│  │  Tab bar or compact session picker           │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │         Active Session View                  │   │
│  │  Widgets | Text Windows | CLI | Vitals       │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  Per-session background tasks (all 15 run always):  │
│  - TCP connection (Lich proxy or direct eAccess)    │
│  - XML parser                                        │
│  - Highlight engine                                  │
│  - Sound/TTS engine                                  │
└─────────────────────────────────────────────────────┘
```

---

## Key Features (from all reference clients)

### From VellumFE (keep all existing)
- Ratatui TUI rendering
- Widget system: progress bars, compass, hands, injury doll, countdowns, active effects,
  indicators, inventory, targets, spells, room window, tabbed text windows
- TOML config per character profile
- Regex highlights with Aho-Corasick fast matching
- Sound alerts on pattern match
- TTS support
- Direct eAccess login (no Lich required)
- Lich proxy login
- Layout editor (F2)
- Highlight browser (F3)
- Themes / full color customization
- Squelch patterns (hide lines)
- Stream routing (redirect text to specific windows)
- Session cache (quickbars, spells survive reconnect)
- 1,003 existing tests

### From Illthorn (port concept to Rust)
- Multi-session support (up to 15 simultaneous)
- Session picker UI
- Per-session isolated state (parser, highlights, UI, history)
- Session tab bar with unread indicators
- Focus switching between sessions

### From Warlock (port concept)
- Per-character settings saved to disk (already in VellumFE via TOML)
- Window layout persistence per character

### New features
- **Compact mode**: collapse all sessions into a minimal tab bar
- **Session picker screen**: launch screen to add/remove/connect sessions
- **Unread message badges** on inactive session tabs
- **Global highlights** (apply to all sessions) + per-session overrides
- **Cross-session commands** (send same command to multiple sessions)

---

## Layout Modes

### Tabbed Mode (default)
```
┌─[Buckwheet]──[Altchar]──[Mule]──[+]──────────────────┐
│                                                        │
│  [Main text window]                    [Vitals]        │
│                                        [Compass]       │
│                                        [Hands]         │
│  [Thoughts/Streams tabs]               [Spells]        │
│                                                        │
│  > _                                                   │
└────────────────────────────────────────────────────────┘
```

### Compact Mode
```
┌─[B●]─[A]─[M●]─[+]────────────────────────────────────┐
│  (● = unread activity)                                 │
│  [Full session view for active session]                │
└────────────────────────────────────────────────────────┘
```

### Session Picker Screen
```
┌─ Sessions ────────────────────────────────────────────┐
│  [Buckwheet]  Prime  Connected  Port 8000             │
│  [Altchar]    Prime  Connected  Port 8001             │
│  [Mule]       Prime  Idle       Port 8002             │
│  [+ Add Session]                                      │
└────────────────────────────────────────────────────────┘
```

---

## Config Structure (per session)

```
~/.config/vellum-fe-tabbed/
├── global/
│   ├── highlights.toml       # Global highlights (all sessions)
│   ├── keybinds.toml
│   └── themes/
├── sessions.toml             # Session list (name, port, login method)
└── characters/
    └── <CharacterName>/
        ├── config.toml       # Connection settings
        ├── highlights.toml   # Per-character highlight overrides
        ├── layout.toml       # Widget layout
        ├── colors.toml
        ├── keybinds.toml
        └── session_cache.toml
```

---

## Implementation Phases

### Phase 1 — Foundation & Multi-Session Core
**Goal**: Multiple sessions running simultaneously, tab switching works.

- [ ] 1.1 Fork VellumFE codebase into VellumFE-Tabbed
- [ ] 1.2 Refactor single-session state into `Session` struct (isolate all per-session data)
- [ ] 1.3 Create `SessionManager` — owns Vec<Session>, handles add/remove/switch
- [ ] 1.4 Add tab bar widget to TUI (shows session names, active indicator, unread badge)
- [ ] 1.5 Keyboard shortcuts: Ctrl+1..9 to switch sessions, Ctrl+T new session, Ctrl+W close
- [ ] 1.6 All 15 sessions run background TCP + parser tasks simultaneously
- [ ] 1.7 Only active session renders to terminal (others buffer in memory)
- [ ] 1.8 Unread message counter increments for inactive sessions

### Phase 2 — Session Picker & Compact Mode
**Goal**: Users can manage sessions visually and toggle compact layout.

- [ ] 2.1 Session picker screen (launch screen when no sessions active)
- [ ] 2.2 Add/edit/remove session configs from picker
- [ ] 2.3 Compact mode toggle (Ctrl+Shift+C)
- [ ] 2.4 Compact tab bar (minimal height, just names + unread dots)
- [ ] 2.5 sessions.toml — persist session list across restarts
- [ ] 2.6 Auto-reconnect on disconnect per session

### Phase 3 — Config & Highlights Polish
**Goal**: Full per-character config persistence, global + per-session highlights.

- [ ] 3.1 Global highlights.toml (applies to all sessions)
- [ ] 3.2 Per-character highlights override global
- [ ] 3.3 In-app highlight editor (add/edit/delete without restarting)
- [ ] 3.4 Highlight categories UI (group by Combat, Players, Squelch, etc.)
- [ ] 3.5 Import/export highlights (share between characters)
- [ ] 3.6 Per-character layout persistence (save widget positions per character)

### Phase 4 — Login Wizard (no command line required)
**Goal**: Fully guided TUI login wizard. Users never need to touch the command line.

Inspired by Warlock's SGE wizard flow: account → game → character, all navigable
with arrow keys and Enter. No flags, no config file editing to get started.

#### Login Wizard Flow (Direct eAccess)
```
┌─ Add Session ─────────────────────────────────────────┐
│                                                        │
│  Login Method:                                         │
│  > [Direct (eAccess)]  [ Lich Proxy ]                 │
│                                                        │
│  Account:    [________________]                        │
│  Password:   [****************]                        │
│                                                        │
│  [ Connect → ]                          [ Cancel ]    │
└────────────────────────────────────────────────────────┘
         ↓ (authenticates, fetches game list)
┌─ Select Game ─────────────────────────────────────────┐
│  > GemStone IV (Prime)                                 │
│    GemStone IV (Platinum)                              │
│    GemStone IV (Shattered)                             │
│                                    [ ← Back ]         │
└────────────────────────────────────────────────────────┘
         ↓ (fetches character list)
┌─ Select Character ────────────────────────────────────┐
│  > Buckwheet                                           │
│    Altchar                                             │
│    Mule                                                │
│                                    [ ← Back ]         │
└────────────────────────────────────────────────────────┘
         ↓ (connects, saves session to sessions.toml)
┌─[Buckwheet]──────────────────────────────────────────┐
│  [Game view]                                          │
└───────────────────────────────────────────────────────┘
```

#### Login Wizard Flow (Lich Proxy)
```
┌─ Add Session ─────────────────────────────────────────┐
│  Login Method:                                         │
│  [ Direct (eAccess) ]  > [Lich Proxy]                 │
│                                                        │
│  Host:   [localhost_______]                            │
│  Port:   [8000____________]                            │
│  Label:  [Buckwheet_______]  (display name for tab)   │
│                                                        │
│  [ Connect → ]                          [ Cancel ]    │
└────────────────────────────────────────────────────────┘
```

#### Implementation Tasks
- [ ] 4.1 Launch screen: show saved sessions + "Add Session" button (shown on first run or when no sessions exist)
- [ ] 4.2 Login method selector (Direct / Lich)
- [ ] 4.3 Direct flow: credentials form → SGE auth → game list → character list → connect
- [ ] 4.4 Lich flow: host/port/label form → connect
- [ ] 4.5 Credential storage: save account credentials encrypted to disk (reuse on next launch)
- [ ] 4.6 Saved sessions: remember all sessions in `sessions.toml`, auto-offer reconnect on startup
- [ ] 4.7 Connection status indicator per tab (connecting / connected / disconnected / error)
- [ ] 4.8 Error screen with back navigation (mirrors Warlock's SgeErrorView)

### Phase 5 — Cross-Session Features
**Goal**: Power user features for multi-boxing.

- [ ] 5.1 Broadcast command to selected sessions (Ctrl+B)
- [ ] 5.2 Global squelch patterns (apply to all sessions)
- [ ] 5.3 Session grouping (group characters together)
- [ ] 5.4 Per-session sound/TTS enable/disable

### Phase 6 — Windows Build & Distribution
**Goal**: Installable Windows binary.

- [ ] 6.1 Windows cross-compile from WSL (cargo build --target x86_64-pc-windows-gnu)
- [ ] 6.2 GitHub Actions CI: build + test on push
- [ ] 6.3 GitHub Releases: attach .exe on tag
- [ ] 6.4 README with install instructions

---

## Performance Targets

| Metric                        | Target         |
|-------------------------------|----------------|
| Memory per idle session       | < 30MB         |
| Memory for 15 sessions        | < 500MB        |
| Input-to-screen latency       | < 16ms         |
| XML parse throughput          | > 10,000 msg/s |
| Startup time (15 sessions)    | < 3 seconds    |

---

## Reference Repos

| Repo | Path | Purpose |
|------|------|---------|
| VellumFE | `GSIV Development/VellumFE` | Base codebase |
| Illthorn | `GSIV Development/Illthorn` | Multi-session architecture reference |
| Warlock | `GSIV Development/Warlock` | Login flow + settings persistence reference |
| ProfanityFE | `GSIV Development/ProfanityFE` | Lightweight TUI reference |

---

## Open Questions

- [x] Should sessions auto-connect on startup, or require manual connect from picker?
  **Decision: Both** — sessions with `auto_connect = true` in sessions.toml reconnect automatically. Others require manual connect from the picker. Default for new sessions is `auto_connect = false`.
- [x] Max buffer size per inactive session (memory vs. scroll history tradeoff)?
  **Decision: 10,000 lines per session** — matches VellumFE base default. Inactive sessions buffer in memory; oldest lines are dropped when the limit is reached. Configurable via `max_scroll_lines` in config.toml.

---

## Lich Script → FE Command Protocol

**Decision: Yes** — Lich scripts can send commands to the FE via the proxy stream.

### Protocol Design

Scripts send a custom XML tag over the Lich connection:

```xml
<vellumfe cmd="highlight.add" pattern="Buckwheet" fg="#ff00ff" bold="true" category="Friends" fast_parse="true"/>
<vellumfe cmd="highlight.remove" pattern="Buckwheet"/>
<vellumfe cmd="highlight.clear" category="Friends"/>
<vellumfe cmd="squelch.add" pattern="A cool breeze"/>
```

### Supported Commands (Phase 3+)

| Command | Params | Description |
|---------|--------|-------------|
| `highlight.add` | pattern, fg, bg, bold, category, fast_parse, sound, squelch | Add/update a highlight |
| `highlight.remove` | pattern | Remove a highlight by pattern |
| `highlight.clear` | category (optional) | Clear all or by category |
| `squelch.add` | pattern | Add squelch pattern |
| `squelch.remove` | pattern | Remove squelch pattern |

### Implementation Notes
- Parser watches for `<vellumfe .../>` tags in the XML stream
- These tags are consumed by the FE and never rendered as text
- Changes apply immediately to the active session
- Changes are optionally persisted to `highlights.toml` (param: `persist="true"`)
- This goes in Phase 3 alongside the highlight editor

---

## Current Status

- [x] Repo created: https://github.com/Buckwheet/VellumFE-Tabbed
- [x] VellumFE codebase cloned as base
- [x] All reference repos synced locally
- [ ] Phase 1 not started
