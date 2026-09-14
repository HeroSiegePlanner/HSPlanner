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

For test packages, run **Native installers** manually (Actions, Run workflow, any
branch) or add the `installers` label to a pull request. It builds Windows and
Linux and uploads Actions artifacts without publishing a release.

For a public release, update `CHANGELOG.md`, then open **Actions → Release → Run
workflow**. Choose the source branch, enter `tag` as `1.1.0` or `v1.1.0`, and
optionally check `prerelease`. Use an unused three-part version; preview releases
use the checkbox rather than a suffix. The workflow file must first be present
on the default branch for GitHub to show the manual dispatch form.

The release workflow:

1. Sets the native workspace version in `Cargo.toml` and `Cargo.lock`, commits the
   change when needed, and atomically pushes the selected branch and `v<version>`
   tag. It does not force-push or update dependency versions. Branch/tag rules
   must allow the workflow's `GITHUB_TOKEN` to perform these writes.
2. Tests that exact commit and builds Windows x64 NSIS, Linux x64 DEB/AppImage/
   Pacman, and macOS Apple Silicon DMG packages through the existing workflows.
3. Verifies every package checksum and produces one combined `SHA256SUMS`,
   including the Pacman `PKGBUILD`.
4. Uploads all packages to a draft with `CHANGELOG.md` as its description, then
   publishes it only after tests, all platforms, and uploads succeed. A preview
   is marked as a prerelease and never as Latest.

If a build or upload fails, use **Re-run failed jobs** on the same run. The tag is
already reserved; an upload failure leaves an unpublished draft that the job can
resume. Existing public releases and drafts from another commit are not replaced.
The version checked into the tagged source matches the executable and installers.

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

The native footer's version button opens the root `CHANGELOG.md`, embedded at build time for offline use.
The dialog displays the running Cargo version, renders Markdown, scrolls at small
window sizes and offers Close/Escape and a keyboard-accessible GitHub link.
Keep all release entries in `CHANGELOG.md`, newest first. Update this single file
before each installer build; **Release** uses it for the GitHub release body.

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

## Problem reports

Native packages use the same report destination as Tauri. Set `HSPLANNER_BUG_REPORT_URL`
(or the existing `VITE_BUG_REPORT_URL`) in the build environment. Local builds also
read these keys from the ignored root `.env`; the native key takes precedence.
The destination is embedded in the executable without printing it in build logs.
A runtime `HSPLANNER_BUG_REPORT_URL` overrides it for isolated testing. Without a
destination, the form remains available and explains why sending is disabled.

The **Native installers** workflow passes the `HSPLANNER_BUG_REPORT_URL` Actions
secret to the build, falling back to the existing `VITE_BUG_REPORT_URL` secret.
No local `.env` file is needed on the runners. Fork pull requests do not receive
repository secrets, so their test packages keep report sending disabled.

Reports are sent only through **Send report**. They include the visible fields,
app version and OS, selected images (PNG/JPEG/WebP/GIF, at most three and 8 MB each),
and the captured build code only when **Attach build** is checked. Network failures
preserve the draft; uploads are never retried automatically.
