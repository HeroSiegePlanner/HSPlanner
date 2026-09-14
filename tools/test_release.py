import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        manifest = tomllib.loads((release.ROOT / "Cargo.toml").read_text())
        for name in ["Cargo.toml", "Cargo.lock", "CHANGELOG.md"]:
            shutil.copyfile(release.ROOT / name, self.root / name)
        for member in manifest["workspace"]["members"]:
            (self.root / member).mkdir(parents=True)
            shutil.copyfile(release.ROOT / member / "Cargo.toml", self.root / member / "Cargo.toml")

    def installers(self):
        source = self.root / "downloads"
        for artifact, patterns in release.ARTIFACTS.items():
            directory = source / artifact
            directory.mkdir(parents=True)
            hashes = []
            for pattern in patterns:
                name = pattern.replace("*", "HSPlanner_1.2.3")
                path = directory / name
                path.write_bytes(f"test installer: {artifact}/{name}".encode())
                if name != "PKGBUILD":
                    hashes.append(f"{release.digest(path)}  {name}\n")
            (directory / "SHA256SUMS").write_text("".join(hashes))
        return source

    def publish(self, existing=None, upload_error=False, prerelease=False, ref_sha=None):
        release.prepare_version(self.root, "1.2.3")
        assets = self.root / "assets"
        release.collect_assets(self.installers(), assets)
        sha = "a" * 40
        responses = [
            json.dumps({"object": {"type": "commit", "sha": ref_sha or sha}}),
            json.dumps([[existing] if existing else []]),
        ]

        def run(command, **_):
            if upload_error and command[:3] == ["gh", "release", "upload"]:
                raise subprocess.CalledProcessError(1, command)

        with patch.dict("os.environ", {"GH_REPO": "owner/repo"}), \
                patch.object(release.subprocess, "check_output", side_effect=responses), \
                patch.object(release.subprocess, "run", side_effect=run) as commands:
            error = None
            try:
                release.publish_release(self.root, "v1.2.3", sha, assets, prerelease)
            except (ValueError, subprocess.CalledProcessError) as failure:
                error = failure
            return [call.args[0] for call in commands.call_args_list], error

    def test_version_updates_native_packages_without_updating_dependencies(self):
        before = tomllib.loads((self.root / "Cargo.lock").read_text())
        original = tomllib.loads((self.root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        version = "1.2.4" if original == "1.2.3" else "1.2.3"
        self.assertEqual(release.prepare_version(self.root, f"v{version}"), f"v{version}")
        manifest = tomllib.loads((self.root / "Cargo.toml").read_text())
        self.assertEqual(manifest["workspace"]["package"]["version"], version)
        after = tomllib.loads((self.root / "Cargo.lock").read_text())
        changed = []
        for old, new in zip(before["package"], after["package"], strict=True):
            if old != new:
                changed.append(new["name"])
                self.assertEqual(new, {**old, "version": version})
                self.assertNotIn("source", new)
        self.assertEqual(set(changed), {
            "hsplanner", "hsplanner-build", "hsplanner-library",
            "hsplanner-notes", "hsplanner-planner", "hsplanner-ui",
        })
        self.assertEqual(release.prepare_version(self.root, version), f"v{version}")

    def test_invalid_versions_do_not_modify_workspace(self):
        original = (self.root / "Cargo.toml").read_bytes()
        for value in ["", "v01.2.3", "1.2", "v1.2.3\ntag=other", "$(echo hi)", "1.2.3-beta", "65536.0.0"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                release.prepare_version(self.root, value)
        self.assertEqual((self.root / "Cargo.toml").read_bytes(), original)

    def test_inconsistent_lockfile_does_not_partially_bump_version(self):
        path = self.root / "Cargo.lock"
        text = path.read_text()
        old = tomllib.loads((self.root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        path.write_text(text.replace(f'name = "hsplanner"\nversion = "{old}"', 'name = "hsplanner"\nversion = "0.0.0"'))
        before = (self.root / "Cargo.toml").read_bytes()
        with self.assertRaisesRegex(ValueError, "Cargo.lock version differs"):
            release.prepare_version(self.root, "1.2.3")
        self.assertEqual((self.root / "Cargo.toml").read_bytes(), before)

    def test_collect_combines_platform_checksums_without_overwriting_them(self):
        destination = self.root / "assets"
        release.collect_assets(self.installers(), destination)
        checksums = (destination / "SHA256SUMS").read_text().splitlines()
        self.assertEqual(len(checksums), 6)
        for line in checksums:
            checksum, name = line.split("  ", 1)
            self.assertEqual(release.digest(destination / name), checksum)

    def test_damaged_or_missing_platform_prevents_collection(self):
        source = self.installers()
        destination = self.root / "assets"
        next((source / "hsplanner-Linux-X64").glob("*.AppImage")).write_bytes(b"corrupted")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            release.collect_assets(source, destination)
        self.assertFalse(destination.exists())
        shutil.rmtree(source / "hsplanner-Windows-X64")
        with self.assertRaises(OSError):
            release.collect_assets(source, destination)
        self.assertFalse(destination.exists())

    def test_publication_happens_after_upload_and_uses_changelog(self):
        commands, error = self.publish()
        self.assertIsNone(error)
        self.assertEqual([command[2] for command in commands], ["create", "upload", "edit"])
        self.assertIn("--draft", commands[0])
        self.assertIn(str(self.root / "CHANGELOG.md"), commands[0])
        self.assertIn("--draft=false", commands[-1])
        self.assertIn("--latest=true", commands[-1])

    def test_failed_upload_never_publishes(self):
        commands, error = self.publish(upload_error=True)
        self.assertIsInstance(error, subprocess.CalledProcessError)
        self.assertEqual([command[2] for command in commands], ["create", "upload"])

    def test_retry_resumes_only_its_own_draft_and_keeps_prerelease_off_latest(self):
        commands, error = self.publish(existing={"tag_name": "v1.2.3", "draft": True, "target_commitish": "a" * 40}, prerelease=True)
        self.assertIsNone(error)
        self.assertEqual([command[2] for command in commands], ["upload", "edit"])
        self.assertIn("--prerelease=true", commands[-1])
        self.assertIn("--latest=false", commands[-1])

    def test_existing_public_release_is_not_overwritten(self):
        commands, error = self.publish(existing={"tag_name": "v1.2.3", "draft": False, "target_commitish": "a" * 40})
        self.assertIsInstance(error, ValueError)
        self.assertEqual(commands, [])

    def test_moved_tag_is_not_published(self):
        commands, error = self.publish(ref_sha="b" * 40)
        self.assertIsInstance(error, ValueError)
        self.assertEqual(commands, [])


if __name__ == "__main__":
    unittest.main()
