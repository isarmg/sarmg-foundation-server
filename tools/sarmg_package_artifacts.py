"""Clean, inspect, pack, and install-smoke the publishable TypeScript packages."""

from __future__ import annotations

import json
import os
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


CLI_VERSION = "0.4.0"
MAX_MANIFEST_BYTES = 1024 * 1024
MAX_TARBALL_BYTES = 64 * 1024 * 1024
PACKAGE_NAME = re.compile(r"@sarmg/[a-z][a-z0-9-]*")
SEMVER = re.compile(
    r"(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


class PackagePolicyError(RuntimeError):
    """A package manifest, build directory, or tarball violates policy."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, entry in pairs:
        if key in value:
            raise PackagePolicyError(f"duplicate JSON object key {key!r}")
        value[key] = entry
    return value


def _read_json(path: Path) -> dict[str, Any]:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise PackagePolicyError(f"{path}: must be a regular, unlinked JSON file")
    data = path.read_bytes()
    if len(data) > MAX_MANIFEST_BYTES:
        raise PackagePolicyError(f"{path}: exceeds the 1 MiB manifest limit")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PackagePolicyError(f"{path}: invalid UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise PackagePolicyError(f"{path}: expected one JSON object")
    return value


def _relative_target(value: Any, context: str) -> str:
    if not isinstance(value, str) or not value.startswith("./dist/"):
        raise PackagePolicyError(f"{context}: target must start with ./dist/")
    if "\\" in value or "\x00" in value:
        raise PackagePolicyError(f"{context}: target contains a forbidden character")
    path = PurePosixPath(value[2:])
    if any(part in ("", ".", "..") for part in path.parts) or path.as_posix() != value[2:]:
        raise PackagePolicyError(f"{context}: target must be a canonical relative path")
    return value


def _export_targets(value: Any, context: str) -> tuple[str, ...]:
    if isinstance(value, str):
        return (_relative_target(value, context),)
    if not isinstance(value, dict) or not value:
        raise PackagePolicyError(f"{context}: export must be a path or non-empty condition map")
    targets: list[str] = []
    for condition, target in value.items():
        if condition not in {"types", "import", "default"}:
            raise PackagePolicyError(f"{context}: unsupported export condition {condition!r}")
        targets.extend(_export_targets(target, f"{context}.{condition}"))
    return tuple(targets)


def _specifier(value: Any, context: str) -> str:
    if not isinstance(value, str) or (value != "." and not value.startswith("./")):
        raise PackagePolicyError(f"{context}: invalid package export specifier")
    if value != ".":
        path = PurePosixPath(value[2:])
        if any(part in ("", ".", "..") for part in path.parts) or path.as_posix() != value[2:]:
            raise PackagePolicyError(f"{context}: export specifier must be canonical")
    return value


def _regular_tree(root: Path) -> tuple[Path, ...]:
    try:
        root_metadata = root.lstat()
    except FileNotFoundError as error:
        raise PackagePolicyError(f"{root}: build output is missing") from error
    if not stat.S_ISDIR(root_metadata.st_mode) or root.is_symlink():
        raise PackagePolicyError(f"{root}: build output must be a real directory")
    files: list[Path] = []
    for current, directories, names in os.walk(root, followlinks=False):
        current_path = Path(current)
        for directory in directories:
            path = current_path / directory
            metadata = path.lstat()
            if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
                raise PackagePolicyError(f"{path}: linked or non-directory entry is forbidden")
        for name in names:
            path = current_path / name
            metadata = path.lstat()
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
                raise PackagePolicyError(f"{path}: artifact must be a regular, unlinked file")
            files.append(path)
    if not files:
        raise PackagePolicyError(f"{root}: build output is empty")
    return tuple(sorted(files))


@dataclass(frozen=True)
class Package:
    root: Path
    name: str
    version: str
    main: str
    types: str
    exports: dict[str, tuple[str, ...]]
    manifest: dict[str, Any]

    @property
    def dist(self) -> Path:
        return self.root / "dist"

    @classmethod
    def load(cls, root: Path) -> Package:
        manifest_path = root / "package.json"
        manifest = _read_json(manifest_path)
        name = manifest.get("name")
        version = manifest.get("version")
        if not isinstance(name, str) or PACKAGE_NAME.fullmatch(name) is None:
            raise PackagePolicyError(f"{manifest_path}: invalid @sarmg package name")
        if not isinstance(version, str) or SEMVER.fullmatch(version) is None:
            raise PackagePolicyError(f"{manifest_path}: invalid semantic version")
        if manifest.get("private") is True:
            raise PackagePolicyError(f"{manifest_path}: publishable package cannot be private")
        if manifest.get("type") != "module":
            raise PackagePolicyError(f"{manifest_path}: type must be module")
        if manifest.get("files") != ["dist"]:
            raise PackagePolicyError(f"{manifest_path}: files must be exactly [\"dist\"]")
        main = _relative_target(manifest.get("main"), f"{manifest_path}.main")
        types = _relative_target(manifest.get("types"), f"{manifest_path}.types")
        raw_exports = manifest.get("exports")
        if not isinstance(raw_exports, dict) or not raw_exports:
            raise PackagePolicyError(f"{manifest_path}: exports must be a non-empty object")
        exports: dict[str, tuple[str, ...]] = {}
        for raw_specifier, target in raw_exports.items():
            specifier = _specifier(raw_specifier, f"{manifest_path}.exports")
            exports[specifier] = _export_targets(
                target, f"{manifest_path}.exports[{specifier!r}]"
            )
        if "." not in exports or main not in exports["."] or types not in exports["."]:
            raise PackagePolicyError(
                f"{manifest_path}: root export must include the main and types targets"
            )
        for section in (
            "dependencies",
            "devDependencies",
            "optionalDependencies",
            "peerDependencies",
        ):
            dependencies = manifest.get(section, {})
            if not isinstance(dependencies, dict):
                raise PackagePolicyError(f"{manifest_path}.{section}: expected an object")
            for dependency, requirement in dependencies.items():
                if isinstance(requirement, str) and requirement.startswith("workspace:"):
                    if requirement != f"workspace:{version}":
                        raise PackagePolicyError(
                            f"{manifest_path}: {dependency} must use workspace:{version}, "
                            "not a range or wildcard"
                        )
        return cls(root, name, version, main, types, exports, manifest)

    def check_build(self) -> tuple[Path, ...]:
        files = _regular_tree(self.dist)
        for targets in self.exports.values():
            for target in targets:
                path = self.root / target[2:]
                try:
                    metadata = path.lstat()
                except FileNotFoundError as error:
                    raise PackagePolicyError(
                        f"{self.name}: exported artifact is missing: {target}"
                    ) from error
                if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
                    raise PackagePolicyError(
                        f"{self.name}: exported artifact must be regular and unlinked: {target}"
                    )
        return files


def discover_packages(repository: Path) -> tuple[Package, ...]:
    repository = repository.resolve(strict=True)
    if repository == Path(repository.anchor):
        raise PackagePolicyError("refusing to operate on a filesystem root")
    packages_root = repository / "packages"
    if not packages_root.is_dir() or packages_root.is_symlink():
        raise PackagePolicyError(f"{packages_root}: packages directory is missing or linked")
    roots: list[Path] = []
    for path in packages_root.iterdir():
        if path.is_symlink():
            raise PackagePolicyError(f"{path}: linked package roots are forbidden")
        if path.is_dir() and (path / "package.json").exists():
            roots.append(path)
    roots.sort()
    if not roots:
        raise PackagePolicyError(f"{packages_root}: no package manifests found")
    packages = tuple(Package.load(root) for root in roots)
    names = [package.name for package in packages]
    if len(names) != len(set(names)):
        raise PackagePolicyError("package names must be unique")
    versions = {package.version for package in packages}
    if len(versions) != 1:
        raise PackagePolicyError(f"all publishable packages must share one version: {sorted(versions)}")
    available = set(names)
    for package in packages:
        for section in (
            "dependencies",
            "devDependencies",
            "optionalDependencies",
            "peerDependencies",
        ):
            for dependency in package.manifest.get(section, {}):
                if dependency.startswith("@sarmg/") and dependency not in available:
                    raise PackagePolicyError(
                        f"{package.name}: in-repository dependency {dependency} is missing"
                    )
    return packages


def clean_dist(packages: Iterable[Package]) -> None:
    for package in packages:
        dist = package.dist
        try:
            metadata = dist.lstat()
        except FileNotFoundError:
            print(f"package clean: already absent {dist.relative_to(package.root.parent.parent)}")
            continue
        if not stat.S_ISDIR(metadata.st_mode) or dist.is_symlink():
            raise PackagePolicyError(f"{dist}: refusing to remove a linked or non-directory path")
        shutil.rmtree(dist)
        print(f"package clean: removed {dist.relative_to(package.root.parent.parent)}")


def check_builds(packages: Iterable[Package]) -> None:
    for package in packages:
        files = package.check_build()
        print(f"package manifest: passed {package.name} ({len(files)} artifacts)")


def _run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            command,
            cwd=cwd,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError as error:
        detail = (error.stderr or error.stdout or "").strip()
        raise PackagePolicyError(f"command failed: {' '.join(command)}: {detail}") from error


def _pack(package: Package, destination: Path) -> Path:
    before = set(destination.glob("*.tgz"))
    _run(
        ["pnpm", "pack", "--pack-destination", str(destination)],
        package.root,
    )
    created = set(destination.glob("*.tgz")) - before
    if len(created) != 1:
        raise PackagePolicyError(
            f"{package.name}: pnpm pack produced {len(created)} new tarballs"
        )
    tarball = created.pop()
    if tarball.stat().st_size > MAX_TARBALL_BYTES:
        raise PackagePolicyError(f"{package.name}: tarball exceeds the 64 MiB policy limit")
    return tarball


def _inspect_tarball(package: Package, tarball: Path, expected_files: tuple[Path, ...]) -> None:
    expected_dist = {
        "package/" + path.relative_to(package.root).as_posix() for path in expected_files
    }
    names: set[str] = set()
    packed_manifest: dict[str, Any] | None = None
    with tarfile.open(tarball, "r:gz") as archive:
        for member in archive.getmembers():
            path = PurePosixPath(member.name)
            if (
                path.is_absolute()
                or not path.parts
                or path.parts[0] != "package"
                or any(part in ("", ".", "..") for part in path.parts)
                or path.as_posix() != member.name
            ):
                raise PackagePolicyError(f"{tarball}: non-canonical member {member.name!r}")
            if member.name in names:
                raise PackagePolicyError(f"{tarball}: duplicate member {member.name!r}")
            names.add(member.name)
            if not member.isfile() and not member.isdir():
                raise PackagePolicyError(f"{tarball}: linked or special member {member.name!r}")
            if member.isfile() and member.name == "package/package.json":
                extracted = archive.extractfile(member)
                if extracted is None:
                    raise PackagePolicyError(f"{tarball}: cannot read packed package.json")
                data = extracted.read(MAX_MANIFEST_BYTES + 1)
                if len(data) > MAX_MANIFEST_BYTES:
                    raise PackagePolicyError(f"{tarball}: packed package.json is too large")
                packed_manifest = json.loads(data.decode("utf-8"), object_pairs_hook=_unique_object)
    missing = sorted(expected_dist - names)
    if missing:
        raise PackagePolicyError(f"{package.name}: tarball omits build artifacts {missing}")
    allowed_metadata = {
        "package/package.json",
        "package/LICENSE",
        "package/README",
        "package/README.md",
    }
    unexpected = sorted(
        name
        for name in names
        if not name.startswith("package/dist/")
        and name not in allowed_metadata
        and name != "package/dist"
    )
    if unexpected:
        raise PackagePolicyError(f"{package.name}: tarball contains unpublished sources {unexpected}")
    if not isinstance(packed_manifest, dict):
        raise PackagePolicyError(f"{package.name}: tarball has no package.json")
    if packed_manifest.get("name") != package.name or packed_manifest.get("version") != package.version:
        raise PackagePolicyError(f"{package.name}: packed identity differs from source manifest")
    for section in (
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ):
        dependencies = packed_manifest.get(section, {})
        if isinstance(dependencies, dict):
            for dependency, requirement in dependencies.items():
                if isinstance(requirement, str) and requirement.startswith("workspace:"):
                    raise PackagePolicyError(
                        f"{package.name}: packed dependency {dependency} retains workspace protocol"
                    )
    print(f"package tarball: passed {package.name} ({tarball.name})")


def _install_smoke(packages: tuple[Package, ...], tarballs: tuple[Path, ...], consumer: Path) -> None:
    (consumer / "package.json").write_text(
        json.dumps({"name": "sarmg-package-smoke", "private": True, "type": "module"}) + "\n",
        encoding="utf-8",
    )
    _run(
        [
            "npm",
            "install",
            "--cache",
            str(consumer / ".npm-cache"),
            "--offline",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--package-lock=false",
            *(str(path) for path in tarballs),
        ],
        consumer,
    )
    imports = [package.name for package in packages]
    resolutions = [
        package.name + ("" if specifier == "." else specifier[1:])
        for package in packages
        for specifier in package.exports
    ]
    smoke = consumer / "smoke.mjs"
    smoke.write_text(
        "import { access } from 'node:fs/promises';\n"
        "import { fileURLToPath } from 'node:url';\n"
        f"const imports = {json.dumps(imports)};\n"
        f"const resolutions = {json.dumps(resolutions)};\n"
        "for (const specifier of imports) await import(specifier);\n"
        "for (const specifier of resolutions) {\n"
        "  const resolved = import.meta.resolve(specifier);\n"
        "  if (!resolved.startsWith('file:')) throw new Error(`non-file export ${specifier}`);\n"
        "  await access(fileURLToPath(resolved));\n"
        "}\n",
        encoding="utf-8",
    )
    _run(["node", str(smoke)], consumer)
    print(f"package install smoke: passed {len(packages)} packages and {len(resolutions)} exports")


def package_smoke(repository: Path, packages: tuple[Package, ...]) -> None:
    clean_dist(packages)
    _run(["pnpm", "-r", "build"], repository)
    check_builds(packages)
    with tempfile.TemporaryDirectory(prefix="sarmg-package-smoke-") as directory:
        temporary = Path(directory)
        tarball_dir = temporary / "tarballs"
        consumer = temporary / "consumer"
        tarball_dir.mkdir()
        consumer.mkdir()
        tarballs: list[Path] = []
        for package in packages:
            expected = package.check_build()
            tarball = _pack(package, tarball_dir)
            _inspect_tarball(package, tarball, expected)
            tarballs.append(tarball)
        _install_smoke(packages, tuple(tarballs), consumer)


def package_release(
    repository: Path,
    packages: tuple[Package, ...],
    destination: Path,
) -> tuple[Path, ...]:
    """Create inspected package tarballs in one caller-owned empty directory."""

    repository = repository.resolve(strict=True)
    destination = destination.resolve()
    if destination == Path(destination.anchor) or destination == repository:
        raise PackagePolicyError("refusing to use a broad release destination")
    if destination.exists():
        metadata = destination.lstat()
        if not stat.S_ISDIR(metadata.st_mode) or destination.is_symlink():
            raise PackagePolicyError(f"{destination}: release destination must be a real directory")
        if any(destination.iterdir()):
            raise PackagePolicyError(f"{destination}: release destination must be empty")
    else:
        destination.mkdir(parents=True)

    clean_dist(packages)
    _run(["pnpm", "-r", "build"], repository)
    check_builds(packages)
    tarballs: list[Path] = []
    for package in packages:
        expected = package.check_build()
        tarball = _pack(package, destination)
        _inspect_tarball(package, tarball, expected)
        tarballs.append(tarball)
    print(f"package release: created {len(tarballs)} inspected tarballs in {destination}")
    return tuple(sorted(tarballs))
