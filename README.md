# VellumFE-Tabbed

A terminal frontend for **GemStone IV** (and DragonRealms) built in Rust. No Electron, no JVM — just a compiled binary.

Built on [VellumFE](https://github.com/Nisugi/VellumFE) by Nisugi.

---

## What it does

The main addition over upstream VellumFE is a **Warlock-style profile manager** shown on every launch:

- Save named character profiles (account, character, game, optional Lich proxy host/port)
- Passwords stored in the OS keychain — never in plain text
- Character list fetched automatically from eAccess when you enter your account name
- Connect directly via eAccess (no Lich required) or through a running Lich instance
- Arrow-key navigation, Left/Right to cycle games, N/E/D to create/edit/delete profiles

Everything else is inherited from VellumFE: regex highlights, Aho-Corasick matching, sound alerts, TTS, layout editor (F2), highlight browser (F3), themes, squelch patterns, stream routing.

---

## Install

### Pre-built binary (Windows recommended)

Download from [Releases](https://github.com/Buckwheet/VellumFE-Tabbed/releases), unzip, run `vellum-fe.exe`.

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

The profile picker opens automatically. Press `N` to create a new profile:

1. Enter a profile name (e.g. `Brashka - Prime`)
2. Enter your SGE account name and password, then press Enter on the Character field to fetch your character list
3. Use Up/Down arrows to cycle through fetched characters; use Left/Right to cycle games (Prime, Platinum, Shattered, DragonRealms)
4. Set `Use Lich` to Yes with Left/Right if connecting through Lich (see below)
5. Press Enter on the last field to save and connect

On subsequent launches, select a profile and press Enter.

---

## Connecting via Lich

[Lich](https://lichproject.org) is a scripting proxy for GemStone IV. To use it with VellumFE:

1. Start Lich normally and let it connect to the game
2. Note the proxy port Lich is listening on (default: `8000`, host: `127.0.0.1`)
3. In VellumFE's profile editor, navigate to `Use Lich` and press Left/Right to set it to **Yes**
4. Set `Lich Host` to `127.0.0.1` (or the machine running Lich if remote)
5. Set `Lich Port` to match Lich's proxy port (default `8000`)
6. Save and connect — VellumFE will connect to Lich's proxy instead of eAccess directly

> **Note:** VellumFE currently requires Lich to already be running and connected before you press Enter. It does not launch Lich for you.

---

## Profile picker keys

| Key | Action |
|-----|--------|
| Up / Down | Navigate profiles (list) / Navigate fields (edit) |
| Up / Down | Cycle characters (edit, when on Character field) |
| Enter | Connect (list) / Next field / Fetch characters (edit, on Character field) |
| Left / Right | Cycle game or toggle Use Lich (edit mode) |
| N | New profile |
| E | Edit selected profile |
| D | Delete selected profile |
| Esc | Back / Quit |

---

## CLI flags

```bash
# Skip the picker and connect to a named profile
vellum-fe --profile "Brashka - Prime"

# Connect via Lich proxy directly
vellum-fe --port 8000

# Direct eAccess login without the picker
vellum-fe --direct --account myaccount --character Brashka
```

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
- [Warlock](https://github.com/WarlockFE/warlock3) — profile picker UX inspiration

## License

MIT
