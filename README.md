# VellumFE-Tabbed

A high-performance, multi-session terminal frontend for [GemStone IV](https://www.play.net/gs4/) — built for players who run multiple characters and refuse to sacrifice speed for features.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)
![Status](https://img.shields.io/badge/status-in%20development-yellow)

---

## What This Is

Most GemStone frontends make you choose between performance and features. Electron-based clients like Wyrath are feature-rich but introduce lag. Lightweight clients like ProfanityFE are fast but lack multi-session support and modern UX.

VellumFE-Tabbed aims to be both — a compiled Rust binary that runs up to 15 simultaneous sessions in a single terminal window, with a full widget system, per-character highlights, and a session picker/tab bar to switch between characters instantly.

---

## Features

- **Multi-session** — up to 15 simultaneous GemStone sessions in one window
- **Tabbed or compact layout** — full tab bar or minimal session picker, your choice
- **Widget system** — progress bars, compass, hands, injury doll, countdowns, active effects, spells, inventory, targets, and more
- **Tabbed text windows** — route game streams (thoughts, combat, loot, death) to organized tabs
- **Highlight system** — regex + fast literal matching (Aho-Corasick), per-character and global
- **Sound alerts** — play sounds on pattern matches
- **TTS support** — text-to-speech for accessibility
- **Direct eAccess login** — connect without Lich proxy
- **Lich proxy login** — full Lich script support
- **Lich script → FE commands** — scripts can add/remove highlights at runtime via `<vellumfe>` XML tags
- **Fully themeable** — complete color customization
- **Layout editor** — interactive widget positioning (F2)
- **Per-character config** — highlights, layout, keybinds, and colors saved per character

---

## Quick Start

> ⚠️ Pre-built binaries not yet available. Build from source below.

### Via Lich Proxy

```bash
vellum-fe-tabbed --port 8000 --character YourCharacter
```

### Direct Connection (no Lich)

```bash
vellum-fe-tabbed --direct \
  --account YOUR_ACCOUNT \
  --password YOUR_PASSWORD \
  --game prime \
  --character CHARACTER_NAME
```

---

## Build from Source

**Requirements:** Rust 1.70+ stable

```bash
git clone https://github.com/Buckwheet/VellumFE-Tabbed.git
cd VellumFE-Tabbed
cargo build --release
```

Binary will be at `target/release/vellum-fe-tabbed.exe` (Windows) or `target/release/vellum-fe-tabbed` (Linux).

---

## Configuration

Config files live in `~/.config/vellum-fe-tabbed/`:

```
~/.config/vellum-fe-tabbed/
├── sessions.toml              # Saved session list
├── global/
│   ├── highlights.toml        # Highlights applied to all sessions
│   └── keybinds.toml
└── characters/
    └── <CharacterName>/
        ├── config.toml
        ├── highlights.toml    # Per-character highlight overrides
        ├── layout.toml
        └── colors.toml
```

Example highlight:

```toml
[stunned]
pattern = "You are stunned"
fg = "#ff0000"
bold = true
sound = "alert.wav"
category = "Combat"
```

---

## Default Keybinds

| Key | Action |
|-----|--------|
| `Ctrl+1..9` | Switch to session by number |
| `Ctrl+T` | New session |
| `Ctrl+W` | Close session |
| `Ctrl+Shift+C` | Toggle compact mode |
| `F2` | Layout editor |
| `F3` | Highlight browser |
| `Page Up/Down` | Scroll main window |
| `Escape` | Close popups |

---

## Project Status

See [PROJECT_PLAN.md](PROJECT_PLAN.md) for the full roadmap and phase breakdown.

- [x] Repo setup, base codebase imported
- [ ] Phase 1: Multi-session core
- [ ] Phase 2: Session picker + compact mode
- [ ] Phase 3: Highlights polish + Lich script protocol
- [ ] Phase 4: Login flows
- [ ] Phase 5: Cross-session features
- [ ] Phase 6: Windows build + CI/CD

---

## Credits & Acknowledgments

This project stands on the shoulders of several excellent open-source GemStone frontends:

- **[VellumFE](https://github.com/Nisugi/VellumFE)** by Nisugi — the Rust/Ratatui foundation this project is built on. The widget system, highlight engine, parser, and TUI architecture all originate here.
- **[Illthorn](https://github.com/elanthia-online/illthorn)** by Benjamin Clos — the multi-session architecture and session picker concept are inspired by Illthorn's TypeScript implementation.
- **[Warlock3](https://github.com/sproctor/warlock3)** by Sean Proctor — reference for dual login (Lich + direct eAccess) and per-character settings persistence.
- **[ProfanityFE](https://github.com/elanthia-online/ProfanityFE)** — the original lightweight terminal frontend that proved a fast GemStone client was possible.

---

## License

Licensed under either of:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
