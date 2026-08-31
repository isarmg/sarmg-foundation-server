from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

from sarmg_package_artifacts import (  # noqa: E402
    PackagePolicyError,
    clean_dist,
    discover_packages,
)


class PackageArtifactTests(unittest.TestCase):
    def fixture(self, repository: Path) -> Path:
        package = repository / "packages" / "fixture"
        (package / "dist").mkdir(parents=True)
        (package / "dist" / "index.js").write_text("export {};\n", encoding="utf-8")
        (package / "dist" / "index.d.ts").write_text("export {};\n", encoding="utf-8")
        (package / "package.json").write_text(
            json.dumps(
                {
                    "name": "@sarmg/fixture",
                    "version": "1.0.0",
                    "type": "module",
                    "main": "./dist/index.js",
                    "types": "./dist/index.d.ts",
                    "exports": {
                        ".": {
                            "types": "./dist/index.d.ts",
                            "import": "./dist/index.js",
                        }
                    },
                    "files": ["dist"],
                }
            )
            + "\n",
            encoding="utf-8",
        )
        return package

    def test_manifest_build_and_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            package = self.fixture(repository)
            packages = discover_packages(repository)
            self.assertEqual(len(packages[0].check_build()), 2)
            clean_dist(packages)
            self.assertFalse((package / "dist").exists())

    def test_export_outside_dist_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            package = self.fixture(repository)
            manifest = json.loads((package / "package.json").read_text(encoding="utf-8"))
            manifest["main"] = "./src/index.js"
            (package / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(PackagePolicyError, "start with ./dist"):
                discover_packages(repository)

    def test_development_workspace_ranges_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            package = self.fixture(repository)
            manifest = json.loads((package / "package.json").read_text(encoding="utf-8"))
            manifest["devDependencies"] = {"@sarmg/fixture": "workspace:*"}
            (package / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(PackagePolicyError, "workspace:1.0.0"):
                discover_packages(repository)

    def test_linked_build_artifacts_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            package = self.fixture(repository)
            os.symlink("index.js", package / "dist" / "linked.js")
            with self.assertRaisesRegex(PackagePolicyError, "regular, unlinked"):
                discover_packages(repository)[0].check_build()
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            package = self.fixture(repository)
            os.link(package / "dist" / "index.js", package / "dist" / "hard.js")
            with self.assertRaisesRegex(PackagePolicyError, "regular, unlinked"):
                discover_packages(repository)[0].check_build()

    def test_clean_refuses_a_linked_dist_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            package = self.fixture(repository)
            real = repository / "real-dist"
            (package / "dist").rename(real)
            os.symlink(real, package / "dist")
            packages = discover_packages(repository)
            with self.assertRaisesRegex(PackagePolicyError, "refusing to remove"):
                clean_dist(packages)
            self.assertTrue((real / "index.js").exists())

    def test_linked_package_root_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            external = repository / "external"
            self.fixture(external)
            packages = repository / "packages"
            packages.mkdir()
            os.symlink(external / "packages" / "fixture", packages / "fixture")
            with self.assertRaisesRegex(PackagePolicyError, "linked package roots"):
                discover_packages(repository)


if __name__ == "__main__":
    unittest.main()
