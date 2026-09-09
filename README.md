<img width="1600" height="360" alt="wordmark-1600" src="https://github.com/user-attachments/assets/fe4fd4b3-a475-438a-a87e-53fd8080632d" />


A desktop build planner for **Hero Siege** - a calculator for the talent tree, gear, stats, and skills.

[![Release](https://img.shields.io/github/v/release/zium1337/HSPlanner)](https://github.com/zium1337/HSPlanner/releases/latest)
[![Download](https://img.shields.io/github/v/release/zium1337/HSPlanner?label=Download)](https://github.com/zium1337/HSPlanner/releases/latest)
![DL](https://badgen.net/github/assets-dl/HeroSiegePlanner/HSPlanner)

---

## Features

The planner is split into tabs:

<details>
<summary>Character — read-only dashboard: class, level, attributes, and the live damage/defence readout for the current build</summary>

<img width="1710" height="1041" alt="image" src="https://github.com/user-attachments/assets/85a6820e-d527-4faa-a15b-a108158fa1ea" />

</details>

<details>
<summary>Incarnation Tree — interactive pan/zoom talent graph with auto-pathfinding, path preview on hover, minimap, and reset</summary>

<img width="1710" height="1071" alt="image" src="https://github.com/user-attachments/assets/f430c2ed-e84f-4dfa-8933-660781b45a31" />

</details>

<details>
<summary>Ether Tree — Same like `Incarnation Tree` but with its own node graph and summary panel</summary>

<img width="1710" height="1043" alt="image" src="https://github.com/user-attachments/assets/8f090a91-f204-41ab-bcd2-4a321de659f5" />

</details>

<details>
<summary>Skills — point allocation that respects skill prerequisites and per-level caps, plus sub-skills</summary>

<img width="1710" height="1044" alt="image" src="https://github.com/user-attachments/assets/dfa357a1-3fb6-4c6c-bbc8-264238c904e8" />

</details>

<details>
<summary>Gear — slots for weapons, armor, charms, jewelry with sockets (gems/runes), runeword detection, and set bonuses</summary>

<img width="1710" height="1042" alt="image" src="https://github.com/user-attachments/assets/1c5e71bb-75d2-429c-a29d-92d31d0102cb" />

</details>

<details>
<summary>Merc — mercenary slot with its own gear and stat contribution</summary>

<img width="1710" height="1042" alt="image" src="https://github.com/user-attachments/assets/8699c5bd-1133-4553-bad9-63004a399e98" />

</details>

<details>
<summary>Stats — aggregated bonuses from tree, ether, gear, merc, attributes, runewords, and sets</summary>

<img width="1710" height="1041" alt="image" src="https://github.com/user-attachments/assets/c47238ca-f3df-4b62-b1eb-b8aa298789db" />

</details>

<details>
<summary>Config — class/level/attributes, conditional toggles, and progression sliders</summary>

<img width="1710" height="1044" alt="image" src="https://github.com/user-attachments/assets/2bb20bbd-3181-4d75-9626-495bf82b916a" />

</details>

<details>
<summary>Notes — sanitized WYSIWYG editor (per build), preserved across share links</summary>

<img width="1710" height="1043" alt="image" src="https://github.com/user-attachments/assets/83d9551f-65b5-43cb-ab14-af465e48cb3b" />

</details>

<details>
<summary>Filters — loot filter editor, including "generate from build"</summary>

<img width="1710" height="1043" alt="image" src="https://github.com/user-attachments/assets/47ffe454-0471-42ca-ba88-75230f4bfc8c" />

</details>

Across every tab:

- [x] **Affixes** — add affixes by family, pick a tier, and drag roll sliders (item-granted skill ranks roll too)
- [x] **Custom stats** — free-text user-entered stats for things outside the data model
- [x] **Seasons** — Season 10 is the base data; later seasons are applied as patch layers on top
- [x] **Builds menu** — multiple saved builds, each with multiple profiles
- [x] **Share** — export the entire build to a compressed URL (lz-string), optionally via the web share service
- [x] **Update check** — opt-in update check via GitHub Releases

<img width="1710" height="1044" alt="image" src="https://github.com/user-attachments/assets/bcbcfce1-b70e-4c8c-bca3-e3f991fe6317" />


---

## How to install

1. Go to [latest release](https://github.com/zium1337/HSPlanner/releases/latest)
2. In the **Assets** section, pick the file for your OS:

| Platform | Asset |
|---|---|
| Windows | `HSPlanner-x64-setup.exe` |
| macOS | `HSPlanner-x64.dmg` |
| Debian / Ubuntu | `HSPlanner-amd64.deb` |
| Fedora / RHEL | `HSPlanner-x86_64.rpm` |
| Arch Linux | `hsplanner-bin-*.pkg.tar.zst` (`pacman -U`) |
| Other Linux | `HSPlanner-amd64.AppImage` |

---

## Runtime requirements (prebuilt app)

The app is self-contained — end users do not need Node or Rust installed.

| Platform | Required component |
|---|---|
| Windows 10/11 | WebView2 Runtime (preinstalled on Win11; the installer pulls it on Win10) |
| macOS 10.15+ | None — WebKit is built into the OS |
| Linux (x86_64) | `webkit2gtk-4.1`, `gtk3`, `libssl`, standard GTK runtime libraries |

---

## Development

The native GPUI app is the default development target. Use stable Rust and the
platform dependencies listed in `.github/workflows/native.yml`. Native targets
are macOS 15+ Apple Silicon, Windows 10/11 x64 and Linux x64 with native Wayland.

```bash
cargo run -p hsplanner
cargo build --release -p hsplanner
```

The native executable is `target/release/hsplanner` (`.exe` on Windows).
Native installer tooling is available via `python3 tools/package-native.py`;
the app checks GitHub releases for updates and can install them from the footer.
See [packaging/README.md](packaging/README.md).

### Archived Tauri reference

`legacy/` and `tree-renderer/` are optional, ignored local archives. They are
not required by the native workspace, CI or packaging. The former Tauri
sources remain available in Git history before the GPUI migration.

### Project structure

| Path | Contents |
|---|---|
| `crates/` | Native GPUI app, views, documents and shared UI |
| `engine/` | Shared Rust calculations, OCR and suggestions |
| `data/` | Shared game JSON; current base is Season 10 |
| `assets/` | Shared graphics (skills, items, tree nodes) |
| `packaging/` | Native installer configuration and icons |
| `tools/` | Packaging and optional visual comparison tools |

## FAQ

**Q:** *Can i import my save file from game to planner?*

**A:** *No you can't. It is against the EULA/TOS.*
