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

from xcss_conformance import (  # noqa: E402
    ConformanceError,
    generate_consumer_matrix,
    verify_foundation,
    verify_manifest,
    verify_release,
    verify_schema,
    verify_source,
    verify_web,
)
from xcss_schema_compose import compose  # noqa: E402


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
    def embedded_fixture(self, product: Path) -> None:
        manifest = VALID_MANIFEST.replace('version = "0.5.0"', 'version = "0.10.0"').replace(
            '  "linux-openat2",', '  "linux-openat2",\n  "embedded-web",'
        )
        (product / "xcss-product.toml").write_text(manifest)
        (product / "xcss-layout.toml").write_text('web_root="web"\n')
        web = product / "web"
        web.mkdir()
        (web / "package.json").write_text(json.dumps({"name": "fixture-web", "scripts": {"build": "vite build"}}))
        (product / "xcss-web-build.json").write_text(json.dumps({
            "format": 1,
            "web": {"directory": "web", "script": "build", "dist": "dist"},
            "rust": {"manifest": "Cargo.toml", "package": "fixture", "binary": "fixture", "source_revision_env": "SOURCE_REVISION"},
        }))
        dependency = '{git="https://github.com/isarmg/xcss.git",rev="0123456789abcdef0123456789abcdef01234567",version="=0.10.0"}'
        (product / "Cargo.toml").write_text(
            '[package]\nname="fixture"\nversion="0.1.0"\n'
            f'[dependencies]\nxcss={dependency}\n'
            f'[build-dependencies]\nxcss={dependency}\n'
        )
        (product / "Cargo.lock").write_text(
            'version=4\n[[package]]\nname="xcss"\nversion="0.10.0"\n'
            'source="git+https://github.com/isarmg/xcss.git?rev=0123456789abcdef0123456789abcdef01234567#0123456789abcdef0123456789abcdef01234567"\n'
        )

    def test_current_web_requires_declared_shared_capability(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST.replace('version = "0.5.0"', 'version = "0.10.0"'))
            with self.assertRaisesRegex(ConformanceError, "requires embedded-web capability"):
                verify_manifest(product, ROOT)
            # Older immutable generations remain inspectable under their own contract.
            (product / "xcss-product.toml").write_text(VALID_MANIFEST)
            verify_manifest(product, ROOT)
            verify_source(product, ROOT)

    def test_shared_web_build_is_checked_for_source_and_web_reports(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            self.embedded_fixture(product)
            verify_source(product, ROOT)
            result = verify_web(product, ROOT)
            self.assertEqual(result["build"]["mode"], "embedded")
            (product / "xcss-web-build.json").unlink()
            for check in (verify_source, verify_web):
                with self.assertRaisesRegex(ConformanceError, "requires xcss-web-build.json"):
                    check(product, ROOT)

    def test_build_declaration_rejects_path_escape_symlinks_and_source_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            self.embedded_fixture(product)
            path = product / "xcss-web-build.json"
            valid = json.loads(path.read_text())
            for section, key, invalid in (
                ("web", "directory", "../outside"),
                ("web", "dist", "../Cargo.toml"),
                ("rust", "manifest", "/tmp/Cargo.toml"),
                ("web", "dist", "C:\\outside"),
                ("web", "dist", "."),
            ):
                config = json.loads(json.dumps(valid))
                config[section][key] = invalid
                path.write_text(json.dumps(config))
                with self.subTest(section=section, key=key, value=invalid), self.assertRaises(ConformanceError):
                    verify_web(product, ROOT)
            path.write_text(json.dumps(valid))
            with tempfile.TemporaryDirectory() as outside:
                (product / "web" / "dist").symlink_to(outside, target_is_directory=True)
                with self.assertRaisesRegex(ConformanceError, "escapes|linked"):
                    verify_web(product, ROOT)

    def test_shared_web_requires_runtime_and_build_crate_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            self.embedded_fixture(product)
            cargo = product / "Cargo.toml"
            original = cargo.read_text()
            for section in ("dependencies", "build-dependencies"):
                # Renaming the dependency removes only this semantic dependency table.
                cargo.write_text(original.replace(f"[{section}]", f"[metadata.{section}]"))
                with self.subTest(section=section), self.assertRaisesRegex(ConformanceError, f"requires xcss::web_assets.* in {section}"):
                    verify_source(product, ROOT)

    def test_manifest_schema_accepts_the_current_version(self) -> None:
        schema = json.loads((ROOT / "schemas/xcss-product.schema.json").read_text())
        pattern = schema["properties"]["foundation"]["properties"]["version"]["pattern"]
        self.assertIsNotNone(re.fullmatch(pattern, "0.5.0"))
        self.assertIsNone(re.fullmatch(pattern, "0x5x0"))

    def test_repository_platform_definition_is_self_consistent(self) -> None:
        result = verify_foundation(ROOT)
        self.assertEqual(len(result["profiles"]), 4)
        self.assertIn("admin-persistent", result["capabilities"])

    def test_consumer_registry_is_an_independent_reporting_contract(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory)
            shutil.copytree(ROOT / "profiles", fixture / "profiles")
            (fixture / "Cargo.toml").write_text(
                '[package]\nname="xcss"\nversion="0.9.0"\n'
            )
            (fixture / "package.json").write_text('{"name":"@xcss/web","version":"0.9.0"}')
            consumers = fixture / "consumers"
            consumers.mkdir()
            (consumers / "repositories.toml").write_text(
                '''format = 1
[[repositories]]
product = "new-product"
url = "https://github.com/example/new-product"
commit = "0123456789abcdef0123456789abcdef01234567"
xcss_version = "0.9.0"
profiles = ["server-filesystem"]
capabilities = ["admin-static", "memory-sessions", "server-runtime", "server-health", "filesystem-root", "linux-openat2"]
packages = ["xcss"]
status = "conforming"
exceptions = []
'''
            )
            generated = generate_consumer_matrix(fixture)
            self.assertEqual([item["product"] for item in generated["consumers"]], ["new-product"])

    def test_manifest_requires_profile_capabilities_and_immutable_revision(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST, encoding="utf-8")
            self.assertEqual(verify_manifest(product, ROOT)["product_id"], "fixture-product")
            invalid = VALID_MANIFEST.replace("linux-openat2", "unknown-capability")
            (product / "xcss-product.toml").write_text(invalid, encoding="utf-8")
            with self.assertRaisesRegex(ConformanceError, "missing required capabilities"):
                verify_manifest(product, ROOT)
            invalid = VALID_MANIFEST.replace(
                "0123456789abcdef0123456789abcdef01234567", "main"
            )
            (product / "xcss-product.toml").write_text(invalid, encoding="utf-8")
            with self.assertRaisesRegex(ConformanceError, "40 lowercase hex"):
                verify_manifest(product, ROOT)

    def test_source_check_resolves_aliased_foundation_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST, encoding="utf-8")
            (product / "Cargo.toml").write_text(
                '[package]\nname="fixture"\nversion="0.1.0"\n'
                '[dependencies]\nlocal-error={package="xcss",path="../foundation"}\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ConformanceError, "immutable-foundation-dependencies"):
                verify_source(product, ROOT)

    def test_source_check_rejects_unpinned_foundation_dependency_forms(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST)
            (product / "Cargo.toml").write_text(
                '[package]\nname="fixture"\nversion="0.1.0"\n'
                '[dependencies]\nxcss="0.5"\n'
            )
            with self.assertRaisesRegex(ConformanceError, "exact xcss Git source"):
                verify_source(product, ROOT)

    def test_removed_independent_packages_cannot_silently_escape_source_checks(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST)
            (product / "Cargo.toml").write_text('[package]\nname="fixture"\nversion="0.1.0"\n[dependencies]\nold={package="xcss-error",version="1.0"}\n')
            with self.assertRaisesRegex(ConformanceError, "independent xcss-error was removed"):
                verify_source(product, ROOT)
            (product / "Cargo.toml").unlink()
            (product / "package.json").write_text(json.dumps({"dependencies": {"@xcss/admin-ui": "1.0.0"}}))
            with self.assertRaisesRegex(ConformanceError, "independent @xcss/admin-ui was removed"):
                verify_source(product, ROOT)

    def test_release_check_distinguishes_source_inspection_from_publication_gate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST)
            self.assertEqual(verify_release(product, ROOT)["status"], "not-checked")
            with self.assertRaisesRegex(ConformanceError, "publication gate"):
                verify_release(product, ROOT, require_published=True)
            (product / "release.json").write_text('{"application":"fixture-product"}')
            with self.assertRaisesRegex(ConformanceError, "target is not canonical"):
                verify_release(product, ROOT)

    def test_filesystem_profile_accepts_native_and_react_web(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            for profile in ("web-embedded-native", "web-react-admin"):
                (product / "xcss-product.toml").write_text(
                    VALID_MANIFEST.replace("web-embedded-native", profile), encoding="utf-8"
                )
                self.assertEqual(verify_manifest(product, ROOT)["components"][0]["web_profile"], profile)
            (product / "xcss-product.toml").write_text(
                VALID_MANIFEST.replace("web-embedded-native", "offline-tool"), encoding="utf-8"
            )
            with self.assertRaisesRegex(ConformanceError, "web_profile is not allowed"):
                verify_manifest(product, ROOT)

    def test_offline_client_profile_is_not_a_server_consumption_entry(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST.replace("server-filesystem", "offline-tool"))
            with self.assertRaisesRegex(ConformanceError, "unknown Profile"):
                verify_manifest(product, ROOT)

    def test_unknown_server_profile_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST.replace("server-filesystem", "unknown-profile"))
            with self.assertRaisesRegex(ConformanceError, "unknown Profile"):
                verify_manifest(product, ROOT)

    def test_server_checks_cover_nested_server_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            product = Path(directory)
            (product / "xcss-product.toml").write_text(VALID_MANIFEST)
            server = product / "server"
            server.mkdir()
            source = '.route("/api/v1/auth/login", handler)\nconst SESSION_COOKIE_NAME = "local";'
            (server / "control.rs").write_text(source)
            result = verify_source(product, ROOT)
            self.assertEqual(result["advisories"][0]["rule"], "foundation-route-ownership")
            self.assertEqual(result["advisories"][0]["path"], str(server / "control.rs"))

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
            (product / "xcss-product.toml").write_text(manifest)
            (product / "xcss-layout.toml").write_text(
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
            [sys.executable, str(ROOT / "scripts" / "xcss-conformance.py"), "verify-xcss"],
            check=True,
            capture_output=True,
            text=True,
        )
        result = json.loads(completed.stdout)
        self.assertIn("server-control-plane", result["profiles"])


if __name__ == "__main__":
    unittest.main()
