<img width="1600" height="360" alt="wordmark-1600" src="https://github.com/user-attachments/assets/fe4fd4b3-a475-438a-a87e-53fd8080632d" />


A desktop build planner for **Hero Siege** - a calculator for the talent tree, gear, stats, and skills.

![Python](https://img.shields.io/github/v/release/zium1337/HSPlanner)
[![Download](https://img.shields.io/github/v/release/zium1337/HSPlanner?label=Download)](https://github.com/zium1337/HSPlanner/releases/latest)

---

## Features

- [ ] **Talent tree** *(around 50% of passives works now)* — interactive pan/zoom graph view with auto-pathfinding, path preview on hover, and reset
- [ ] **Skills and sub-skills** *(currently stormweaver was added)* — point allocation that respects skill prerequisites and per-level caps
- [ ] **Gear** *(around 70% done)* — slots for weapons, armor, charms, jewelry with sockets (gems/runes) and runeword detection
- [x] **Affixes** — add affixes with tier and adjustable roll
- [x] **Stats** — aggregated bonuses from tree, gear, attributes, and runewords
- [x] **Custom stats** — free-text user-entered stats for things outside the data model
- [x] **Notes** — sanitized WYSIWYG editor (per build), preserved across share links
- [x] **Builds menu** — multiple saved builds, each with multiple profiles
- [x] **Share** — export the entire build to a compressed URL (lz-string)
- [x] **Update check** — opt-in update check via GitHub Releases

<img width="1710" height="1041" alt="image" src="https://github.com/user-attachments/assets/1bed9cf0-fbff-4231-b7bd-42ae25968938" />

---

## How to install

1. Go to [latest release](https://github.com/zium1337/HSPlanner/releases/latest)
2. Scroll down and in **assets** section, pick the installer for your OS for example Windows installer is named HSPlanner_x.x.x_x64-setup.exe where "x" it is a version number

---

## Runtime requirements (prebuilt app)

Download the installer / binary for your platform from Releases. The app is self-contained — end users do not need Node or Rust installed.

| Platform | Required component |
|---|---|
| Windows 10/11 | WebView2 Runtime (preinstalled on Win11; the installer pulls it on Win10) |
| macOS 10.15+ | None — WebKit is built into the OS |
| Linux (x86_64) | `webkit2gtk-4.1`, `libssl`, standard GTK runtime libraries |

---

## Development requirements

| Tool | Minimum version | Purpose |
|---|---|---|
| **Node.js** | 20.x LTS or newer | Frontend (Vite + React) |
| **npm** | 10.x (ships with Node 20+) | Package manager |
| **Rust toolchain** | `rustup` with `stable` (≥ 1.77) | Tauri backend |
| **Tauri prerequisites** | see below per OS | Linker, system libraries |

### Build

```bash
git clone https://github.com/zium1337/HSPlanner.git
cd HSPlanner
npm install
npm run tauri:dev
```

### Tauri — system prerequisites

**Windows**
- Microsoft Visual Studio C++ Build Tools (workload "Desktop development with C++")
- WebView2 Runtime (only needed on Win10; Win11 has it preinstalled)

**macOS (only for development purpose because game doesn't support macos)**
- Xcode Command Line Tools: `xcode-select --install`

**Linux (Debian/Ubuntu)**
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

For more information about tauri see [official Tauri guide](https://tauri.app/start/prerequisites/)
