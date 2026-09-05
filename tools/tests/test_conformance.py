from __future__ import annotations

import json
import re
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
    verify_baselines,
    verify_foundation,
    verify_manifest,
    verify_source,
)


VALID_MANIFEST = """\
format = 1
product_id = "fixture-product"

[foundation]
platform_generation = 1
version = "0.5.0"
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
    def test_manifest_schema_accepts_the_current_version(self) -> None:
        schema = json.loads((ROOT / "schemas/sarmg-product.schema.json").read_text())
        pattern = schema["properties"]["foundation"]["properties"]["version"]["pattern"]
        self.assertIsNotNone(re.fullmatch(pattern, "0.5.0"))
        self.assertIsNone(re.fullmatch(pattern, "0x5x0"))

    def test_repository_platform_definition_is_self_consistent(self) -> None:
        result = verify_foundation(ROOT)
        self.assertEqual(len(result["profiles"]), 5)
        self.assertIn("admin-persistent", result["capabilities"])
        generated = generate_consumer_matrix(ROOT)
        checked_in = json.loads((ROOT / "consumers" / "consumer-matrix.json").read_text())
        self.assertEqual(generated, checked_in)
        baselines = verify_baselines(ROOT)
        consumers = {entry["product"]: entry for entry in generated["consumers"]}
        self.assertNotEqual(
            baselines["sarmg-upgrade"]["source_commit"],
            consumers["sarmg-upgrade"]["commit"],
        )
        self.assertEqual(baselines["sarmg-upgrade"]["foundation_version"], "0.3.0")
        self.assertEqual(consumers["sarmg-upgrade"]["foundation_version"], "0.6.0")
        self.assertEqual(consumers["sarmg-upgrade"]["status"], "migration-in-progress")

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

    def test_agent_profile_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST.replace("server-filesystem", "desktop-agent"))
            with self.assertRaisesRegex(ConformanceError, "unknown Profile"):
                verify_manifest(product, ROOT)

    def test_server_checks_do_not_govern_client_web_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST)
            (product / "sarmg-agent.toml").write_text('source_roots = ["client"]\n')
            client = product / "client"
            client.mkdir()
            source = '.route("/api/v2/auth/login", handler)\nconst SESSION_COOKIE_NAME = "local";'
            (client / "control.rs").write_text(source)
            verify_source(product, ROOT)
            (product / "server.rs").write_text(source)
            with self.assertRaisesRegex(ConformanceError, "foundation-route-ownership"):
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
