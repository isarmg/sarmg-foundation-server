from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


TOOLS = Path(__file__).resolve().parents[1]
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))

from sarmg_release import (  # noqa: E402
    BuildIdentity,
    PolicyError,
    build_manifest,
    parse_manifest,
    render_manifest,
    verify_release_tree,
)
import sarmg_release.release as release_module  # noqa: E402


IDENTITY = BuildIdentity(
    product="fixture-product",
    version="1.2.3-rc.1+build.7",
    source_revision="0123456789abcdef0123456789abcdef01234567",
    target="x86_64-unknown-linux-gnu",
    state_contract_sha256="89abcdef" * 8,
)


class ReleaseTreeTests(unittest.TestCase):
    def release_tree(self, root: Path) -> None:
        (root / "bin").mkdir()
        executable = root / "bin" / "fixture"
        executable.write_bytes(b"#!/bin/sh\nexit 0\n")
        executable.chmod(0o755)
        (root / "share").mkdir()
        (root / "share" / "config.json").write_text("{}\n", encoding="utf-8")

    def test_manifest_round_trip_and_exact_verification(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.release_tree(root)
            manifest = build_manifest(root, IDENTITY)
            encoded = render_manifest(manifest)
            self.assertEqual(encoded, render_manifest(parse_manifest(encoded)))
            verify_release_tree(root, parse_manifest(encoded))
            self.assertEqual(
                [entry.path for entry in manifest.files],
                ["bin/fixture", "share/config.json"],
            )

    def test_changed_and_unlisted_files_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.release_tree(root)
            manifest = build_manifest(root, IDENTITY)
            (root / "share" / "config.json").write_text('{"changed":true}\n', encoding="utf-8")
            with self.assertRaisesRegex(PolicyError, "(size|sha256) mismatch"):
                verify_release_tree(root, manifest)
            (root / "share" / "config.json").write_text("{}\n", encoding="utf-8")
            (root / "unexpected").write_bytes(b"no\n")
            with self.assertRaisesRegex(PolicyError, "unexpected"):
                verify_release_tree(root, manifest)

    def test_mode_change_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.release_tree(root)
            manifest = build_manifest(root, IDENTITY)
            (root / "bin" / "fixture").chmod(0o644)
            with self.assertRaisesRegex(PolicyError, "mode mismatch"):
                verify_release_tree(root, manifest)

    def test_symbolic_and_hard_links_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "file").write_bytes(b"content")
            os.symlink("file", root / "symlink")
            with self.assertRaisesRegex(PolicyError, "symbolic links"):
                build_manifest(root, IDENTITY)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "file").write_bytes(b"content")
            os.link(root / "file", root / "hardlink")
            with self.assertRaisesRegex(PolicyError, "hard-linked"):
                build_manifest(root, IDENTITY)

    def test_path_replacement_during_hash_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "artifact"
            moved = Path(directory) / "original"
            path.write_bytes(b"same-size-content")
            expected = path.lstat()
            original_read = os.read
            replaced = False

            def replace_after_read(descriptor: int, size: int) -> bytes:
                nonlocal replaced
                chunk = original_read(descriptor, size)
                if chunk and not replaced:
                    replaced = True
                    path.rename(moved)
                    path.write_bytes(b"different-content")
                return chunk

            with mock.patch.object(release_module.os, "read", side_effect=replace_after_read):
                with self.assertRaisesRegex(PolicyError, "(changed while it was hashed|path was replaced)"):
                    release_module._hash_regular_file(path, expected)

    def test_final_path_identity_is_rechecked_after_hashing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "artifact"
            path.write_bytes(b"content")
            expected = path.lstat()
            replacement = SimpleNamespace(
                st_mode=expected.st_mode,
                st_nlink=1,
                st_dev=expected.st_dev,
                st_ino=expected.st_ino + 1,
                st_size=expected.st_size,
            )
            with mock.patch.object(Path, "lstat", return_value=replacement):
                with self.assertRaisesRegex(PolicyError, "path was replaced"):
                    release_module._hash_regular_file(path, expected)

    def test_file_growth_during_hash_is_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "artifact"
            path.write_bytes(b"abc")
            expected = path.lstat()
            original_read = os.read
            grown = False

            def grow_before_read(descriptor: int, size: int) -> bytes:
                nonlocal grown
                if not grown:
                    grown = True
                    with path.open("ab") as output:
                        output.write(b"d")
                return original_read(descriptor, size)

            with (
                mock.patch.object(release_module, "MAX_FILE_BYTES", 3),
                mock.patch.object(release_module.os, "read", side_effect=grow_before_read),
            ):
                with self.assertRaisesRegex(PolicyError, "grew beyond"):
                    release_module._hash_regular_file(path, expected)

    def test_tree_limits_fail_before_hashing_excess_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index in range(3):
                (root / f"{index}.txt").write_bytes(str(index).encode("ascii"))
            with (
                mock.patch.object(release_module, "MAX_FILES", 2),
                mock.patch.object(
                    release_module,
                    "_hash_regular_file",
                    wraps=release_module._hash_regular_file,
                ) as hasher,
            ):
                with self.assertRaisesRegex(PolicyError, "2-file policy limit"):
                    release_module.scan_release_tree(root)
                self.assertEqual(hasher.call_count, 2)

    def test_total_entry_limit_is_fail_fast(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index in range(3):
                (root / f"directory-{index}").mkdir()
            with mock.patch.object(release_module, "MAX_TREE_ENTRIES", 2):
                with self.assertRaisesRegex(PolicyError, "2-entry policy limit"):
                    release_module.scan_release_tree(root)

    def test_manifest_shape_and_values_are_strict(self) -> None:
        base = {
            "format": "sarmg.release-tree.v1",
            "identity": IDENTITY.to_dict(),
            "files": [
                {
                    "path": "bin/app",
                    "mode": "0755",
                    "size": 1,
                    "sha256": "0" * 64,
                }
            ],
        }
        invalid = []
        for mutation in (
            {"unknown": True},
            {"format": "sarmg.release-tree.v0"},
        ):
            value = json.loads(json.dumps(base))
            value.update(mutation)
            invalid.append(value)
        for path in ("/bin/app", "../bin/app", "bin\\app", "bin/./app"):
            value = json.loads(json.dumps(base))
            value["files"][0]["path"] = path
            invalid.append(value)
        value = json.loads(json.dumps(base))
        value["files"][0]["size"] = True
        invalid.append(value)
        value = json.loads(json.dumps(base))
        value["identity"]["source_revision"] = "main"
        invalid.append(value)
        value = json.loads(json.dumps(base))
        value["files"][0]["extra"] = "forbidden"
        invalid.append(value)
        for case in invalid:
            with self.subTest(case=case), self.assertRaises(PolicyError):
                parse_manifest(json.dumps(case))

        duplicate = '{"format":"a","format":"b","identity":{},"files":[]}'
        with self.assertRaisesRegex(PolicyError, "duplicate object key"):
            parse_manifest(duplicate)

    def test_cli_version_and_create_verify(self) -> None:
        launcher = ROOT / "scripts" / "sarmg-release.py"
        version = subprocess.run(
            [sys.executable, str(launcher), "--version"],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(version.stdout.strip(), "sarmg-release 0.3.0")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "tree"
            root.mkdir()
            (root / "app").write_bytes(b"app")
            create = subprocess.run(
                [
                    sys.executable,
                    str(launcher),
                    "create",
                    str(root),
                    "--product",
                    IDENTITY.product,
                    "--release-version",
                    IDENTITY.version,
                    "--source-revision",
                    IDENTITY.source_revision,
                    "--target",
                    IDENTITY.target,
                    "--state-contract-sha256",
                    IDENTITY.state_contract_sha256,
                ],
                check=True,
                capture_output=True,
            )
            manifest = Path(directory) / "manifest.json"
            manifest.write_bytes(create.stdout)
            subprocess.run(
                [sys.executable, str(launcher), "verify", str(root), str(manifest)],
                check=True,
                capture_output=True,
                text=True,
            )


if __name__ == "__main__":
    unittest.main()
