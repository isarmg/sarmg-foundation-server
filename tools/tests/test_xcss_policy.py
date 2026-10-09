from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

from xcss_policy import (  # noqa: E402
    CURRENT_VERSION,
    XcssPolicyError,
    check_dependency_boundaries,
    check_server_dependency_targets,
    check_web_lockfile,
)


class DependencyBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.cargo = self.root / "Cargo.toml"
        self.cargo.write_text("[workspace]\nmembers = []\n")
        self.package = self.root / "package.json"
        self.package.write_text("{}")
        self.crate = self.root / "src/error/Cargo.toml"
        self.crate.parent.mkdir(parents=True)
        self.crate.write_text("[package]\nname = 'xcss-error'\n")

    def test_repository_has_no_downstream_package_dependencies(self) -> None:
        check_dependency_boundaries(TOOLS.parent)

    def test_rust_aliases_and_target_scopes_cannot_import_downstream_packages(self) -> None:
        for scope in ("dependencies", "dev-dependencies", "build-dependencies", "target.'cfg(unix)'.dependencies"):
            with self.subTest(scope=scope):
                self.crate.write_text(f"[{scope}]\nlocal = {{ package = 'xcsc-core', version = '1' }}\n")
                with self.assertRaisesRegex(XcssPolicyError, "package boundary"):
                    check_dependency_boundaries(self.root)

    def test_rust_self_dependencies_and_independent_internal_aliases_are_rejected(self) -> None:
        for package in ("xcss", "xcss-error"):
            self.crate.write_text(f"[dependencies]\nlocal = {{ package = '{package}', path = '.', version = '={CURRENT_VERSION}' }}\n")
            with self.assertRaisesRegex(XcssPolicyError, "package boundary"):
                check_dependency_boundaries(self.root)

    def test_rust_workspace_and_patch_dependencies_obey_the_same_boundary(self) -> None:
        for scope in ("workspace.dependencies", "patch.crates-io", "replace"):
            with self.subTest(scope=scope):
                self.cargo.write_text(f"[{scope}]\nlocal = {{ package = 'xcss-product', path = '../product' }}\n")
                with self.assertRaisesRegex(XcssPolicyError, "package boundary"):
                    check_dependency_boundaries(self.root)

    def test_rust_external_path_cannot_reach_a_consumer(self) -> None:
        self.crate.write_text("[dependencies]\nproduct = { path = '../../../../product' }\n")
        with self.assertRaisesRegex(XcssPolicyError, "external local dependency"):
            check_dependency_boundaries(self.root)

    def test_inherited_rust_dependencies_cannot_bypass_the_monolith(self) -> None:
        self.cargo.write_text(f"[workspace.dependencies]\nlocal = {{ package = 'xcss', path = '.', version = '={CURRENT_VERSION}' }}\n")
        self.crate.write_text("[dependencies]\nlocal = { workspace = true }\n")
        with self.assertRaisesRegex(XcssPolicyError, "package boundary"):
            check_dependency_boundaries(self.root)

    def test_web_root_and_package_aliases_cannot_import_downstream(self) -> None:
        child = self.root / "web/admin-ui/package.json"
        child.parent.mkdir(parents=True)
        child.write_text("{}")
        for path in (self.package, child):
            for scope in ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies"):
                for name, requirement in (
                    ("@xcss/client-core", "1.0.0"),
                    ("alias", "npm:@xcss/client-core@1.0.0"),
                    ("alias", "npm:@xcss/client-core"),
                    ("product", "file:../product"),
                    ("product", "link:../product"),
                ):
                    with self.subTest(path=path, scope=scope, requirement=requirement):
                        path.write_text(json.dumps({scope: {name: requirement}}))
                        with self.assertRaises(XcssPolicyError):
                            check_dependency_boundaries(self.root)
                        path.write_text("{}")

    def test_web_self_dependencies_are_rejected_and_external_libraries_allowed(self) -> None:
        self.package.write_text(json.dumps({"dependencies": {"react": "19.3.0"}}))
        check_dependency_boundaries(self.root)
        for name in ("@xcss/web", "@xcss/admin-ui"):
            self.package.write_text(json.dumps({"dependencies": {name: CURRENT_VERSION}}))
            with self.assertRaisesRegex(XcssPolicyError, "package boundary"):
                check_dependency_boundaries(self.root)


class CleanBuildDependencyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def test_current_server_dependencies_and_clean_build_lock_are_complete(self) -> None:
        check_server_dependency_targets(TOOLS.parent)
        check_web_lockfile(TOOLS.parent)

    def test_external_rust_dependencies_cannot_run_before_the_server_gate(self) -> None:
        canonical = (TOOLS.parent / "Cargo.toml").read_text()
        for source in (
            canonical.replace("[target.'cfg(all(target_os = \"linux\", target_arch = \"x86_64\", target_env = \"gnu\"))'.dependencies]", "[dependencies]"),
            canonical + "\n[target.'cfg(unix)'.dependencies]\nlibc = '0.2'\n",
        ):
            (self.root / "Cargo.toml").write_text(source)
            with self.assertRaisesRegex(XcssPolicyError, "scoped to the canonical"):
                check_server_dependency_targets(self.root)

    def test_removed_web_importers_cannot_survive_in_the_lockfile(self) -> None:
        lock = (TOOLS.parent / "pnpm-lock.yaml").read_text()
        (self.root / "pnpm-lock.yaml").write_text(lock.replace("\npackages:\n", "\n  packages/admin-ui: {}\n\npackages:\n", 1))
        with self.assertRaisesRegex(XcssPolicyError, "root Web package importer"):
            check_web_lockfile(self.root)

    def test_missing_native_binary_metadata_is_rejected_without_node_modules(self) -> None:
        lock = (TOOLS.parent / "pnpm-lock.yaml").read_text()
        for declaration in (
            "      '@typescript/typescript-linux-x64': 7.0.2\n",
            "      '@rolldown/binding-linux-x64-gnu': 1.2.12\n",
            "      lightningcss-linux-x64-gnu: 1.33.0\n",
        ):
            self.assertIn(declaration, lock)
            (self.root / "pnpm-lock.yaml").write_text(lock.replace(declaration, ""))
            with self.assertRaisesRegex(XcssPolicyError, "locked native dependencies"):
                check_web_lockfile(self.root)


if __name__ == "__main__":
    unittest.main()
