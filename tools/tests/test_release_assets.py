from __future__ import annotations

import hashlib
import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "xcss_build_release_assets",
    ROOT / "scripts" / "build-release-assets.py",
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ReleaseAssetTests(unittest.TestCase):
    def test_release_rejects_other_targets_before_touching_source_or_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "release"
            for target in ("source-any", "x86_64-unknown-linux-musl", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc"):
                with self.subTest(target=target), self.assertRaisesRegex(MODULE.ReleaseBuildError, "must be x86_64-unknown-linux-gnu"):
                    MODULE.build(output, "a" * 40, f"v{MODULE.CURRENT_VERSION}", target)
                self.assertFalse(output.exists())

    def test_packaged_tool_and_package_cli_versions_match_the_release(self) -> None:
        from xcss_package_artifacts import CLI_VERSION as package_version
        from xcss_release.cli import CLI_VERSION as release_version
        self.assertEqual(package_version, MODULE.CURRENT_VERSION)
        self.assertEqual(release_version, MODULE.CURRENT_VERSION)

    def test_previous_tag_is_rejected_before_source_inspection(self) -> None:
        with self.assertRaisesRegex(MODULE.ReleaseBuildError, "tag must be exactly"):
            MODULE.verify_source("a" * 40, "v1.0.0")

    def test_state_contract_has_one_current_exact_shape(self) -> None:
        revision = "a" * 40
        contract = MODULE.state_contract(revision)
        self.assertEqual(
            set(contract),
            {
                "contract_version",
                "application",
                "application_version",
                "source_revision",
                "schema",
                "maintenance_locks",
                "resources",
                "external_requirements",
                "companion_contracts",
            },
        )
        self.assertEqual(contract["application"], "xcss")
        self.assertEqual(contract["application_version"], MODULE.CURRENT_VERSION)
        self.assertEqual(contract["contract_version"], 1)
        self.assertEqual(contract["source_revision"], revision)
        self.assertIsNone(contract["schema"])

    def test_tool_bundle_is_byte_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            first_path = MODULE.build_tool_bundle(Path(first))
            second_path = MODULE.build_tool_bundle(Path(second))
            self.assertEqual(
                hashlib.sha256(first_path.read_bytes()).digest(),
                hashlib.sha256(second_path.read_bytes()).digest(),
            )

    def test_release_output_must_be_empty(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "release"
            root.mkdir()
            (root / "existing").write_text("no\n", encoding="utf-8")
            with self.assertRaisesRegex(MODULE.ReleaseBuildError, "must be empty"):
                MODULE.prepare_output(root)


if __name__ == "__main__":
    unittest.main()
