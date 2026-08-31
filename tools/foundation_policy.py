"""Repository-wide identity and consumer-matrix checks for one current release."""

from __future__ import annotations

import json
import re
import stat
import tomllib
from pathlib import Path
from typing import Any


CURRENT_VERSION = "0.3.0"
NODE_VERSION = "26.7.0"
PNPM_VERSION = "10.12.1"
RUST_VERSION = "1.98.0"
ADMIN_WEB_DEV_DEPENDENCIES = {
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
    "@sarmg/contracts",
    "@sarmg/design-tokens",
    "@sarmg/http-client",
    "sarmg-admin-auth",
    "sarmg-contracts",
    "sarmg-error",
    "sarmg-schema-identity",
    "sarmg-server-target",
    "sarmg-sqlite",
}
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
        if manifest.get("license") != "Apache-2.0":
            raise FoundationPolicyError(f"{path}: license must be Apache-2.0")
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

    admin_web = _json(root / "packages" / "admin-web" / "package.json")
    admin_development = admin_web.get("devDependencies", {})
    for dependency, expected in ADMIN_WEB_DEV_DEPENDENCIES.items():
        if admin_development.get(dependency) != expected:
            raise FoundationPolicyError(
                f"packages/admin-web/package.json: {dependency} must be exactly {expected}"
            )
    admin_peers = admin_web.get("peerDependencies", {})
    for dependency in ("@vitejs/plugin-react", "react", "react-dom", "vite"):
        expected = ADMIN_WEB_DEV_DEPENDENCIES[dependency]
        if admin_peers.get(dependency) != expected:
            raise FoundationPolicyError(
                f"packages/admin-web/package.json peer {dependency} must be exactly {expected}"
            )

    members = set(cargo.get("workspace", {}).get("members", []))
    expected_members = {
        f"rust/crates/{name}" for name in KNOWN_PACKAGES if not name.startswith("@")
    }
    if members != expected_members:
        raise FoundationPolicyError(f"Rust workspace members differ: {sorted(members)}")
    for member in sorted(members):
        manifest_path = root / member / "Cargo.toml"
        manifest = _toml(manifest_path)
        package = manifest.get("package", {})
        if package.get("name") != Path(member).name or package.get("version") != {"workspace": True}:
            raise FoundationPolicyError(f"{manifest_path}: crate identity is not workspace-owned")
        if manifest.get("lints") != {"workspace": True}:
            raise FoundationPolicyError(f"{manifest_path}: workspace lints are not enabled")
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            dependencies = manifest.get(section, {})
            for dependency, requirement in dependencies.items():
                if dependency not in KNOWN_PACKAGES or dependency.startswith("@"):
                    continue
                if not isinstance(requirement, dict) or requirement.get("version") != f"={CURRENT_VERSION}":
                    raise FoundationPolicyError(
                        f"{manifest_path}: internal {dependency} must pin ={CURRENT_VERSION}"
                    )


def check_consumer_matrix(root: Path) -> None:
    schema = _json(root / "consumers" / "consumer-matrix.schema.json")
    allowed = set(
        schema["$defs"]["consumer"]["properties"]["packages"]["items"]["enum"]
    )
    if allowed != KNOWN_PACKAGES:
        raise FoundationPolicyError("consumer matrix schema does not enumerate every component")

    matrix = _json(root / "consumers" / "consumer-matrix.json")
    _exact_keys(
        matrix,
        {"$schema", "format", "foundation_version", "updated_on", "consumers"},
        "consumer matrix",
    )
    if matrix["format"] != "sarmg.consumer-matrix.v1":
        raise FoundationPolicyError("consumer matrix: unsupported format")
    if matrix["foundation_version"] != CURRENT_VERSION:
        raise FoundationPolicyError("consumer matrix: foundation_version differs")
    consumers = matrix["consumers"]
    if not isinstance(consumers, list):
        raise FoundationPolicyError("consumer matrix: consumers must be an array")
    repositories: set[str] = set()
    for index, consumer in enumerate(consumers):
        if not isinstance(consumer, dict):
            raise FoundationPolicyError(f"consumer[{index}]: expected an object")
        _exact_keys(
            consumer,
            {"repository", "commit", "adopted_version", "packages", "status", "last_verified_commit"},
            f"consumer[{index}]",
        )
        repository = consumer["repository"]
        if repository in repositories:
            raise FoundationPolicyError(f"consumer matrix: duplicate {repository!r}")
        repositories.add(repository)
        if not isinstance(consumer["commit"], str) or SOURCE_REVISION.fullmatch(consumer["commit"]) is None:
            raise FoundationPolicyError(f"consumer[{index}].commit: invalid revision")
        packages = consumer["packages"]
        if not isinstance(packages, list) or len(packages) != len(set(packages)) or not set(packages) <= KNOWN_PACKAGES:
            raise FoundationPolicyError(f"consumer[{index}].packages: invalid component set")
        status = consumer["status"]
        if status not in {"not-integrated", "integration-pending", "passing", "failing"}:
            raise FoundationPolicyError(f"consumer[{index}].status: invalid status")
        if status == "not-integrated":
            if consumer["adopted_version"] is not None or packages or consumer["last_verified_commit"] is not None:
                raise FoundationPolicyError(f"consumer[{index}]: not-integrated evidence must be empty")
        else:
            if consumer["adopted_version"] != CURRENT_VERSION or not packages:
                raise FoundationPolicyError(f"consumer[{index}]: integrated entry lacks current components")
            verified = consumer["last_verified_commit"]
            if verified is not None and (
                not isinstance(verified, str) or SOURCE_REVISION.fullmatch(verified) is None
            ):
                raise FoundationPolicyError(f"consumer[{index}].last_verified_commit: invalid revision")
            if status == "passing" and verified is None:
                raise FoundationPolicyError(f"consumer[{index}]: passing entry needs verified commit")
    if repositories != KNOWN_CONSUMERS:
        raise FoundationPolicyError(
            f"consumer repository set differs: {sorted(repositories)}"
        )


def check_repository(root: Path) -> None:
    root = root.resolve(strict=True)
    if root == Path(root.anchor):
        raise FoundationPolicyError("refusing to check a filesystem root")
    check_versions(root)
    check_consumer_matrix(root)
