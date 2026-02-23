# VellumFE-Tabbed

A high-performance, multi-session GemStone IV terminal frontend built on [VellumFE](https://github.com/Nisugi/VellumFE). Run up to 15 simultaneous sessions in a single terminal window. Zero Electron, zero JVM — compiled Rust binary.

## Features

- **Multi-session** — up to 15 simultaneous GemStone IV sessions, each with isolated state
- **Tab bar** — session tabs with unread badges and live status indicators (●=connected, …=connecting, ↻=reconnecting, !=error)
- **Compact mode** — minimal tab bar (Ctrl+Shift+C)
- **Session picker** — TUI launch screen to add/remove/connect sessions
- **Login wizard** — full TUI wizard for Direct eAccess login (account → game → character), no command line required
- **Lich proxy** — connect via Lich proxy (host:port)
- **Auto-connect** — sessions with `auto_connect = true` in sessions.toml reconnect on startup
- **Credential storage** — passwords stored in OS keychain (Keychain on macOS, libsecret on Linux, Credential Manager on Windows)
- **Broadcast** — Ctrl+B sends the next command to all sessions simultaneously
- **Highlights** — global highlights + per-character overrides; Lich scripts can control highlights via `<vellumfe cmd="..."/>` XML tags
- **Per-session sound/TTS** — enable or disable sound alerts and TTS per session
- All VellumFE features: regex highlights, Aho-Corasick fast matching, sound alerts, TTS, layout editor (F2), highlight browser (F3), themes, squelch patterns, stream routing, session cache

## Install

### Pre-built binaries (recommended)

Download the latest release from [GitHub Releases](https://github.com/Buckwheet/VellumFE-Tabbed/releases):

**Linux:**
```bash
tar -xzf vellum-fe-tabbed-linux-x86_64.tar.gz
chmod +x vellum-fe-tabbed
./vellum-fe-tabbed
```

**Windows:**
```
Unzip vellum-fe-tabbed-windows-x86_64.zip
Run vellum-fe-tabbed.exe
```

### Build from source

Requires Rust 1.75+:

```bash
git clone https://github.com/Buckwheet/VellumFE-Tabbed
cd VellumFE-Tabbed
cargo build --release
./target/release/vellum-fe-tabbed
```

**Linux dependencies** (for keyring/libsecret):
```bash
# Debian/Ubuntu
sudo apt install libdbus-1-dev pkg-config

# Fedora/RHEL
sudo dnf install dbus-devel pkgconf
```

## Usage

```bash
# Launch (shows session picker if no sessions configured)
vellum-fe-tabbed

# Connect directly to a Lich proxy
vellum-fe-tabbed --host localhost --port 8000

# Connect with a character label
vellum-fe-tabbed --host localhost --port 8000 --character Buckwheet
```

On first launch with no sessions configured, the session picker opens automatically. Use it to add sessions via Lich proxy or Direct eAccess login.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Ctrl+1..9 | Switch to session 1–9 |
| Ctrl+Tab | Next session |
| Ctrl+Shift+Tab | Previous session |
| Ctrl+T | New session (opens picker) |
| Ctrl+W | Close current session |
| Ctrl+B | Broadcast next command to all sessions |
| Ctrl+Shift+C | Toggle compact tab bar |
| F2 | Layout editor |
| F3 | Highlight browser |

## Session Picker

Press Ctrl+T or launch with no sessions to open the session picker.

- Arrow keys to navigate
- Enter to connect
- `A` or select `[+ Add Session]` to add a new session
- F2 in the add form to toggle Lich Proxy / Direct mode
- `D` to remove a session

## Lich Script Protocol

Lich scripts can control highlights at runtime by sending XML tags over the proxy stream:

```xml
<vellumfe cmd="highlight.add" pattern="Buckwheet" fg="#ff00ff" bold="true" category="Friends"/>
<vellumfe cmd="highlight.remove" pattern="Buckwheet"/>
<vellumfe cmd="highlight.clear" category="Friends"/>
<vellumfe cmd="squelch.add" pattern="A cool breeze"/>
<vellumfe cmd="squelch.remove" pattern="A cool breeze"/>
```

Add `persist="true"` to save the change to disk:
```xml
<vellumfe cmd="highlight.add" pattern="Buckwheet" fg="#ff00ff" persist="true"/>
```

## Config Files

```
~/.config/vellum-fe-tabbed/sessions.toml   # Session list
~/.vellum-fe/global/highlights.toml        # Global highlights (all sessions)
~/.vellum-fe/profiles/<char>/highlights.toml  # Per-character highlight overrides
~/.vellum-fe/profiles/<char>/layout.toml      # Widget layout (auto-saved on exit)
```

## Credits

Built on [VellumFE](https://github.com/Nisugi/VellumFE) by Nisugi. Multi-session architecture inspired by [Illthorn](https://github.com/elanthia-online/illthorn). Login wizard flow inspired by [Warlock](https://github.com/WarlockFE/warlock3).

## License

MIT
