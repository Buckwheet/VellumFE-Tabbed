# VellumFE-Tabbed

A terminal frontend for **GemStone IV** (and DragonRealms) built in Rust. Run multiple characters simultaneously in a single terminal window — no Electron, no JVM.

Built on [VellumFE](https://github.com/Nisugi/VellumFE) by Nisugi.

---

## What it does

- **Multi-session** — up to 15 simultaneous sessions, each with isolated state, in one window
- **Profile manager** — Warlock-style TUI picker on every launch; save named character profiles (account, character, game, optional Lich proxy); passwords stored in the OS keychain
- **Direct login** — authenticates directly via eAccess (no Lich required); fetches your character list automatically
- **Lich proxy** — connect through a running Lich instance instead
- **Tab bar** — live status indicators per session (●=connected, …=connecting, ↻=reconnecting, !=error), unread badges, compact mode (Ctrl+Shift+C)
- **Broadcast** — Ctrl+B sends the next command to all sessions at once
- **Highlights** — global + per-character overrides; Lich scripts can push highlight changes at runtime via XML tags
- **Themes, layouts, TTS, sound alerts** — all inherited from VellumFE; layout editor (F2), highlight browser (F3)

---

## Install

### Pre-built binary (Windows recommended)

Download from [Releases](https://github.com/Buckwheet/VellumFE-Tabbed/releases), unzip, run `vellum-fe-tabbed.exe`.

### Build from source

Requires Rust 1.75+:

```bash
git clone https://github.com/Buckwheet/VellumFE-Tabbed
cd VellumFE-Tabbed
cargo build --release
```

**Linux** — keyring dependency:
```bash
# Debian/Ubuntu
sudo apt install libdbus-1-dev pkg-config
```

---

## First launch

On first launch the profile picker opens automatically. Press `N` to create a new profile:

1. Enter a profile name (e.g. `Brashka - Prime`)
2. Enter your SGE account name — your character list loads automatically
3. Pick a character and game (Left/Right arrows cycle games)
4. Optionally configure a Lich proxy host/port
5. Press Enter to save and connect

Passwords are stored in the OS keychain (Windows Credential Manager, macOS Keychain, Linux libsecret) — never in plain text.

---

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| Ctrl+1–9 | Switch to session 1–9 |
| Ctrl+Tab / Ctrl+Shift+Tab | Next / previous session |
| Ctrl+T | New session |
| Ctrl+W | Close current session |
| Ctrl+B | Broadcast next command to all sessions |
| Ctrl+Shift+C | Toggle compact tab bar |
| F2 | Layout editor |
| F3 | Highlight browser |

**Profile picker** (shown on launch):

| Key | Action |
|-----|--------|
| N | New profile |
| E | Edit selected profile |
| D | Delete selected profile |
| Enter | Connect |
| Left / Right | Cycle game (in edit mode, Game field) |

---

## Lich script protocol

Lich scripts can control highlights at runtime via XML tags sent over the proxy stream:

```xml
<vellumfe cmd="highlight.add" pattern="Buckwheet" fg="#ff00ff" bold="true" category="Friends"/>
<vellumfe cmd="highlight.remove" pattern="Buckwheet"/>
<vellumfe cmd="highlight.clear" category="Friends"/>
<vellumfe cmd="squelch.add" pattern="A cool breeze"/>
```

Add `persist="true"` to save to disk.

---

## Config files

```
~/.vellum-fe/profiles.toml                    # Saved character profiles
~/.vellum-fe/global/highlights.toml           # Global highlights
~/.vellum-fe/profiles/<char>/highlights.toml  # Per-character overrides
~/.vellum-fe/profiles/<char>/layout.toml      # Widget layout (auto-saved)
```

---

## Credits

- [VellumFE](https://github.com/Nisugi/VellumFE) by Nisugi — core engine
- [Illthorn](https://github.com/elanthia-online/illthorn) — multi-session inspiration
- [Warlock](https://github.com/WarlockFE/warlock3) — profile picker UX inspiration

## License

MIT
