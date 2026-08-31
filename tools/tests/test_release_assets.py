from __future__ import annotations

import hashlib
import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "sarmg_build_release_assets",
    ROOT / "scripts" / "build-release-assets.py",
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ReleaseAssetTests(unittest.TestCase):
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
        self.assertEqual(contract["application"], "sarmg-foundation")
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
