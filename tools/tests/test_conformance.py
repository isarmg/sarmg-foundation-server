from __future__ import annotations

import json
import re
import shutil
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
    verify_schema,
    verify_source,
)
from sarmg_schema_compose import compose  # noqa: E402


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
http_adapter = "axum"
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

    def test_consumer_registry_is_an_independent_reporting_contract(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory)
            shutil.copytree(ROOT / "profiles", fixture / "profiles")
            (fixture / "Cargo.toml").write_text(
                '[workspace]\nmembers=[]\n[workspace.package]\nversion="0.8.9"\n'
            )
            package = fixture / "rust" / "crates" / "sarmg-error"
            package.mkdir(parents=True)
            (package / "Cargo.toml").write_text(
                '[package]\nname="sarmg-error"\nversion="0.8.9"\n'
            )
            consumers = fixture / "consumers"
            consumers.mkdir()
            (consumers / "repositories.toml").write_text(
                '''format = 1
[[repositories]]
product = "new-product"
url = "https://github.com/example/new-product"
commit = "0123456789abcdef0123456789abcdef01234567"
foundation_version = "0.8.9"
profiles = ["offline-tool"]
capabilities = ["explicit-paths", "private-state", "restore-journal", "linux-openat2"]
packages = ["sarmg-error"]
status = "conforming"
exceptions = []
'''
            )
            generated = generate_consumer_matrix(fixture)
            self.assertEqual([item["product"] for item in generated["consumers"]], ["new-product"])

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

    def test_source_check_resolves_aliased_foundation_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST, encoding="utf-8")
            (product / "Cargo.toml").write_text(
                '[package]\nname="fixture"\nversion="0.1.0"\n'
                '[dependencies]\nlocal-error={package="sarmg-error",path="../foundation"}\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ConformanceError, "immutable-foundation-dependencies"):
                verify_source(product, ROOT)

    def test_filesystem_profile_accepts_native_and_react_web(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            for profile in ("web-embedded-native", "web-react-admin"):
                (product / "sarmg-product.toml").write_text(
                    VALID_MANIFEST.replace("web-embedded-native", profile), encoding="utf-8"
                )
                self.assertEqual(verify_manifest(product, ROOT)["components"][0]["web_profile"], profile)
            (product / "sarmg-product.toml").write_text(
                VALID_MANIFEST.replace("web-embedded-native", "offline-tool"), encoding="utf-8"
            )
            with self.assertRaisesRegex(ConformanceError, "web_profile is not allowed"):
                verify_manifest(product, ROOT)

    def test_client_profile_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST.replace("server-filesystem", "desktop-client"))
            with self.assertRaisesRegex(ConformanceError, "unknown Profile"):
                verify_manifest(product, ROOT)

    def test_server_checks_do_not_govern_client_web_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(VALID_MANIFEST)
            (product / "sarmg-client.toml").write_text('source_roots = ["client"]\n')
            client = product / "client"
            client.mkdir()
            source = '.route("/api/v2/auth/login", handler)\nconst SESSION_COOKIE_NAME = "local";'
            (client / "control.rs").write_text(source)
            verify_source(product, ROOT)
            (product / "server.rs").write_text(source)
            result = verify_source(product, ROOT)
            self.assertEqual(result["advisories"][0]["rule"], "foundation-route-ownership")

    def test_schema_is_discovered_from_declared_layout_and_recomposed(self) -> None:
        manifest = '''format = 1
product_id = "fixture-product"
[foundation]
platform_generation = 1
version = "0.5.0"
git_rev = "0123456789abcdef0123456789abcdef01234567"
[[components]]
id = "server"
profile = "server-control-plane"
http_adapter = "axum"
web_profile = "web-react-admin"
capabilities = ["platform-sqlite", "admin-persistent", "server-runtime", "server-health"]
'''
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "sarmg-product.toml").write_text(manifest)
            (product / "sarmg-layout.toml").write_text(
                'schema_product="db/source.sql"\nschema_generated="artifacts/schema.sql"\n'
            )
            source = product / "db" / "source.sql"
            generated = product / "artifacts" / "schema.sql"
            source.parent.mkdir()
            generated.parent.mkdir()
            source.write_text("CREATE TABLE fixture (id INTEGER PRIMARY KEY);\n")
            generated.write_text(
                compose(
                    "server-control-plane",
                    ["admin-persistent"],
                    source,
                )
            )
            self.assertEqual(verify_schema(product, ROOT)["status"], "verified")
            generated.write_text(generated.read_text() + "-- stale\n")
            with self.assertRaisesRegex(ConformanceError, "does not match recomposition"):
                verify_schema(product, ROOT)

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
