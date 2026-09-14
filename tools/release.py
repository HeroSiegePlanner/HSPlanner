#!/usr/bin/env python3
"""Prepare native release versions, verify installers, and publish a complete draft."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ARTIFACTS = {
    "hsplanner-Windows-X64": ("*.exe",),
    "hsplanner-Linux-X64": ("*.deb", "*.AppImage", "*.tar.gz", "PKGBUILD"),
    "hsplanner-macOS-ARM64": ("*.dmg",),
}


def release_version(value):
    match = re.fullmatch(r"v?((?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))", value.strip())
    if not match:
        raise ValueError("Enter a version such as 1.1.0 or v1.1.0; use the prerelease checkbox for previews.")
    version = match[1]
    if any(int(part) > 65535 for part in version.split(".")):
        raise ValueError("Version components must not exceed 65535 (Windows installer limit).")
    return version


def replace_version(text, version):
    updated, count = re.subn(r'''(?m)^version\s*=\s*["'][^"']*["']''', f'version = "{version}"', text, count=1)
    if count != 1:
        raise ValueError("Version field is missing.")
    return updated


def prepare_version(root, value):
    version = release_version(value)
    manifest_path = root / "Cargo.toml"
    lock_path = root / "Cargo.lock"
    manifest_text = manifest_path.read_text()
    manifest = tomllib.loads(manifest_text)
    old_version = manifest["workspace"]["package"]["version"]
    if not (root / "CHANGELOG.md").read_text().strip():
        raise ValueError("CHANGELOG.md is empty.")
    members = set()
    for member in manifest["workspace"]["members"]:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())["package"]
        if isinstance(package.get("version"), dict) and package["version"].get("workspace"):
            members.add(package["name"])
    section = re.search(r"(?ms)^\[workspace\.package\]\s*\n.*?(?=^\[|\Z)", manifest_text)
    if not section or not members:
        raise ValueError("Native workspace version or packages are missing.")
    new_manifest = manifest_text[:section.start()] + replace_version(section[0], version) + manifest_text[section.end():]
    blocks = re.split(r"(?m)(?=^\[\[package\]\]$)", lock_path.read_text())
    updated_members = set()
    for index, block in enumerate(blocks):
        if not block.startswith("[[package]]"):
            continue
        package = tomllib.loads(block)["package"][0]
        if package["name"] in members and "source" not in package:
            if package["version"] != old_version:
                raise ValueError(f"Cargo.lock version differs for {package['name']}.")
            blocks[index] = replace_version(block, version)
            updated_members.add(package["name"])
    if updated_members != members:
        raise ValueError("Cargo.lock does not contain all native workspace packages.")
    new_lock = "".join(blocks)
    tomllib.loads(new_manifest)
    tomllib.loads(new_lock)
    manifest_path.write_text(new_manifest)
    lock_path.write_text(new_lock)
    return f"v{version}"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def collect_assets(source, destination):
    assets = {}
    for artifact, patterns in ARTIFACTS.items():
        directory = source / artifact
        checksums = {}
        for line in (directory / "SHA256SUMS").read_text().splitlines():
            match = re.fullmatch(r"([0-9a-f]{64})  ([^/\\]+)", line)
            if not match or match[2] in checksums:
                raise ValueError(f"Invalid checksum entry in {artifact}.")
            checksums[match[2]] = match[1]
        expected = set()
        for pattern in patterns:
            matches = list(directory.glob(pattern))
            if len(matches) != 1 or not matches[0].is_file():
                raise ValueError(f"Expected one {pattern} in {artifact}.")
            path = matches[0]
            expected.add(path.name)
            if path.name in assets:
                raise ValueError(f"Duplicate release asset: {path.name}.")
            checksum = digest(path)
            if path.name != "PKGBUILD" and checksums.get(path.name) != checksum:
                raise ValueError(f"Checksum mismatch: {path.name}.")
            assets[path.name] = (path, checksum)
        if set(checksums) != expected - {"PKGBUILD"}:
            raise ValueError(f"Unexpected or missing packages in {artifact} checksums.")
    # Validate every platform before producing any release assets.
    destination.mkdir(parents=True, exist_ok=True)
    if any(destination.iterdir()):
        raise ValueError("Release asset directory must be empty.")
    for name, (path, _) in assets.items():
        shutil.copyfile(path, destination / name)
    (destination / "SHA256SUMS").write_text("".join(
        f"{checksum}  {name}\n" for name, (_, checksum) in sorted(assets.items())
    ))


def publish_release(root, tag, sha, assets, prerelease):
    if tag != f"v{release_version(tag)}" or not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ValueError("Invalid release tag or commit.")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    if tag != f"v{version}":
        raise ValueError("Release tag does not match the application version.")
    files = sorted(path for path in assets.iterdir() if path.is_file())
    if not files or not (assets / "SHA256SUMS").is_file():
        raise ValueError("Verified release assets are missing.")
    verified = set()
    for line in (assets / "SHA256SUMS").read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([^/\\]+)", line)
        if not match or match[2] in verified or digest(assets / match[2]) != match[1]:
            raise ValueError("Release assets changed after verification.")
        verified.add(match[2])
    if verified != {path.name for path in files} - {"SHA256SUMS"} or not verified:
        raise ValueError("Release checksums do not cover all assets.")
    ref = json.loads(subprocess.check_output(
        ["gh", "api", f"repos/{os.environ['GH_REPO']}/git/ref/tags/{tag}"], text=True,
    ))
    if ref["object"]["type"] != "commit" or ref["object"]["sha"] != sha:
        raise ValueError("Release tag does not point to the tested commit.")
    # Listing must succeed; network/authentication errors must not mean 'absent'.
    releases = json.loads(subprocess.check_output(
        ["gh", "api", "--paginate", "--slurp", f"repos/{os.environ['GH_REPO']}/releases"], text=True,
    ))
    existing = next((release for page in releases for release in page if release["tag_name"] == tag), None)
    if existing:
        if not existing["draft"] or existing["target_commitish"] != sha:
            raise ValueError("Refusing to replace a published release or a draft from another commit.")
    else:
        subprocess.run([
            "gh", "release", "create", tag, "--verify-tag", "--target", sha,
            "--draft", "--title", tag, "--notes-file", str(root / "CHANGELOG.md"),
            *(["--prerelease"] if prerelease else []),
        ], check=True)
    # A failed upload leaves an unpublished draft. Re-running the failed job can resume it.
    subprocess.run(["gh", "release", "upload", tag, *map(str, files), "--clobber"], check=True)
    subprocess.run([
        "gh", "release", "edit", tag, "--draft=false",
        f"--prerelease={str(prerelease).lower()}", f"--latest={str(not prerelease).lower()}",
    ], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("prepare")
    prepare.add_argument("version")
    prepare.add_argument("--output", type=Path, required=True)
    collect = commands.add_parser("collect")
    collect.add_argument("source", type=Path)
    collect.add_argument("destination", type=Path)
    publish = commands.add_parser("publish")
    publish.add_argument("tag")
    publish.add_argument("sha")
    publish.add_argument("assets", type=Path)
    publish.add_argument("--prerelease", choices=("true", "false"), default="false")
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            tag = prepare_version(ROOT, args.version)
            with args.output.open("a") as output:
                output.write(f"tag={tag}\n")
        elif args.command == "collect":
            collect_assets(args.source, args.destination)
        else:
            publish_release(ROOT, args.tag, args.sha, args.assets, args.prerelease == "true")
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Release failed: {error}\n")


if __name__ == "__main__":
    main()
