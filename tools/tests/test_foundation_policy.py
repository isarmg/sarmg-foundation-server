from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

from foundation_policy import (  # noqa: E402
    CURRENT_VERSION,
    FoundationPolicyError,
    check_dependency_boundaries,
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
        self.crate = self.root / "rust/crates/sarmg-error/Cargo.toml"
        self.crate.parent.mkdir(parents=True)
        self.crate.write_text("[package]\nname = 'sarmg-error'\n")

    def test_repository_has_no_downstream_package_dependencies(self) -> None:
        check_dependency_boundaries(TOOLS.parent)

    def test_rust_aliases_and_target_scopes_cannot_import_downstream_packages(self) -> None:
        for scope in ("dependencies", "dev-dependencies", "build-dependencies", "target.'cfg(unix)'.dependencies"):
            with self.subTest(scope=scope):
                self.crate.write_text(f"[{scope}]\nlocal = {{ package = 'sarmg-client-core', version = '1' }}\n")
                with self.assertRaisesRegex(FoundationPolicyError, "outside Foundation Server"):
                    check_dependency_boundaries(self.root)

    def test_rust_internal_alias_requires_the_owned_crate_path(self) -> None:
        self.crate.write_text(
            f"[dependencies]\nlocal = {{ package = 'sarmg-error', path = '.', version = '={CURRENT_VERSION}' }}\n"
        )
        check_dependency_boundaries(self.root)
        self.crate.write_text(self.crate.read_text().replace("path = '.'", "path = '../../../../product'"))
        with self.assertRaisesRegex(FoundationPolicyError, "workspace crate path"):
            check_dependency_boundaries(self.root)

    def test_rust_workspace_and_patch_dependencies_obey_the_same_boundary(self) -> None:
        for scope in ("workspace.dependencies", "patch.crates-io", "replace"):
            with self.subTest(scope=scope):
                self.cargo.write_text(f"[{scope}]\nlocal = {{ package = 'sarmg-product', path = '../product' }}\n")
                with self.assertRaisesRegex(FoundationPolicyError, "outside Foundation Server"):
                    check_dependency_boundaries(self.root)

    def test_rust_external_path_cannot_reach_a_consumer(self) -> None:
        self.crate.write_text("[dependencies]\nproduct = { path = '../../../../product' }\n")
        with self.assertRaisesRegex(FoundationPolicyError, "external local dependency"):
            check_dependency_boundaries(self.root)

    def test_rust_workspace_paths_resolve_from_the_workspace(self) -> None:
        self.cargo.write_text(
            f"[workspace.dependencies]\nlocal = {{ package = 'sarmg-error', path = 'rust/crates/sarmg-error', version = '={CURRENT_VERSION}' }}\n"
        )
        self.crate.write_text("[dependencies]\nlocal = { workspace = true }\n")
        check_dependency_boundaries(self.root)

    def test_web_root_and_package_aliases_cannot_import_downstream(self) -> None:
        child = self.root / "packages/admin-ui/package.json"
        child.parent.mkdir(parents=True)
        child.write_text("{}")
        for path in (self.package, child):
            for scope in ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies"):
                for name, requirement in (
                    ("@sarmg/client-core", "1.0.0"),
                    ("alias", "npm:@sarmg/client-core@1.0.0"),
                    ("alias", "npm:@sarmg/client-core"),
                    ("product", "file:../product"),
                    ("product", "link:../product"),
                ):
                    with self.subTest(path=path, scope=scope, requirement=requirement):
                        path.write_text(json.dumps({scope: {name: requirement}}))
                        with self.assertRaises(FoundationPolicyError):
                            check_dependency_boundaries(self.root)
                        path.write_text("{}")

    def test_web_internal_packages_use_the_workspace_and_external_libraries_are_allowed(self) -> None:
        self.package.write_text(json.dumps({"dependencies": {
            "@sarmg/admin-ui": f"workspace:{CURRENT_VERSION}", "react": "19.2.8",
        }}))
        check_dependency_boundaries(self.root)
        self.package.write_text(json.dumps({"dependencies": {"@sarmg/admin-ui": f"^{CURRENT_VERSION}"}}))
        with self.assertRaisesRegex(FoundationPolicyError, "workspace:"):
            check_dependency_boundaries(self.root)


if __name__ == "__main__":
    unittest.main()
