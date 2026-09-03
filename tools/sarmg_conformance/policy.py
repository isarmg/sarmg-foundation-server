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
PRODUCT_IDS = {
    "dufs-ram",
    "host-monitoring",
    "media-backup",
    "sarmg-upgrade",
    "sentinel-monitor",
    "sunshine-manager",
}
MATRIX_STATUSES = {
    "not-migrated",
    "migration-in-progress",
    "conforming",
    "non-conforming",
    "temporary-exception",
}
FOUNDATION_ROUTES = ("/api/v2/auth/", "/api/v2/platform/", "/healthz", "/readyz")


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
        if identifier in PRODUCT_IDS:
            raise ConformanceError(f"{path}: product-specific profile names are forbidden")
        if value["kind"] not in {"server", "agent", "tool", "web"}:
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
        "desktop-agent",
        "mobile-agent",
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


def _load_exceptions(foundation_root: Path, product_id: str) -> tuple[dict[str, dict[str, Any]], set[str]]:
    directory = foundation_root / "exceptions" / product_id
    if not directory.exists():
        return {}, set()
    exceptions: dict[str, dict[str, Any]] = {}
    rules: set[str] = set()
    for path in sorted(directory.glob("*.toml")):
        value = _toml(path)
        expected = {"id", "product", "reason", "introduced_at", "must_remove_before", "security_reduction", "rules"}
        _exact_keys(value, expected, expected, str(path))
        if value["id"] != path.stem or value["product"] != product_id:
            raise ConformanceError(f"{path}: exception identity mismatch")
        if not isinstance(value["reason"], str) or not value["reason"].strip():
            raise ConformanceError(f"{path}: exception reason is required")
        if any(not isinstance(value[key], str) or SEMVER.fullmatch(value[key]) is None for key in ("introduced_at", "must_remove_before")):
            raise ConformanceError(f"{path}: exception versions must be SemVer")
        current = _semver_core(_foundation_version(foundation_root))
        introduced = _semver_core(value["introduced_at"])
        removal = _semver_core(value["must_remove_before"])
        if not introduced <= current < removal or removal > (1, 0, 0):
            raise ConformanceError(f"{path}: exception is not active or extends beyond Foundation 1.0")
        if value["security_reduction"] is not False:
            raise ConformanceError(f"{path}: security-reducing exceptions are forbidden")
        exception_rules = _string_list(value["rules"], f"{path}.rules", allow_empty=False)
        if value["id"] in exceptions:
            raise ConformanceError(f"{path}: duplicate exception id")
        exceptions[value["id"]] = value
        rules.update(exception_rules)
    return exceptions, rules


def _semver_core(value: str) -> tuple[int, int, int]:
    core = value.split("+", 1)[0].split("-", 1)[0]
    major, minor, patch = core.split(".")
    return int(major), int(minor), int(patch)


def _walk_dependency_tables(value: Any, key: str = "") -> Iterable[tuple[str, Any]]:
    if not isinstance(value, dict):
        return
    if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
        yield from value.items()
    for nested_key, nested in value.items():
        if isinstance(nested, dict):
            yield from _walk_dependency_tables(nested, nested_key)


def _source_files(product_root: Path, suffixes: set[str]) -> Iterable[Path]:
    ignored = {".git", "node_modules", "target", "dist", "release"}
    for path in sorted(product_root.rglob("*")):
        if any(part in ignored for part in path.relative_to(product_root).parts):
            continue
        if path.is_file() and path.suffix in suffixes:
            yield path


def verify_source(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    product_id = manifest["product_id"]
    exceptions, allowed_rules = _load_exceptions(foundation_root, product_id)
    findings: list[tuple[str, str]] = []
    observed_versions: set[str] = set()
    observed_revisions: set[str] = set()
    for path in _source_files(product_root, {".toml"}):
        cargo = _toml(path)
        features = cargo.get("features", {})
        if isinstance(features, dict):
            for feature in features:
                if feature in PRODUCT_IDS or any(product in feature for product in PRODUCT_IDS):
                    findings.append(("no-product-features", f"{path}: product-named Cargo feature {feature!r}"))
        for dependency, requirement in _walk_dependency_tables(cargo):
            if not dependency.startswith("sarmg-"):
                continue
            if isinstance(requirement, dict) and "path" in requirement:
                findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} uses a path dependency"))
            if isinstance(requirement, dict) and "git" in requirement:
                if REVISION.fullmatch(str(requirement.get("rev", ""))) is None:
                    findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} lacks a full git rev"))
                else:
                    observed_revisions.add(requirement["rev"])
                version = requirement.get("version")
                if not isinstance(version, str) or not version.startswith("=") or SEMVER.fullmatch(version[1:]) is None:
                    findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} lacks an exact version"))
                else:
                    observed_versions.add(version[1:])
    for path in _source_files(product_root, {".json"}):
        if path.name != "package.json":
            continue
        package = _json(path)
        for section in ("dependencies", "devDependencies", "optionalDependencies"):
            dependencies = package.get(section, {})
            if not isinstance(dependencies, dict):
                continue
            for dependency, requirement in dependencies.items():
                if dependency.startswith("@sarmg/") and isinstance(requirement, str) and requirement.startswith(("file:", "workspace:")):
                    findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} uses {requirement!r}"))
                if dependency.startswith("@sarmg/") and isinstance(requirement, str):
                    match = re.search(r"/releases/download/v([^/]+)/[^/]+-([^/]+)\.tgz$", requirement)
                    if match is not None:
                        tag_version, asset_version = match.groups()
                        if tag_version != asset_version or SEMVER.fullmatch(tag_version) is None:
                            findings.append(("immutable-foundation-dependencies", f"{path}: {dependency} release URL is inconsistent"))
                        else:
                            observed_versions.add(tag_version)
    expected_version = manifest["foundation"]["version"]
    expected_revision = manifest["foundation"]["git_rev"]
    if observed_versions and observed_versions != {expected_version}:
        findings.append(("single-foundation-release", f"observed Foundation versions {sorted(observed_versions)} differ from manifest {expected_version}"))
    if observed_revisions and observed_revisions != {expected_revision}:
        findings.append(("single-foundation-release", f"observed Foundation revisions differ from manifest {expected_revision}"))
    route_pattern = re.compile(r"\.route\s*\(\s*[\"'](/(?:api/v2/(?:auth|platform)/|healthz|readyz))")
    ddl_pattern = re.compile(r"CREATE\s+(?:TABLE|INDEX|TRIGGER)\s+(?:IF\s+NOT\s+EXISTS\s+)?(?:auth_users|auth_sessions|browser_sessions|_sarmg_[a-z0-9_]+)", re.IGNORECASE)
    policy_pattern = re.compile(r"\b(?:SESSION_COOKIE_NAME|SESSION_IDLE_TIMEOUT|ARGON2_(?:MEMORY|TIME|PARALLELISM))\b")
    for path in _source_files(product_root, {".rs", ".ts", ".tsx", ".js", ".mjs", ".sql"}):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeError as error:
            raise ConformanceError(f"{path}: source is not UTF-8") from error
        if route_pattern.search(text):
            findings.append(("foundation-route-ownership", f"{path}: product registers a Foundation-owned route"))
        # Historical DDL is owned by the offline upgrade repository. It must not
        # be mistaken for a second online platform implementation.
        if product_id != "sarmg-upgrade" and ddl_pattern.search(text):
            findings.append(("platform-schema-ownership", f"{path}: product defines platform/admin DDL"))
        if policy_pattern.search(text):
            findings.append(("platform-policy-ownership", f"{path}: product defines a platform security constant"))
    active = [(rule, message) for rule, message in findings if rule not in allowed_rules]
    if active:
        raise ConformanceError("source verification failed:\n" + "\n".join(f"- [{rule}] {message}" for rule, message in active))
    return {
        "product": product_id,
        "suppressed_findings": len(findings) - len(active),
        "exceptions": sorted(exceptions),
    }


def verify_schema(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    product_id = manifest["product_id"]
    _, allowed_rules = _load_exceptions(foundation_root, product_id)
    needs_composed = any(component["profile"] == "server-control-plane" for component in manifest["components"])
    product_schema = product_root / "schema" / "product.sql"
    generated_schema = product_root / "schema" / "generated" / "current_schema.sql"
    if needs_composed and (not product_schema.is_file() or not generated_schema.is_file()):
        if "generated-schema" not in allowed_rules:
            raise ConformanceError("server-control-plane requires schema/product.sql and generated/current_schema.sql")
        return {"product": product_id, "status": "temporary-exception"}
    if product_schema.is_file():
        text = product_schema.read_text(encoding="utf-8")
        if re.search(r"CREATE\s+(?:TABLE|INDEX|TRIGGER)\s+(?:IF\s+NOT\s+EXISTS\s+)?_sarmg_", text, re.IGNORECASE):
            raise ConformanceError(f"{product_schema}: product Schema uses reserved _sarmg_ prefix")
    if generated_schema.is_file():
        text = generated_schema.read_text(encoding="utf-8")
        if not text.startswith("-- Generated by sarmg-schema-compose; DO NOT EDIT.\n"):
            raise ConformanceError(f"{generated_schema}: missing generated-file identity")
    return {"product": product_id, "status": "verified"}


def verify_web(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    expected = [component["web_profile"] for component in manifest["components"] if "web_profile" in component]
    if not expected:
        return {"product": manifest["product_id"], "status": "not-applicable"}
    package_path = product_root / "clients" / "web" / "package.json"
    if not package_path.is_file():
        raise ConformanceError(f"{package_path}: Web Profile requires a package manifest")
    package = _json(package_path)
    for section in ("dependencies", "devDependencies"):
        dependencies = package.get(section, {})
        if isinstance(dependencies, dict):
            for dependency, requirement in dependencies.items():
                if dependency.startswith("@sarmg/") and isinstance(requirement, str) and requirement.startswith(("file:", "workspace:")):
                    raise ConformanceError(f"{package_path}: mutable Foundation dependency {dependency}={requirement}")
    return {"product": manifest["product_id"], "profiles": expected}


def verify_release(product_root: Path, foundation_root: Path) -> dict[str, Any]:
    manifest = verify_manifest(product_root, foundation_root)
    release_paths = [product_root / "release.json", product_root / "release" / "release.json"]
    existing = [path for path in release_paths if path.is_file()]
    if not existing:
        return {"product": manifest["product_id"], "status": "not-published"}
    release = _json(existing[0])
    target = release.get("target")
    server_profiles = [component["profile"] for component in manifest["components"] if component["profile"].startswith("server-")]
    if server_profiles and target is not None and target != "x86_64-unknown-linux-gnu":
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
    registered = {item["product"] for item in generated["consumers"]}
    if registered != PRODUCT_IDS:
        raise ConformanceError(f"consumer registry set differs: {sorted(registered)}")
    for entry in generated["consumers"]:
        exception_values, _ = _load_exceptions(foundation_root, entry["product"])
        if set(entry["exceptions"]) != set(exception_values):
            raise ConformanceError(f"{entry['product']}: matrix exception ids differ from exception files")
    return generated


def verify_baselines(foundation_root: Path) -> dict[str, dict[str, Any]]:
    matrix = generate_consumer_matrix(foundation_root)
    consumers = {entry["product"]: entry for entry in matrix["consumers"]}
    baselines: dict[str, dict[str, Any]] = {}
    always = {
        "format",
        "product",
        "source_commit",
        "product_version",
        "state_kind",
        "foundation_version",
        "foundation_rev",
        "fixture_status",
    }
    schema_keys = {"schema_revision", "schema_sha256", "fixture"}
    for path in sorted((foundation_root / "consumers" / "baselines").glob("*.toml")):
        value = _toml(path)
        if value.get("state_kind") == "none":
            _exact_keys(value, always, always, str(path))
        else:
            _exact_keys(value, always | schema_keys, always | schema_keys, str(path))
            revision = value["schema_revision"]
            if isinstance(revision, bool) or not isinstance(revision, int) or revision < 1:
                raise ConformanceError(f"{path}: schema_revision must be a positive integer")
            digest = value["schema_sha256"]
            if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
                raise ConformanceError(f"{path}: schema_sha256 must be 64 lowercase hex characters")
            fixture = value["fixture"]
            if not isinstance(fixture, str) or not fixture.startswith("sarmg-upgrade/tests/fixtures/sources/"):
                raise ConformanceError(f"{path}: fixture must name an immutable sarmg-upgrade source fixture")
        product = value["product"]
        if value["format"] != 1 or product != path.stem or product in baselines:
            raise ConformanceError(f"{path}: invalid or duplicate baseline identity")
        if product not in consumers:
            raise ConformanceError(f"{path}: product is absent from the consumer registry")
        if REVISION.fullmatch(str(value["source_commit"])) is None:
            raise ConformanceError(f"{path}: invalid source revision")
        if not isinstance(value["product_version"], str) or SEMVER.fullmatch(value["product_version"]) is None:
            raise ConformanceError(f"{path}: invalid product version")
        if (
            not isinstance(value["foundation_version"], str)
            or SEMVER.fullmatch(value["foundation_version"]) is None
        ):
            raise ConformanceError(f"{path}: invalid Foundation version")
        if REVISION.fullmatch(str(value["foundation_rev"])) is None:
            raise ConformanceError(f"{path}: invalid Foundation revision")
        if value["fixture_status"] not in {
            "pending-sanitized-fixture",
            "schema-only",
            "sanitized-current-state",
            "not-applicable",
        }:
            raise ConformanceError(f"{path}: invalid fixture status")
        if value["state_kind"] != "none" and value["fixture_status"] != "sanitized-current-state":
            raise ConformanceError(f"{path}: persistent products require a sanitized current-state fixture")
        baselines[product] = value
    if set(baselines) != set(consumers):
        raise ConformanceError(f"baseline product set differs: {sorted(baselines)}")
    return baselines


def verify_foundation(foundation_root: Path) -> dict[str, Any]:
    profiles, capabilities = load_profiles(foundation_root)
    readme = (foundation_root / "README.md").read_text(encoding="utf-8")
    workflow = (foundation_root / "docs" / "project-workflow.md").read_text(encoding="utf-8")
    if "上游平台规范" not in readme or "构建期中央平台" not in readme:
        raise ConformanceError("README.md: upstream build-time platform position is missing")
    for forbidden in ("只有一个真实消费者", "等待第二消费者证据", "不把 Foundation 变成中央平台"):
        if forbidden in workflow:
            raise ConformanceError(f"project-workflow.md: obsolete admission rule remains: {forbidden}")
    adr_root = foundation_root / "docs" / "architecture"
    missing = [number for number in range(1, 9) if not list(adr_root.glob(f"ADR-{number:04d}-*.md"))]
    if missing:
        raise ConformanceError(f"architecture: missing ADRs {missing}")
    schema = _json(foundation_root / "schemas" / "sarmg-product.schema.json")
    schema_profiles = set(schema["properties"]["components"]["items"]["properties"]["profile"]["enum"])
    if schema_profiles != set(profiles):
        raise ConformanceError("sarmg-product Schema profile enum is stale")
    for path in sorted((foundation_root / "rust" / "crates").glob("*/Cargo.toml")):
        cargo = _toml(path)
        for dependency, _ in _walk_dependency_tables(cargo):
            if dependency in PRODUCT_IDS:
                raise ConformanceError(f"{path}: Foundation depends on product crate {dependency}")
        features = cargo.get("features", {})
        if isinstance(features, dict) and any(product in feature for feature in features for product in PRODUCT_IDS):
            raise ConformanceError(f"{path}: product-named Feature is forbidden")
    verify_consumer_registry(foundation_root)
    verify_baselines(foundation_root)
    return {"profiles": sorted(profiles), "capabilities": sorted(capabilities)}
