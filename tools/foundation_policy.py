"""Repository-wide identity and consumer-matrix checks for one current release."""

from __future__ import annotations

import hashlib
import json
import re
import stat
import tomllib
from pathlib import Path
from typing import Any


CURRENT_VERSION = "0.7.1"
NODE_VERSION = "26.7.0"
PNPM_VERSION = "10.12.1"
RUST_VERSION = "1.98.0"
APACHE_2_LICENSE_SHA256 = (
    "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"
)
WEB_TOOLCHAIN_DEV_DEPENDENCIES = {
    "@types/react": "19.2.18",
    "@types/react-dom": "19.2.5",
    "@vitejs/plugin-react": "4.7.0",
    "react": "19.2.8",
    "react-dom": "19.2.8",
    "typescript": "5.8.3",
    "vite": "7.3.6",
}
SOURCE_REVISION = re.compile(r"[0-9a-f]{40}")
KNOWN_PACKAGES = {
    "@sarmg/admin-web",
    "@sarmg/admin-shell",
    "@sarmg/admin-ui",
    "@sarmg/contracts",
    "@sarmg/design-tokens",
    "@sarmg/http-client",
    "@sarmg/web-fonts",
    "@sarmg/web-toolchain",
    "sarmg-admin-auth",
    "sarmg-admin-axum",
    "sarmg-admin-core",
    "sarmg-admin-hyper",
    "sarmg-admin-sqlite",
    "sarmg-admin-static",
    "sarmg-contracts",
    "sarmg-error",
    "sarmg-schema-identity",
    "sarmg-server-target",
    "sarmg-sqlite",
    "sarmg-state-file",
    "sarmg-platform-db",
    "sarmg-server-runtime",
    "sarmg-fs-safety",
    "sarmg-secret",
    "sarmg-secret-envelope",
    "sarmg-secure-http",
    "sarmg-secure-xml",
    "sarmg-operations",
    "sarmg-testkit",
}
RUST_PACKAGES = tuple(
    sorted(name for name in KNOWN_PACKAGES if not name.startswith("@"))
)
KNOWN_CONSUMERS = {
    "dufs-ram",
    "host-monitoring",
    "media-backup",
    "sarmg-upgrade",
    "sentinel-monitor",
    "sunshine-manager",
}
class FoundationPolicyError(RuntimeError):
    """The repository does not describe one internally consistent release."""


def _regular_file(path: Path) -> None:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise FoundationPolicyError(f"{path}: must be one regular, unlinked file")


def read_audited_root_license(path: Path) -> bytes:
    _regular_file(path)
    content = path.read_bytes()
    if hashlib.sha256(content).hexdigest() != APACHE_2_LICENSE_SHA256:
        raise FoundationPolicyError(
            f"{path}: content differs from the audited Apache-2.0 text"
        )
    return content


def require_license_copy(path: Path, expected: bytes) -> None:
    _regular_file(path)
    if path.read_bytes() != expected:
        raise FoundationPolicyError(
            f"{path}: content differs from the audited root LICENSE"
        )


def _json(path: Path) -> dict[str, Any]:
    _regular_file(path)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise FoundationPolicyError(f"{path}: invalid UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise FoundationPolicyError(f"{path}: expected an object")
    return value


def _toml(path: Path) -> dict[str, Any]:
    _regular_file(path)
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise FoundationPolicyError(f"{path}: invalid UTF-8 TOML: {error}") from error


def _exact_keys(value: dict[str, Any], expected: set[str], context: str) -> None:
    if set(value) != expected:
        raise FoundationPolicyError(
            f"{context}: keys differ: missing={sorted(expected - set(value))}, "
            f"unknown={sorted(set(value) - expected)}"
        )


def check_versions(root: Path) -> None:
    root_license = root / "LICENSE"
    root_license_bytes = read_audited_root_license(root_license)

    root_package = _json(root / "package.json")
    if root_package.get("version") != CURRENT_VERSION:
        raise FoundationPolicyError("package.json: workspace version differs from policy")
    if root_package.get("engines", {}).get("node") != f">={NODE_VERSION} <27":
        raise FoundationPolicyError("package.json: Node engine is not the current exact major line")
    if root_package.get("packageManager") != f"pnpm@{PNPM_VERSION}":
        raise FoundationPolicyError("package.json: packageManager differs from the current pnpm version")
    if (root / ".node-version").read_text(encoding="utf-8").strip() != NODE_VERSION:
        raise FoundationPolicyError(".node-version differs from the current Node version")

    cargo = _toml(root / "Cargo.toml")
    workspace_package = cargo.get("workspace", {}).get("package", {})
    if workspace_package.get("version") != CURRENT_VERSION:
        raise FoundationPolicyError("Cargo workspace version differs from policy")
    if workspace_package.get("rust-version") != RUST_VERSION.removesuffix(".0"):
        raise FoundationPolicyError("Cargo rust-version differs from policy")
    toolchain = _toml(root / "rust-toolchain.toml")
    if toolchain.get("toolchain", {}).get("channel") != RUST_VERSION:
        raise FoundationPolicyError("rust-toolchain.toml differs from policy")

    package_roots = sorted((root / "packages").glob("*/package.json"))
    observed_web: set[str] = set()
    for path in package_roots:
        manifest = _json(path)
        name = manifest.get("name")
        if not isinstance(name, str) or name not in KNOWN_PACKAGES:
            raise FoundationPolicyError(f"{path}: unknown package {name!r}")
        observed_web.add(name)
        if manifest.get("version") != CURRENT_VERSION:
            raise FoundationPolicyError(f"{path}: version must be {CURRENT_VERSION}")
        expected_license = "OFL-1.1" if name == "@sarmg/web-fonts" else "Apache-2.0"
        if manifest.get("license") != expected_license:
            raise FoundationPolicyError(f"{path}: license must be {expected_license}")
        if manifest.get("engines", {}).get("node") != f">={NODE_VERSION} <27":
            raise FoundationPolicyError(f"{path}: Node engine differs from policy")
        for section in ("dependencies", "devDependencies", "optionalDependencies"):
            dependencies = manifest.get(section, {})
            if not isinstance(dependencies, dict):
                raise FoundationPolicyError(f"{path}.{section}: expected an object")
            for dependency, requirement in dependencies.items():
                if dependency.startswith("@sarmg/") and requirement != f"workspace:{CURRENT_VERSION}":
                    raise FoundationPolicyError(
                        f"{path}: internal {dependency} must use workspace:{CURRENT_VERSION}"
                    )
        for dependency, requirement in manifest.get("peerDependencies", {}).items():
            if dependency.startswith("@sarmg/") and requirement != CURRENT_VERSION:
                raise FoundationPolicyError(
                    f"{path}: internal peer {dependency} must be exactly {CURRENT_VERSION}"
                )
    expected_web = {name for name in KNOWN_PACKAGES if name.startswith("@sarmg/")}
    if observed_web != expected_web:
        raise FoundationPolicyError(
            f"publishable Web package set differs: {sorted(observed_web)}"
        )

    toolchain = _json(root / "packages" / "web-toolchain" / "package.json")
    toolchain_development = toolchain.get("devDependencies", {})
    for dependency, expected in WEB_TOOLCHAIN_DEV_DEPENDENCIES.items():
        if dependency.startswith("@types/"):
            continue
        if toolchain_development.get(dependency) != expected:
            raise FoundationPolicyError(
                f"packages/web-toolchain/package.json: {dependency} must be exactly {expected}"
            )
    toolchain_peers = toolchain.get("peerDependencies", {})
    for dependency in ("@vitejs/plugin-react", "react", "react-dom", "vite"):
        expected = WEB_TOOLCHAIN_DEV_DEPENDENCIES[dependency]
        if toolchain_peers.get(dependency) != expected:
            raise FoundationPolicyError(
                f"packages/web-toolchain/package.json peer {dependency} must be exactly {expected}"
            )

    members = set(cargo.get("workspace", {}).get("members", []))
    expected_members = {f"rust/crates/{name}" for name in RUST_PACKAGES}
    if members != expected_members:
        raise FoundationPolicyError(f"Rust workspace members differ: {sorted(members)}")
    for member in sorted(members):
        manifest_path = root / member / "Cargo.toml"
        manifest = _toml(manifest_path)
        package = manifest.get("package", {})
        if package.get("name") != Path(member).name or package.get("version") != {"workspace": True}:
            raise FoundationPolicyError(f"{manifest_path}: crate identity is not workspace-owned")
        if package.get("license") != {"workspace": True} or "license-file" in package:
            raise FoundationPolicyError(
                f"{manifest_path}: crate must inherit the Apache-2.0 SPDX expression"
            )
        crate_license = root / member / "LICENSE"
        require_license_copy(crate_license, root_license_bytes)
        if manifest.get("lints") != {"workspace": True}:
            raise FoundationPolicyError(f"{manifest_path}: workspace lints are not enabled")
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            dependencies = manifest.get(section, {})
            for dependency, requirement in dependencies.items():
                package = requirement.get("package", dependency) if isinstance(requirement, dict) else dependency
                if package.startswith("sarmg-client-") or package == "sarmg-mobile-ffi":
                    raise FoundationPolicyError(f"{manifest_path}: Server must not depend on Client package {package}")
                if dependency not in KNOWN_PACKAGES or dependency.startswith("@"):
                    continue
                if not isinstance(requirement, dict) or requirement.get("version") != f"={CURRENT_VERSION}":
                    raise FoundationPolicyError(
                        f"{manifest_path}: internal {dependency} must pin ={CURRENT_VERSION}"
                    )


def check_consumer_matrix(root: Path) -> None:
    from sarmg_conformance import ConformanceError, verify_consumer_registry

    try:
        matrix = verify_consumer_registry(root)
    except ConformanceError as error:
        raise FoundationPolicyError(str(error)) from error
    repositories = {consumer["product"] for consumer in matrix["consumers"]}
    if repositories != KNOWN_CONSUMERS:
        raise FoundationPolicyError(f"consumer repository set differs: {sorted(repositories)}")


def check_repository(root: Path) -> None:
    root = root.resolve(strict=True)
    if root == Path(root.anchor):
        raise FoundationPolicyError("refusing to check a filesystem root")
    check_versions(root)
    check_consumer_matrix(root)
