"""Repository-wide checks for one self-contained xcss release."""

from __future__ import annotations

import hashlib
import json
import re
import stat
import tomllib
from pathlib import Path
from typing import Any


CURRENT_VERSION = "1.0.1"
NODE_VERSION = "26.7.0"
PNPM_VERSION = "10.34.6"
RUST_VERSION = "1.99.0"
APACHE_2_LICENSE_SHA256 = (
    "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"
)
WEB_TOOLCHAIN_DEV_DEPENDENCIES = {
    "@types/node": "26.6.4",
    "@types/react": "19.3.0",
    "@types/react-dom": "19.3.0",
    "@vitejs/plugin-react": "6.1.2",
    "react": "19.3.0",
    "react-dom": "19.3.0",
    "typescript": "7.0.2",
    "vite": "8.3.3",
}
SOURCE_REVISION = re.compile(r"[0-9a-f]{40}")
KNOWN_PACKAGES = {"xcss", "@xcss/web"}
RUST_PACKAGES = ("xcss",)
SERVER_DEPENDENCY_TARGET = 'cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))'

class XcssPolicyError(RuntimeError):
    """The repository does not describe one internally consistent release."""


def _regular_file(path: Path) -> None:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise XcssPolicyError(f"{path}: must be one regular, unlinked file")


def read_audited_root_license(path: Path) -> bytes:
    _regular_file(path)
    content = path.read_bytes()
    if hashlib.sha256(content).hexdigest() != APACHE_2_LICENSE_SHA256:
        raise XcssPolicyError(
            f"{path}: content differs from the audited Apache-2.0 text"
        )
    return content


def require_license_copy(path: Path, expected: bytes) -> None:
    _regular_file(path)
    if path.read_bytes() != expected:
        raise XcssPolicyError(
            f"{path}: content differs from the audited root LICENSE"
        )


def _json(path: Path) -> dict[str, Any]:
    _regular_file(path)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise XcssPolicyError(f"{path}: invalid UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise XcssPolicyError(f"{path}: expected an object")
    return value


def _toml(path: Path) -> dict[str, Any]:
    _regular_file(path)
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise XcssPolicyError(f"{path}: invalid UTF-8 TOML: {error}") from error


def _exact_keys(value: dict[str, Any], expected: set[str], context: str) -> None:
    if set(value) != expected:
        raise XcssPolicyError(
            f"{context}: keys differ: missing={sorted(expected - set(value))}, "
            f"unknown={sorted(set(value) - expected)}"
        )


def check_dependency_boundaries(root: Path) -> None:
    """A server monolith depends only on external libraries, never another project."""
    root = root.resolve()
    cargo = _toml(root / "Cargo.toml")
    workspace_dependencies = cargo.get("workspace", {}).get("dependencies", {})
    def inspect(table: dict[str, Any], path: Path) -> None:
        for alias, declared in table.items():
            value = workspace_dependencies.get(alias) if isinstance(declared, dict) and declared.get("workspace") else declared
            if value is None:
                raise XcssPolicyError(f"{path}: unknown workspace dependency {alias}")
            name = value.get("package", alias) if isinstance(value, dict) else alias
            if name == "xcss" or name.startswith("xcss-"):
                raise XcssPolicyError(f"{path}: {name} is outside the single xcss package boundary")
            if isinstance(value, dict) and "path" in value:
                raise XcssPolicyError(f"{path}: external local dependency {name} is not self-contained")
    manifests = [root / "Cargo.toml", *sorted((root / "src").glob("*/Cargo.toml")), *sorted((root / "rust/crates").glob("*/Cargo.toml"))]
    for path in manifests:
        manifest = cargo if path == root / "Cargo.toml" else _toml(path)
        for scope in [manifest, *manifest.get("target", {}).values(), manifest.get("workspace", {})]:
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                inspect(scope.get(section, {}), path)
        for patches in manifest.get("patch", {}).values():
            inspect(patches, path)
        inspect({name.split(":", 1)[0]: value for name, value in manifest.get("replace", {}).items()}, path)
    for path in [root / "package.json", *sorted((root / "web").glob("*/package.json")), *sorted((root / "packages").glob("*/package.json"))]:
        manifest = _json(path)
        for section in ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies"):
            for alias, value in manifest.get(section, {}).items():
                name = alias
                if isinstance(value, str) and value.startswith("npm:"):
                    match = re.fullmatch(r"npm:((?:@[^/]+/)?[^@]+)(?:@.*)?", value)
                    if match:
                        name = match.group(1)
                if name.startswith("@xcss/"):
                    raise XcssPolicyError(f"{path}: {name} is outside the single Web package boundary")
                if isinstance(value, str) and value.startswith(("file:", "link:", "workspace:", "./", "../", "/")):
                    raise XcssPolicyError(f"{path}: external local dependency {name} is not self-contained")


def check_server_dependency_targets(root: Path) -> None:
    """Reject unsupported targets before any external C dependency can build."""
    cargo = _toml(root / "Cargo.toml")
    sections = ("dependencies", "dev-dependencies", "build-dependencies")
    if any(cargo.get(section) for section in sections):
        raise XcssPolicyError("all xcss dependencies must be scoped to the canonical server target")
    for target, scope in cargo.get("target", {}).items():
        if target != SERVER_DEPENDENCY_TARGET and any(scope.get(section) for section in sections):
            raise XcssPolicyError("all xcss dependencies must be scoped to the canonical server target")
    if not cargo.get("target", {}).get(SERVER_DEPENDENCY_TARGET, {}).get("dependencies"):
        raise XcssPolicyError("xcss must declare its canonical server dependency graph")


def check_web_lockfile(root: Path) -> None:
    """Check the owned pnpm layout and binaries needed by a clean Linux build."""
    path = root / "pnpm-lock.yaml"
    _regular_file(path)
    text = path.read_text(encoding="utf-8")

    def entries(section: str) -> dict[str, str]:
        match = re.search(r"^" + section + r":\n(.*?)(?=^\S|\Z)", text, re.M | re.S)
        if match is None:
            raise XcssPolicyError(f"{path}: missing pnpm {section} section")
        body = match.group(1)
        headers = list(re.finditer(r"^  (\S.*):(?:[^\n]*)\n", body, re.M))
        return {
            header.group(1).strip("'"): body[header.start(): headers[index + 1].start() if index + 1 < len(headers) else len(body)]
            for index, header in enumerate(headers)
        }

    if set(entries("importers")) != {"."} or re.search(r"(?:workspace:|link:)", text):
        raise XcssPolicyError(f"{path}: only the root Web package importer is allowed")
    packages = entries("packages")
    snapshots = entries("snapshots")
    required = {
        "typescript@7.0.2": ("@typescript/typescript-linux-x64", "7.0.2"),
        "rolldown@1.2.12": ("@rolldown/binding-linux-x64-gnu", "1.2.12"),
        "lightningcss@1.33.0": ("lightningcss-linux-x64-gnu", "1.33.0"),
    }
    for parent, (native, version) in required.items():
        native_package = packages.get(f"{native}@{version}", "")
        snapshot = snapshots.get(parent, "")
        declaration = rf"^      '?{re.escape(native)}'?: {re.escape(version)}$"
        if (
            "integrity: sha512-" not in native_package
            or "    optionalDependencies:\n" not in snapshot
            or re.search(declaration, snapshot, re.M) is None
        ):
            raise XcssPolicyError(f"{path}: locked native dependencies are incomplete for {parent}")


def check_versions(root: Path) -> None:
    read_audited_root_license(root / "LICENSE")
    cargo = _toml(root / "Cargo.toml")
    package = cargo.get("package", {})
    if package.get("name") != "xcss" or package.get("version") != CURRENT_VERSION:
        raise XcssPolicyError(f"Cargo.toml must define the one xcss {CURRENT_VERSION} package")
    if package.get("rust-version") != RUST_VERSION.removesuffix(".0") or package.get("license") != "Apache-2.0":
        raise XcssPolicyError("Cargo.toml toolchain or license differs from policy")
    if "workspace" in cargo or list((root / "src").rglob("Cargo.toml")) or list((root / "rust/crates").glob("*/Cargo.toml")):
        raise XcssPolicyError("xcss must not contain independent Rust subpackages")
    gate = (root / "src/lib.rs").read_text()
    if not all(token in gate for token in ('target_os = "linux"', 'target_arch = "x86_64"', 'target_env = "gnu"', 'compile_error!')):
        raise XcssPolicyError("the whole xcss crate must enforce the Linux AMD64 GNU target")
    manifest = _json(root / "package.json")
    if manifest.get("name") != "@xcss/web" or manifest.get("version") != CURRENT_VERSION or manifest.get("private") is True:
        raise XcssPolicyError(f"package.json must publish the one @xcss/web {CURRENT_VERSION} package")
    for key, expected in {"os": ["linux"], "cpu": ["x64"], "libc": ["glibc"]}.items():
        if manifest.get(key) != expected:
            raise XcssPolicyError(f"package.json {key} differs from the server build boundary")
    if list((root / "web").glob("*/package.json")) or list((root / "packages").glob("*/package.json")):
        raise XcssPolicyError("Web support must not contain independent npm subpackages")
    if manifest.get("engines", {}).get("node") != f">={NODE_VERSION} <27" or manifest.get("packageManager") != f"pnpm@{PNPM_VERSION}":
        raise XcssPolicyError("package.json Node or pnpm version differs from policy")
    for dependency, expected in WEB_TOOLCHAIN_DEV_DEPENDENCIES.items():
        if manifest.get("devDependencies", {}).get(dependency) != expected:
            raise XcssPolicyError(f"package.json {dependency} must be exactly {expected}")
    if (root / ".node-version").read_text().strip() != NODE_VERSION:
        raise XcssPolicyError(".node-version differs from policy")
    if _toml(root / "rust-toolchain.toml").get("toolchain", {}).get("channel") != RUST_VERSION:
        raise XcssPolicyError("Rust toolchain differs from policy")


def check_repository(root: Path) -> None:
    root = root.resolve(strict=True)
    if root == Path(root.anchor):
        raise XcssPolicyError("refusing to check a filesystem root")
    check_dependency_boundaries(root)
    check_server_dependency_targets(root)
    check_web_lockfile(root)
    check_versions(root)
