# Native installers

The next public release is GPUI. Users download a new installer from GitHub;
Tauri does not upgrade into it. Old Tauri libraries are unsupported and ignored.
Original Tauri data is not deleted. The installation name is **HSPlanner**
and its identifier remains `com.zium.hsplanner.gpui`.

## Build locally

Use Python 3.11+, the repository's stable Rust toolchain and the platform build
requirements in `.github/workflows/native.yml`. Node.js/Tauri are not needed.

```sh
cargo install cargo-packager --version 0.11.8 --locked
python3 tools/package-native.py
```

The script builds only `hsplanner`, reads the version from root `Cargo.toml`, and
writes packages to `target/packages/release/`. Formats are host-specific:

| Host | Output |
|---|---|
| macOS 15+, Apple Silicon | `.app`, `.dmg` |
| Windows 10/11 x64 | NSIS `.exe`, current-user installation |
| Linux x64 | `.deb`, `.AppImage`, Pacman package + PKGBUILD |

Linux requires native Wayland and a working Vulkan driver. Initial DEB dependencies
target Ubuntu 24.04; other distributions need separate installation verification.
RPM packaging is not implemented by the pinned packager and remains a follow-up.

For a test package: `python3 tools/package-native.py --debug`. Use `--formats app`
on macOS for a quick bundle, or `--skip-build` only with a matching already-built
executable. `HSPLANNER_PACKAGER` can point to a locally installed packager binary.

## GitHub workflow and signing

Run **Native installers** manually. It builds all three platforms and uploads
installers as Actions artifacts, without publishing a release or updating any feed.
Before publishing, choose an unused version in root `Cargo.toml`, edit
`packaging/release-notes.md`, verify the packages and create the GitHub release.

## In-app updates

The native app reads `https://api.github.com/repos/HeroSiegePlanner/HSPlanner/releases/latest`
(see `crates/app/src/update.rs`). No manifest or signing key is needed; the release
itself is the feed:

- Publish native installers as regular assets and keep the file names produced by
  cargo-packager (`HSPlanner_<version>_<arch>.dmg` / `_x64-setup.exe`). The
  `SHA256SUMS` file is how the app distinguishes current packages from legacy Tauri
  assets in the same repository; a release without it is ignored.
- Tag the release `v<version>` matching root `Cargo.toml`. Drafts and pre-releases are
  never offered.
- GitHub publishes a sha256 digest per asset; the app refuses to run an installer
  without one, so upload through the GitHub UI, `gh release upload` or the API.

macOS replaces the running `.app` bundle in place and relaunches; if the bundle is not
writable the DMG is opened in Finder instead. Windows starts the NSIS installer and
the app quits. Linux users get the release page (deb/AppImage/pacman cannot be told
apart). Failed checks on startup stay silent; the footer button reports errors when
pressed by hand.

Local/CI builds without certificates are test packages, not signed public releases.
For macOS configure `HSPLANNER_MACOS_SIGNING_IDENTITY`, `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, and notarization credentials (`APPLE_ID`,
`APPLE_PASSWORD`, `APPLE_TEAM_ID`). For Windows configure the trusted
`HSPLANNER_WINDOWS_SIGN_COMMAND` with `%1` as the file to sign. Keep credentials in
CI secrets, never in configuration files. Signing and notarization still require
verification on the release runners. See the [packager configuration reference](https://docs.crabnebula.dev/packager/configuration/).

## Changelog and verification

The native footer's version button opens the bundled `release-notes.md` offline.
The dialog displays the running Cargo version, renders Markdown, scrolls at small
window sizes and offers Close/Escape and a keyboard-accessible GitHub link.
Developer history stays in root `CHANGELOG.md`; it is not copied into user-facing
release notes. Update release notes before each installer build.

Verify on each target: install, launch without a development checkout, reopen a
native save, open/scroll/close changelog, uninstall without deleting saved data,
and reinstall. Windows and Wayland installation are not certified by macOS tests.
Updates between GPUI releases are handled in-app; see *In-app updates*.

## Verified locally — 2026-09-08

- Built a macOS Apple Silicon **debug/test** DMG; `hdiutil verify` passed.
- Copied the app out of the mounted DMG, detached it, then launched the copied
  executable outside the checkout with isolated data. Native trees and fonts loaded.
- Verified the package identity, version 1.0.7 and macOS minimum 15.0; bundled
  the project and font licenses. The final executable hash matches the tested copy.
- Verified changelog at 960×600, 100% and 150%, and Escape plus Tab/Enter closing.
  Screenshot: local `outputs/native-installer/changelog-960.jpg`.
- Formatting, app/build/library all-target Clippy and 22 build/storage/gear tests
  passed, including ignoring an old malformed transfer file.
- Windows/Linux packaging workflow, RPM support, public signing/notarization and
  installation on other machines remain unverified. No release was published.
