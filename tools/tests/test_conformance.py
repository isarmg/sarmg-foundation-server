from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parents[1]
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))

from sarmg_conformance import (  # noqa: E402
    ConformanceError,
    generate_consumer_matrix,
    verify_foundation,
    verify_manifest,
    verify_source,
)


VALID_MANIFEST = """\
format = 1
product_id = "fixture-product"

[foundation]
platform_generation = 1
version = "0.4.0"
git_rev = "0123456789abcdef0123456789abcdef01234567"

[[components]]
id = "server"
profile = "server-filesystem"
http_adapter = "hyper"
web_profile = "web-embedded-native"
capabilities = [
  "admin-static",
  "memory-sessions",
  "server-runtime",
  "server-health",
  "filesystem-root",
  "linux-openat2",
]
"""


class ConformanceTests(unittest.TestCase):
    def test_repository_platform_definition_is_self_consistent(self) -> None:
        result = verify_foundation(ROOT)
        self.assertEqual(len(result["profiles"]), 7)
        self.assertIn("admin-persistent", result["capabilities"])
        generated = generate_consumer_matrix(ROOT)
        checked_in = json.loads((ROOT / "consumers" / "consumer-matrix.json").read_text())
        self.assertEqual(generated, checked_in)

    def test_manifest_requires_profile_capabilities_and_immutable_revision(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST, encoding="utf-8")
            self.assertEqual(verify_manifest(product, ROOT)["product_id"], "fixture-product")
            invalid = VALID_MANIFEST.replace("linux-openat2", "unknown-capability")
            (product / "sarmg-product.toml").write_text(invalid, encoding="utf-8")
            with self.assertRaisesRegex(ConformanceError, "missing required capabilities"):
                verify_manifest(product, ROOT)
            invalid = VALID_MANIFEST.replace(
                "0123456789abcdef0123456789abcdef01234567", "main"
            )
            (product / "sarmg-product.toml").write_text(invalid, encoding="utf-8")
            with self.assertRaisesRegex(ConformanceError, "40 lowercase hex"):
                verify_manifest(product, ROOT)

    def test_source_check_rejects_product_named_feature(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST, encoding="utf-8")
            (product / "Cargo.toml").write_text(
                '[package]\nname="fixture"\nversion="0.1.0"\n'
                '[features]\nsunshine-manager-mode=[]\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ConformanceError, "no-product-features"):
                verify_source(product, ROOT)

    def test_cli_reports_machine_readable_result(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "sarmg-conformance.py"), "verify-foundation"],
            check=True,
            capture_output=True,
            text=True,
        )
        result = json.loads(completed.stdout)
        self.assertIn("server-control-plane", result["profiles"])


if __name__ == "__main__":
    unittest.main()
