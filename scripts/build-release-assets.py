#!/usr/bin/env python3
"""Build one verified, deterministic Foundation release asset tree."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from foundation_policy import (  # noqa: E402
    CURRENT_VERSION,
    NODE_VERSION,
    PNPM_VERSION,
    RUST_VERSION,
    FoundationPolicyError,
    check_repository,
)
from sarmg_package_artifacts import (  # noqa: E402
    PackagePolicyError,
    discover_packages,
    package_release,
)
from sarmg_release import (  # noqa: E402
    BuildIdentity,
    PolicyError,
    build_manifest,
    render_manifest,
    verify_release_tree,
)


SHA256 = re.compile(r"[0-9a-f]{64}")
SOURCE_REVISION = re.compile(r"[0-9a-f]{40}")
MAX_SOURCE_FILE_BYTES = 4 * 1024 * 1024
TOOL_BUNDLE_PATHS = (
    Path("LICENSE"),
    Path("docs/operations.md"),
    Path("scripts/sarmg-release.py"),
    Path("tools/sarmg_release/__init__.py"),
    Path("tools/sarmg_release/cli.py"),
    Path("tools/sarmg_release/release.py"),
)


class ReleaseBuildError(RuntimeError):
    """Release inputs or generated assets are unsafe or inconsistent."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def checked_output(command: list[str]) -> str:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def verify_source(source_revision: str, tag: str) -> None:
    if SOURCE_REVISION.fullmatch(source_revision) is None:
        raise ReleaseBuildError("source revision must be a full lowercase Git commit")
    if tag != f"v{CURRENT_VERSION}":
        raise ReleaseBuildError(f"tag must be exactly v{CURRENT_VERSION}")
    if checked_output(["git", "rev-parse", "HEAD"]) != source_revision:
        raise ReleaseBuildError("source revision does not equal the checked-out commit")
    if checked_output(["git", "tag", "--points-at", "HEAD"]).splitlines().count(tag) != 1:
        raise ReleaseBuildError("the exact release tag does not point at HEAD")
    if checked_output(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"]
    ):
        raise ReleaseBuildError("tracked or untracked source changes are forbidden during release")


def prepare_output(output: Path) -> tuple[Path, Path]:
    output = output.resolve()
    if output == ROOT or output == Path(output.anchor):
        raise ReleaseBuildError("refusing to use a broad release output path")
    if output.exists():
        metadata = output.lstat()
        if not stat.S_ISDIR(metadata.st_mode) or output.is_symlink():
            raise ReleaseBuildError("release output must be a real directory")
        if any(output.iterdir()):
            raise ReleaseBuildError("release output must be empty")
    else:
        output.mkdir(parents=True)
    artifacts = output / "artifacts"
    artifacts.mkdir()
    return output, artifacts


def add_regular_file(archive: tarfile.TarFile, source: Path, name: PurePosixPath) -> None:
    metadata = source.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_size > MAX_SOURCE_FILE_BYTES
    ):
        raise ReleaseBuildError(f"{source}: tool bundle source is unsafe or too large")
    info = tarfile.TarInfo(name.as_posix())
    info.size = metadata.st_size
    info.mode = 0o755 if source == ROOT / "scripts/sarmg-release.py" else 0o644
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = "root"
    info.gname = "root"
    with source.open("rb") as stream:
        archive.addfile(info, stream)


def build_tool_bundle(artifacts: Path) -> Path:
    name = f"sarmg-release-tool-{CURRENT_VERSION}"
    destination = artifacts / f"{name}.tar.gz"
    with destination.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for relative in TOOL_BUNDLE_PATHS:
                    add_regular_file(archive, ROOT / relative, PurePosixPath(name) / relative.as_posix())
    return destination


def state_contract(source_revision: str) -> dict[str, Any]:
    return {
        "contract_version": 1,
        "application": "sarmg-foundation",
        "application_version": CURRENT_VERSION,
        "source_revision": source_revision,
        "schema": None,
        "maintenance_locks": [],
        "resources": [],
        "external_requirements": [],
        "companion_contracts": [],
    }


def inventory(identity: BuildIdentity, artifacts: Path) -> dict[str, Any]:
    components = []
    for package in discover_packages(ROOT):
        components.append({"name": package.name, "version": package.version, "kind": "npm"})
    cargo = json.loads(checked_output(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"]))
    for package in cargo["packages"]:
        if package["name"].startswith("sarmg-"):
            components.append({"name": package["name"], "version": package["version"], "kind": "rust"})
    described = []
    for path in sorted(artifacts.iterdir()):
        if path.name in {"SHA256SUMS", "build-inventory.json"}:
            continue
        described.append({"file": path.name, "bytes": path.stat().st_size, "sha256": sha256_file(path)})
    return {
        "format": "sarmg.build-inventory.v1",
        "identity": identity.to_dict(),
        "toolchains": {
            "node": NODE_VERSION,
            "pnpm": PNPM_VERSION,
            "rust": RUST_VERSION,
        },
        "lockfiles": [
            {"path": name, "sha256": sha256_file(ROOT / name)}
            for name in ("Cargo.lock", "pnpm-lock.yaml")
        ],
        "components": sorted(components, key=lambda item: (item["kind"], item["name"])),
        "artifacts": described,
    }


def checksums(artifacts: Path) -> None:
    entries = []
    for path in sorted(artifacts.iterdir()):
        if path.name == "SHA256SUMS":
            continue
        entries.append(f"{sha256_file(path)}  {path.name}\n")
    (artifacts / "SHA256SUMS").write_text("".join(entries), encoding="ascii")


def build(output: Path, source_revision: str, tag: str, target: str) -> Path:
    check_repository(ROOT)
    verify_source(source_revision, tag)
    output, artifacts = prepare_output(output)
    package_release(ROOT, discover_packages(ROOT), artifacts)
    build_tool_bundle(artifacts)

    contract_path = artifacts / "state-contract.json"
    write_json(contract_path, state_contract(source_revision))
    contract_hash = sha256_file(contract_path)
    if SHA256.fullmatch(contract_hash) is None:  # pragma: no cover - hashlib contract
        raise AssertionError(contract_hash)
    identity = BuildIdentity(
        product="sarmg-foundation",
        version=CURRENT_VERSION,
        source_revision=source_revision,
        target=target,
        state_contract_sha256=contract_hash,
    )
    write_json(artifacts / "release-identity.json", identity.to_dict())
    write_json(artifacts / "build-inventory.json", inventory(identity, artifacts))
    checksums(artifacts)

    manifest = build_manifest(artifacts, identity)
    manifest_path = output / "release-tree.json"
    manifest_path.write_bytes(render_manifest(manifest))
    verify_release_tree(artifacts, manifest)
    print(f"release assets: passed {artifacts}")
    print(f"release manifest: {manifest_path}")
    return output


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="build-release-assets.py")
    parser.add_argument("--version", action="version", version=f"%(prog)s {CURRENT_VERSION}")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", default="source-any")
    arguments = parser.parse_args(argv)
    try:
        build(arguments.output, arguments.source_revision, arguments.tag, arguments.target)
        return 0
    except (
        FoundationPolicyError,
        PackagePolicyError,
        PolicyError,
        ReleaseBuildError,
        OSError,
        subprocess.SubprocessError,
        UnicodeError,
    ) as error:
        print(f"release assets: FAILED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
