#!/usr/bin/env python3
"""Build and package the native app; never publish or read legacy user data."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PACKAGER_VERSION = "0.11.8"
FORMATS = {"Darwin": ["app", "dmg"], "Windows": ["nsis"], "Linux": ["deb", "appimage", "pacman"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--debug", action="store_true", help="Package a development build for local review")
    parser.add_argument("--formats", help="Comma-separated package formats for the current OS")
    parser.add_argument("--skip-build", action="store_true", help="Use an already built executable")
    args = parser.parse_args()
    system, machine = platform.system(), platform.machine().lower()
    if system not in FORMATS or (system == "Darwin" and machine not in ("arm64", "aarch64")) or (system != "Darwin" and machine not in ("amd64", "x86_64")):
        parser.error("Supported targets: macOS Apple Silicon, Windows x64, Linux x64")
    formats = args.formats.split(",") if args.formats else FORMATS[system]
    if not formats or any(item not in FORMATS[system] for item in formats):
        parser.error(f"Allowed formats on this host: {', '.join(FORMATS[system])}")
    packager = os.environ.get("HSPLANNER_PACKAGER") or shutil.which("cargo-packager")
    if not packager:
        parser.error(f"Install cargo-packager: cargo install cargo-packager --version {PACKAGER_VERSION} --locked")
    if subprocess.check_output([packager, "--version"], text=True).strip().split()[-1] != PACKAGER_VERSION:
        parser.error(f"Use cargo-packager {PACKAGER_VERSION} for reproducible packaging")
    env = os.environ.copy()
    if system == "Darwin":
        env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    if not args.skip_build:
        subprocess.run(["cargo", "build", "-p", "hsplanner", "--locked"] + ([] if args.debug else ["--release"]), cwd=ROOT, env=env, check=True)
    profile = "debug" if args.debug else "release"
    binary_dir = ROOT / "target" / profile
    binary = binary_dir / ("hsplanner.exe" if system == "Windows" else "hsplanner")
    if not binary.is_file():
        parser.error(f"Missing executable: {binary}")
    config = json.loads((ROOT / "packaging/packager.json").read_text())
    config["version"] = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    config["icons"] = [str(ROOT / "packaging" / path) for path in config["icons"]]
    for resource in config.get("resources", []):
        resource["src"] = str((ROOT / "packaging" / resource["src"]).resolve())
    config["binariesDir"] = str(binary_dir)
    destination = ROOT / "target/packages" / profile
    destination.mkdir(parents=True, exist_ok=True)
    config["outDir"] = str(destination)
    config["formats"] = formats
    identity = env.get("HSPLANNER_MACOS_SIGNING_IDENTITY")
    if identity:
        config["macos"]["signingIdentity"] = identity
    if env.get("HSPLANNER_WINDOWS_SIGN_COMMAND"):
        config["windows"] = {"signCommand": env["HSPLANNER_WINDOWS_SIGN_COMMAND"]}
    config_path = destination / "packager.json"
    config_path.write_text(json.dumps(config, indent=2) + "\n")
    subprocess.run([packager, "--config", str(config_path)], cwd=ROOT, env=env, check=True)
    packages = sorted(path for path in destination.iterdir() if path.is_file()
                      and path.suffix in (".dmg", ".exe", ".deb", ".AppImage", ".gz"))
    checksums = []
    for path in packages:
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        checksums.append(f"{digest}  {path.name}\n")
    (destination / "SHA256SUMS").write_text("".join(checksums))
    print(f"Packages: {destination}")


if __name__ == "__main__":
    main()
