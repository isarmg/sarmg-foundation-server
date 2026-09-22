"""Strict, dependency-free conformance checks for platform generation 1."""

from __future__ import annotations

import json
import re
import stat
import tomllib
from pathlib import Path
from typing import Any, Iterable


SEMVER = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)
REVISION = re.compile(r"[0-9a-f]{40}")
IDENTIFIER = re.compile(r"[a-z][a-z0-9-]{0,62}")
MATRIX_STATUSES = {
    "not-migrated",
    "migration-in-progress",
    "conforming",
    "non-conforming",
    "temporary-exception",
}
FOUNDATION_ROUTES = ("/api/v2/auth/", "/api/v2/platform/", "/healthz", "/readyz")
FOUNDATION_GIT_URL = "https://github.com/isarmg/sarmg-foundation-server.git"
FOUNDATION_RELEASE_URL = re.compile(
    r"https://github\.com/isarmg/sarmg-foundation-server/releases/download/"
    r"v([^/]+)/sarmg-([a-z0-9-]+)-([^/]+)\.tgz"
)


class ConformanceError(RuntimeError):
    """A manifest, source tree, or generated artifact violates platform policy."""


def _regular_file(path: Path) -> None:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise ConformanceError(f"{path}: must be one regular, unlinked file")


def _toml(path: Path) -> dict[str, Any]:
    _regular_file(path)
    try:
        value = tomllib.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise ConformanceError(f"{path}: invalid UTF-8 TOML: {error}") from error
    if not isinstance(value, dict):
        raise ConformanceError(f"{path}: expected a table")
    return value


def _json(path: Path) -> dict[str, Any]:
    _regular_file(path)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ConformanceError(f"{path}: invalid UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise ConformanceError(f"{path}: expected an object")
    return value


def _exact_keys(value: dict[str, Any], allowed: set[str], required: set[str], context: str) -> None:
    unknown = set(value) - allowed
    missing = required - set(value)
    if unknown or missing:
        raise ConformanceError(
            f"{context}: keys differ: missing={sorted(missing)}, unknown={sorted(unknown)}"
        )


def _string_list(value: Any, context: str, *, allow_empty: bool = True) -> list[str]:
    if not isinstance(value, list) or (not allow_empty and not value):
        raise ConformanceError(f"{context}: expected {'a non-empty' if not allow_empty else 'an'} array")
    if any(not isinstance(item, str) or IDENTIFIER.fullmatch(item) is None for item in value):
        raise ConformanceError(f"{context}: contains an invalid identifier")
    if len(value) != len(set(value)):
        raise ConformanceError(f"{context}: duplicate values are forbidden")
    return value


def load_profiles(foundation_root: Path) -> tuple[dict[str, dict[str, Any]], set[str]]:
    profile_root = foundation_root / "profiles"
    catalog = _toml(profile_root / "capabilities.toml")
    _exact_keys(catalog, {"format", "capabilities"}, {"format", "capabilities"}, "capability catalog")
    if catalog["format"] != 1 or not isinstance(catalog["capabilities"], dict):
        raise ConformanceError("capability catalog: unsupported format")
    capabilities = set(catalog["capabilities"])
    if not capabilities or any(IDENTIFIER.fullmatch(item) is None for item in capabilities):
        raise ConformanceError("capability catalog: invalid capability identifier")
    for identifier, value in catalog["capabilities"].items():
        if not isinstance(value, dict):
            raise ConformanceError(f"capability {identifier}: expected a table")
        _exact_keys(value, {"owner", "state"}, {"owner", "state"}, f"capability {identifier}")
        if value["owner"] not in {"foundation", "product-adapter"}:
            raise ConformanceError(f"capability {identifier}: invalid owner")
        if value["state"] not in {"current", "planned"}:
            raise ConformanceError(f"capability {identifier}: invalid state")

    profiles: dict[str, dict[str, Any]] = {}
    required_keys = {
        "format",
        "id",
        "kind",
        "formal_targets",
        "http_adapters",
        "web_profiles",
        "required_capabilities",
        "optional_capabilities",
    }
    for path in sorted(profile_root.glob("*.toml")):
        if path.name == "capabilities.toml":
            continue
        value = _toml(path)
        _exact_keys(value, required_keys | {"policy"}, required_keys, str(path))
        identifier = value["id"]
        if value["format"] != 1 or not isinstance(identifier, str) or IDENTIFIER.fullmatch(identifier) is None:
            raise ConformanceError(f"{path}: invalid profile identity")
        if path.stem != identifier or identifier in profiles:
            raise ConformanceError(f"{path}: profile filename/id mismatch or duplicate")
        if value["kind"] not in {"server", "tool", "web"}:
            raise ConformanceError(f"{path}: invalid profile kind")
        for key in ("formal_targets", "http_adapters", "web_profiles"):
            items = value[key]
            if not isinstance(items, list) or any(not isinstance(item, str) or not item for item in items):
                raise ConformanceError(f"{path}.{key}: expected unique strings")
            if len(items) != len(set(items)):
                raise ConformanceError(f"{path}.{key}: duplicate values are forbidden")
        required = set(_string_list(value["required_capabilities"], f"{path}.required_capabilities"))
        optional = set(_string_list(value["optional_capabilities"], f"{path}.optional_capabilities"))
        if required & optional or not required | optional <= capabilities:
            raise ConformanceError(f"{path}: capability set is inconsistent with the catalog")
        profiles[identifier] = value
    if set(profiles) != {
        "server-control-plane",
        "server-filesystem",
        "offline-tool",
        "web-react-admin",
        "web-embedded-native",
    }:
        raise ConformanceError("profiles: first-generation profile set is incomplete")
    return profiles, capabilities


def load_product_manifest(product_root: Path) -> dict[str, Any]:
    return _toml(product_root / "sarmg-product.toml")


def verify_manifest(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    profiles, _ = load_profiles(foundation_root)
    manifest = load_product_manifest(product_root)
    _exact_keys(
        manifest,
        {"format", "product_id", "foundation", "components"},
        {"format", "product_id", "foundation", "components"},
        "product manifest",
    )
    product_id = manifest["product_id"]
    if manifest["format"] != 1 or not isinstance(product_id, str) or IDENTIFIER.fullmatch(product_id) is None:
        raise ConformanceError("product manifest: invalid format or product_id")
    foundation = manifest["foundation"]
    if not isinstance(foundation, dict):
        raise ConformanceError("product manifest.foundation: expected a table")
    _exact_keys(
        foundation,
        {"platform_generation", "version", "git_rev"},
        {"platform_generation", "version", "git_rev"},
        "product manifest.foundation",
    )
    if foundation["platform_generation"] != 1:
        raise ConformanceError("product manifest: unsupported platform generation")
    if not isinstance(foundation["version"], str) or SEMVER.fullmatch(foundation["version"]) is None:
        raise ConformanceError("product manifest: invalid Foundation version")
    if not isinstance(foundation["git_rev"], str) or REVISION.fullmatch(foundation["git_rev"]) is None:
        raise ConformanceError("product manifest: Foundation git_rev must be 40 lowercase hex characters")
    components = manifest["components"]
    if not isinstance(components, list) or not components:
        raise ConformanceError("product manifest: at least one component is required")
    component_ids: set[str] = set()
    for index, component in enumerate(components):
        context = f"product manifest.components[{index}]"
        if not isinstance(component, dict):
            raise ConformanceError(f"{context}: expected a table")
        _exact_keys(
            component,
            {"id", "profile", "http_adapter", "web_profile", "capabilities"},
            {"id", "profile", "capabilities"},
            context,
        )
        component_id = component["id"]
        profile_id = component["profile"]
        if not isinstance(component_id, str) or IDENTIFIER.fullmatch(component_id) is None:
            raise ConformanceError(f"{context}.id: invalid identifier")
        if component_id in component_ids:
            raise ConformanceError(f"{context}.id: duplicate component")
        component_ids.add(component_id)
        if profile_id not in profiles:
            raise ConformanceError(f"{context}.profile: unknown Profile {profile_id!r}")
        profile = profiles[profile_id]
        declared = set(_string_list(component["capabilities"], f"{context}.capabilities", allow_empty=False))
        required = set(profile["required_capabilities"])
        allowed = required | set(profile["optional_capabilities"])
        if not required <= declared:
            raise ConformanceError(f"{context}: missing required capabilities {sorted(required - declared)}")
        if not declared <= allowed:
            raise ConformanceError(f"{context}: capabilities are not allowed by Profile: {sorted(declared - allowed)}")
        adapters = profile["http_adapters"]
        if adapters:
            if component.get("http_adapter") not in adapters:
                raise ConformanceError(f"{context}: http_adapter is not allowed by Profile")
        elif "http_adapter" in component:
            raise ConformanceError(f"{context}: Profile does not accept an http_adapter")
        web_profiles = profile["web_profiles"]
        if web_profiles:
            if component.get("web_profile") not in web_profiles:
                raise ConformanceError(f"{context}: web_profile is not allowed by Profile")
        elif "web_profile" in component:
            raise ConformanceError(f"{context}: Profile does not accept a web_profile")
    return manifest


def _walk_dependency_tables(value: Any, key: str = "") -> Iterable[tuple[str, Any]]:
    if not isinstance(value, dict):
        return
    if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
        yield from value.items()
    for nested_key, nested in value.items():
        if isinstance(nested, dict):
            yield from _walk_dependency_tables(nested, nested_key)


def _used_dependency_aliases(cargo: dict[str, Any]) -> set[str]:
    aliases: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        value = cargo.get(section, {})
        if isinstance(value, dict):
            aliases.update(value)
    targets = cargo.get("target", {})
    if isinstance(targets, dict):
        for target in targets.values():
            if not isinstance(target, dict):
                continue
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                value = target.get(section, {})
                if isinstance(value, dict):
                    aliases.update(value)
    return aliases


def _foundation_package_names(foundation_root: Path) -> set[str]:
    names: set[str] = set()
    for path in (foundation_root / "rust" / "crates").glob("*/Cargo.toml"):
        package = _toml(path).get("package", {})
        name = package.get("name") if isinstance(package, dict) else None
        if isinstance(name, str):
            names.add(name)
    for path in (foundation_root / "packages").glob("*/package.json"):
        name = _json(path).get("name")
        if isinstance(name, str):
            names.add(name)
    return names


def _dependency_name(alias: str, requirement: Any) -> str:
    if isinstance(requirement, dict) and isinstance(requirement.get("package"), str):
        return requirement["package"]
    return alias


def _workspace_dependencies(product_root: Path) -> dict[str, Any]:
    root_manifest = product_root / "Cargo.toml"
    if not root_manifest.is_file():
        return {}
    cargo = _toml(root_manifest)
    workspace = cargo.get("workspace", {})
    dependencies = workspace.get("dependencies", {}) if isinstance(workspace, dict) else {}
    return dependencies if isinstance(dependencies, dict) else {}


def _resolved_rust_requirement(
    alias: str,
    requirement: Any,
    workspace_dependencies: dict[str, Any],
    path: Path,
) -> Any:
    if not isinstance(requirement, dict) or requirement.get("workspace") is not True:
        return requirement
    inherited = workspace_dependencies.get(alias)
    if inherited is None:
        raise ConformanceError(
            f"{path}: Foundation dependency {alias} is inherited but absent from workspace.dependencies"
        )
    return inherited


def _verify_cargo_lock(
    product_root: Path,
    packages: set[str],
    expected_version: str,
    expected_revision: str,
) -> None:
    if not packages:
        return
    lock_path = product_root / "Cargo.lock"
    if not lock_path.is_file():
        raise ConformanceError(f"{lock_path}: Foundation Rust dependencies require a lock file")
    lock = _toml(lock_path)
    entries = lock.get("package", [])
    if not isinstance(entries, list):
        raise ConformanceError(f"{lock_path}: invalid package list")
    for package in sorted(packages):
        matching = [
            entry
            for entry in entries
            if isinstance(entry, dict)
            and entry.get("name") == package
            and entry.get("version") == expected_version
            and isinstance(entry.get("source"), str)
            and entry["source"].startswith(f"git+{FOUNDATION_GIT_URL}?")
            and entry["source"].endswith(f"#{expected_revision}")
        ]
        if len(matching) != 1:
            raise ConformanceError(
                f"{lock_path}: expected one locked {package} {expected_version} from Foundation revision {expected_revision}"
            )


def _verify_npm_lock(
    package_path: Path,
    requirements: dict[str, str],
    expected_version: str,
) -> None:
    if not requirements:
        return
    lock_path = package_path.parent / "package-lock.json"
    if not lock_path.is_file():
        raise ConformanceError(f"{lock_path}: Foundation Web dependencies require a lock file")
    packages = _json(lock_path).get("packages", {})
    if not isinstance(packages, dict):
        raise ConformanceError(f"{lock_path}: invalid packages object")
    for dependency, requirement in sorted(requirements.items()):
        locked = packages.get(f"node_modules/{dependency}")
        if (
            not isinstance(locked, dict)
            or locked.get("version") != expected_version
            or locked.get("resolved") != requirement
            or not isinstance(locked.get("integrity"), str)
            or not locked["integrity"].startswith("sha512-")
        ):
            raise ConformanceError(
                f"{lock_path}: {dependency} is not locked to the declared Foundation asset with SHA-512 integrity"
            )


def _relative_layout_path(value: Any, context: str) -> Path:
    if not isinstance(value, str) or not value:
        raise ConformanceError(f"{context}: expected a non-empty relative path")
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise ConformanceError(f"{context}: path must stay inside the product repository")
    return relative


def _layout(product_root: Path) -> dict[str, Path]:
    path = product_root / "sarmg-layout.toml"
    if not path.exists():
        return {}
    value = _toml(path)
    allowed = {"schema_product", "schema_generated", "web_root"}
    _exact_keys(value, allowed, set(), str(path))
    return {key: _relative_layout_path(item, f"{path}.{key}") for key, item in value.items()}


def _discover_file(product_root: Path, filename: str, configured: Path | None) -> Path | None:
    if configured is not None:
        candidate = product_root / configured
        if not candidate.is_file():
            raise ConformanceError(f"{candidate}: configured layout file does not exist")
        return candidate
    ignored = {".git", "node_modules", "target", "dist", "release"}
    candidates = sorted(
        path
        for path in product_root.rglob(filename)
        if path.is_file() and not any(part in ignored for part in path.relative_to(product_root).parts)
    )
    if len(candidates) > 1:
        raise ConformanceError(
            f"multiple {filename} files found; select one explicitly in sarmg-layout.toml"
        )
    return candidates[0] if candidates else None


def _discover_web_package(product_root: Path, configured: Path | None) -> Path | None:
    if configured is not None:
        package = product_root / configured / "package.json"
        if not package.is_file():
            raise ConformanceError(f"{package}: configured Web root does not contain package.json")
        return package
    candidates: list[Path] = []
    ignored = {".git", "node_modules", "target", "dist", "release"}
    for path in product_root.rglob("package.json"):
        if not path.is_file() or any(
            part in ignored for part in path.relative_to(product_root).parts
        ):
            continue
        package = _json(path)
        dependencies = {
            name
            for section in ("dependencies", "devDependencies", "optionalDependencies")
            for name in (
                package.get(section, {}) if isinstance(package.get(section, {}), dict) else {}
            )
        }
        if any(name.startswith("@sarmg/") for name in dependencies):
            candidates.append(path)
    if len(candidates) > 1:
        raise ConformanceError(
            "multiple Foundation Web package manifests found; select web_root in sarmg-layout.toml"
        )
    return candidates[0] if candidates else None


def _source_files(product_root: Path, suffixes: set[str]) -> Iterable[Path]:
    import os

    # Routing metadata only: client behavior is checked by the Client repository.
    # Never import its policy or require that repository for Server verification.
    client_roots: set[Path] = set()
    client_manifest = product_root / "sarmg-client.toml"
    if client_manifest.is_file():
        for relative in _toml(client_manifest).get("source_roots", []):
            if not isinstance(relative, str) or Path(relative).is_absolute() or ".." in Path(relative).parts:
                raise ConformanceError("client source routing must use relative directories")
            root = (product_root / relative).resolve(strict=True)
            if root == product_root.resolve() or not root.is_relative_to(product_root.resolve()):
                raise ConformanceError("client source routing must not hide the product root")
            client_roots.add(root)
    ignored = {".git", "node_modules", "target", "dist", "release"}
    for directory, directories, files in os.walk(product_root, followlinks=False):
        directories[:] = sorted(name for name in directories if name not in ignored and (Path(directory) / name).resolve() not in client_roots)
        for name in sorted(files):
            path = Path(directory) / name
            if path.is_file() and path.suffix in suffixes:
                yield path


def verify_source(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    product_id = manifest["product_id"]
    findings: list[tuple[str, str]] = []
    advisories: list[dict[str, str]] = []
    observed_versions: set[str] = set()
    observed_revisions: set[str] = set()
    observed_rust_packages: set[str] = set()
    observed_web_requirements: dict[Path, dict[str, str]] = {}
    foundation_packages = _foundation_package_names(foundation_root)
    workspace_dependencies = _workspace_dependencies(product_root)
    expected_version = manifest["foundation"]["version"]
    expected_revision = manifest["foundation"]["git_rev"]
    for path in _source_files(product_root, {".toml"}):
        cargo = _toml(path)
        used_aliases = _used_dependency_aliases(cargo)
        for alias, requirement in _walk_dependency_tables(cargo):
            try:
                resolved = _resolved_rust_requirement(
                    alias, requirement, workspace_dependencies, path
                )
            except ConformanceError as error:
                findings.append(("immutable-foundation-dependencies", str(error)))
                continue
            dependency = _dependency_name(alias, resolved)
            if dependency not in foundation_packages:
                continue
            if alias in used_aliases:
                observed_rust_packages.add(dependency)
            if not isinstance(resolved, dict):
                findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} must use the exact Foundation Git source"))
                continue
            if resolved.get("git") != FOUNDATION_GIT_URL or "path" in resolved:
                findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} does not use the canonical Foundation Git source"))
            revision = resolved.get("rev")
            if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
                findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} lacks a full git rev"))
            else:
                observed_revisions.add(revision)
                if revision != expected_revision:
                    findings.append(("single-foundation-release", f"{path}: {dependency} revision differs from manifest {expected_revision}"))
            version = resolved.get("version")
            if not isinstance(version, str) or not version.startswith("=") or SEMVER.fullmatch(version[1:]) is None:
                findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} lacks an exact version"))
            else:
                observed_versions.add(version[1:])
                if version[1:] != expected_version:
                    findings.append(("single-foundation-release", f"{path}: {dependency} version differs from manifest {expected_version}"))
    for path in _source_files(product_root, {".json"}):
        if path.name != "package.json":
            continue
        package = _json(path)
        for section in ("dependencies", "devDependencies", "optionalDependencies"):
            dependencies = package.get(section, {})
            if not isinstance(dependencies, dict):
                continue
            for dependency, requirement in dependencies.items():
                if dependency not in foundation_packages:
                    continue
                if not isinstance(requirement, str):
                    findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} must use one immutable release asset URL"))
                    continue
                match = FOUNDATION_RELEASE_URL.fullmatch(requirement)
                expected_slug = dependency.removeprefix("@sarmg/")
                if match is None or match.groups() != (
                    expected_version,
                    expected_slug,
                    expected_version,
                ):
                    findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} must use its v{expected_version} Foundation release asset"))
                    continue
                observed_versions.add(expected_version)
                observed_web_requirements.setdefault(path, {})[dependency] = requirement
    if observed_versions and observed_versions != {expected_version}:
        findings.append(("single-foundation-release", f"observed Foundation versions {sorted(observed_versions)} differ from manifest {expected_version}"))
    if observed_revisions and observed_revisions != {expected_revision}:
        findings.append(("single-foundation-release", f"observed Foundation revisions differ from manifest {expected_revision}"))
    try:
        _verify_cargo_lock(
            product_root,
            observed_rust_packages,
            expected_version,
            expected_revision,
        )
        for package_path, requirements in observed_web_requirements.items():
            _verify_npm_lock(package_path, requirements, expected_version)
    except ConformanceError as error:
        findings.append(("immutable-foundation-dependencies", str(error)))
    # Text searches are useful migration hints, but cannot prove ownership: comments,
    # fixtures, aliases and generated sources all create false positives. Keep them as
    # non-blocking advisories; executable dependency/schema checks remain the gate.
    route_pattern = re.compile(r"\.route\s*\(\s*[\"'](/(?:api/v2/(?:auth|platform)/|healthz|readyz))")
    policy_pattern = re.compile(r"\b(?:SESSION_COOKIE_NAME|SESSION_IDLE_TIMEOUT|ARGON2_(?:MEMORY|TIME|PARALLELISM))\b")
    for path in _source_files(product_root, {".rs", ".ts", ".tsx", ".js", ".mjs", ".sql", ".swift", ".kt"}):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeError as error:
            raise ConformanceError(f"{path}: source is not UTF-8") from error
        if route_pattern.search(text):
            advisories.append({"rule": "foundation-route-ownership", "path": str(path)})
        if policy_pattern.search(text):
            advisories.append({"rule": "platform-policy-ownership", "path": str(path)})
    if findings:
        raise ConformanceError("source verification failed:\n" + "\n".join(f"- [{rule}] {message}" for rule, message in findings))
    return {
        "product": product_id,
        "advisories": advisories,
    }


def verify_schema(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    product_id = manifest["product_id"]
    control_planes = [
        component
        for component in manifest["components"]
        if component["profile"] == "server-control-plane"
    ]
    needs_composed = bool(control_planes)
    layout = _layout(product_root)
    product_schema = _discover_file(product_root, "product.sql", layout.get("schema_product"))
    generated_schema = _discover_file(
        product_root, "current_schema.sql", layout.get("schema_generated")
    )
    if needs_composed and (product_schema is None or generated_schema is None):
        raise ConformanceError(
            "server-control-plane requires one product.sql and one current_schema.sql; "
            "use sarmg-layout.toml when discovery is ambiguous"
        )
    if needs_composed:
        if len(control_planes) != 1:
            raise ConformanceError("schema verification requires exactly one server-control-plane component")
        from sarmg_schema_compose import ComposeError, compose, schema_capabilities

        component = control_planes[0]
        try:
            expected = compose(
                component["profile"],
                schema_capabilities(component["capabilities"]),
                product_schema,
            )
        except ComposeError as error:
            raise ConformanceError(str(error)) from error
        try:
            actual = generated_schema.read_text(encoding="utf-8")
        except UnicodeError as error:
            raise ConformanceError(f"{generated_schema}: generated schema is not UTF-8") from error
        if actual != expected:
            raise ConformanceError(f"{generated_schema}: generated schema does not match recomposition")
    return {"product": product_id, "status": "verified"}


def verify_web(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    expected = [component["web_profile"] for component in manifest["components"] if "web_profile" in component]
    if not expected:
        return {"product": manifest["product_id"], "status": "not-applicable"}
    layout = _layout(product_root)
    web_root = layout.get("web_root")
    package_path = _discover_web_package(product_root, web_root)
    if package_path is None or not package_path.is_file():
        raise ConformanceError(f"{package_path}: Web Profile requires a package manifest")
    package = _json(package_path)
    expected_version = manifest["foundation"]["version"]
    foundation_packages = _foundation_package_names(foundation_root)
    requirements: dict[str, str] = {}
    for section in ("dependencies", "devDependencies", "optionalDependencies"):
        dependencies = package.get(section, {})
        if isinstance(dependencies, dict):
            for dependency, requirement in dependencies.items():
                if dependency not in foundation_packages:
                    continue
                if not isinstance(requirement, str):
                    raise ConformanceError(f"{package_path}: invalid Foundation dependency {dependency}")
                match = FOUNDATION_RELEASE_URL.fullmatch(requirement)
                expected_slug = dependency.removeprefix("@sarmg/")
                if match is None or match.groups() != (
                    expected_version,
                    expected_slug,
                    expected_version,
                ):
                    raise ConformanceError(
                        f"{package_path}: {dependency} must use its v{expected_version} Foundation release asset"
                    )
                requirements[dependency] = requirement
    _verify_npm_lock(package_path, requirements, expected_version)
    return {"product": manifest["product_id"], "profiles": expected}


def verify_release(
    product_root: Path,
    foundation_root: Path,
    *,
    require_published: bool = False,
) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    release_paths = [product_root / "release.json", product_root / "release" / "release.json"]
    existing = [path for path in release_paths if path.is_file()]
    if not existing:
        if require_published:
            raise ConformanceError("release manifest is required for a publication gate")
        return {
            "product": manifest["product_id"],
            "status": "not-checked",
            "reason": "no release manifest is present in the source tree",
        }
    release = _json(existing[0])
    target = release.get("target")
    server_profiles = [component["profile"] for component in manifest["components"] if component["profile"].startswith("server-")]
    if server_profiles and target != "x86_64-unknown-linux-gnu":
        raise ConformanceError(f"{existing[0]}: formal Server target is not canonical")
    return {"product": manifest["product_id"], "status": "verified", "path": str(existing[0])}


def _registry(foundation_root: Path) -> list[dict[str, Any]]:
    value = _toml(foundation_root / "consumers" / "repositories.toml")
    _exact_keys(value, {"format", "repositories"}, {"format", "repositories"}, "consumer registry")
    if value["format"] != 1 or not isinstance(value["repositories"], list):
        raise ConformanceError("consumer registry: unsupported format")
    return value["repositories"]


def generate_consumer_matrix(foundation_root: Path) -> dict[str, Any]:
    profile_catalog, capability_catalog = load_profiles(foundation_root)
    known_packages = {
        path.parent.name for path in (foundation_root / "rust" / "crates").glob("*/Cargo.toml")
    }
    for path in (foundation_root / "packages").glob("*/package.json"):
        package = _json(path)
        name = package.get("name")
        if isinstance(name, str):
            known_packages.add(name)
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, repository in enumerate(_registry(foundation_root)):
        context = f"consumer registry.repositories[{index}]"
        if not isinstance(repository, dict):
            raise ConformanceError(f"{context}: expected a table")
        expected = {"product", "url", "commit", "foundation_version", "profiles", "capabilities", "packages", "status", "exceptions"}
        _exact_keys(repository, expected, expected, context)
        product = repository["product"]
        if not isinstance(product, str) or IDENTIFIER.fullmatch(product) is None or product in seen:
            raise ConformanceError(f"{context}: invalid or duplicate product")
        seen.add(product)
        if not isinstance(repository["url"], str) or not repository["url"].startswith("https://github.com/"):
            raise ConformanceError(f"{context}: repository URL must be an HTTPS GitHub URL")
        if REVISION.fullmatch(str(repository["commit"])) is None:
            raise ConformanceError(f"{context}: commit must be 40 lowercase hex characters")
        version = repository["foundation_version"]
        if version is not None and (not isinstance(version, str) or SEMVER.fullmatch(version) is None):
            raise ConformanceError(f"{context}: invalid Foundation version")
        profiles = _string_list(repository["profiles"], f"{context}.profiles")
        capabilities = _string_list(repository["capabilities"], f"{context}.capabilities")
        if not set(profiles) <= set(profile_catalog):
            raise ConformanceError(f"{context}: unknown Profiles {sorted(set(profiles) - set(profile_catalog))}")
        if not set(capabilities) <= capability_catalog:
            raise ConformanceError(f"{context}: unknown capabilities {sorted(set(capabilities) - capability_catalog)}")
        packages = repository["packages"]
        if not isinstance(packages, list) or any(not isinstance(item, str) or not item for item in packages) or len(packages) != len(set(packages)):
            raise ConformanceError(f"{context}.packages: invalid package list")
        if not set(packages) <= known_packages:
            raise ConformanceError(f"{context}: unknown packages {sorted(set(packages) - known_packages)}")
        status = repository["status"]
        if status not in MATRIX_STATUSES:
            raise ConformanceError(f"{context}: invalid migration status")
        exceptions = _string_list(repository["exceptions"], f"{context}.exceptions")
        if status == "conforming" and (exceptions or version is None or not profiles):
            raise ConformanceError(f"{context}: conforming evidence is incomplete")
        if status == "temporary-exception" and not exceptions:
            raise ConformanceError(f"{context}: temporary-exception requires exception ids")
        entries.append(
            {
                "product": product,
                "commit": repository["commit"],
                "foundation_version": version,
                "profiles": sorted(profiles),
                "capabilities": sorted(capabilities),
                "packages": sorted(packages),
                "status": status,
                "exceptions": sorted(exceptions),
            }
        )
    entries.sort(key=lambda item: item["product"])
    return {
        "$schema": "./consumer-matrix.schema.json",
        "format": "sarmg.consumer-matrix.v2",
        "platform_generation": 1,
        "foundation_version": _foundation_version(foundation_root),
        "consumers": entries,
    }


def _foundation_version(foundation_root: Path) -> str:
    cargo = _toml(foundation_root / "Cargo.toml")
    version = cargo.get("workspace", {}).get("package", {}).get("version")
    if not isinstance(version, str) or SEMVER.fullmatch(version) is None:
        raise ConformanceError("Cargo.toml: invalid Foundation workspace version")
    return version


def verify_consumer_registry(foundation_root: Path) -> dict[str, Any]:
    generated = generate_consumer_matrix(foundation_root)
    checked_in = _json(foundation_root / "consumers" / "consumer-matrix.json")
    if checked_in != generated:
        raise ConformanceError("consumer-matrix.json is stale; run generate-consumer-matrix")
    return generated


def verify_foundation(foundation_root: Path) -> dict[str, Any]:
    profiles, capabilities = load_profiles(foundation_root)
    schema = _json(foundation_root / "schemas" / "sarmg-product.schema.json")
    schema_profiles = set(schema["properties"]["components"]["items"]["properties"]["profile"]["enum"])
    if schema_profiles != set(profiles):
        raise ConformanceError("sarmg-product Schema profile enum is stale")
    own_packages = _foundation_package_names(foundation_root)
    for path in sorted((foundation_root / "rust" / "crates").glob("*/Cargo.toml")):
        cargo = _toml(path)
        for alias, requirement in _walk_dependency_tables(cargo):
            dependency = _dependency_name(alias, requirement)
            if isinstance(requirement, dict) and "git" in requirement:
                raise ConformanceError(
                    f"{path}: Foundation must not use Git dependency {alias!r} ({dependency!r})"
                )
            if isinstance(requirement, dict) and "path" in requirement:
                dependency_root = (path.parent / requirement["path"]).resolve(strict=True)
                if not dependency_root.is_relative_to(foundation_root.resolve()) or dependency not in own_packages:
                    raise ConformanceError(
                        f"{path}: path dependency {alias!r} ({dependency!r}) is outside Foundation ownership"
                    )
    return {"profiles": sorted(profiles), "capabilities": sorted(capabilities)}
