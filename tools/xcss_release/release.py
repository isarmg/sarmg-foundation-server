"""Deterministic release identity and exact release-tree verification."""

from __future__ import annotations

import hashlib
import json
import os
import re
import stat
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any


FORMAT = "sarmg.release-tree.v1"
MAX_MANIFEST_BYTES = 1024 * 1024
MAX_FILES = 10_000
MAX_TREE_ENTRIES = 20_000
MAX_FILE_BYTES = 16 * 1024 * 1024 * 1024
MAX_PATH_BYTES = 1024
HASH_CHUNK_BYTES = 1024 * 1024

PRODUCT = re.compile(r"[a-z][a-z0-9-]{0,62}")
SEMVER = re.compile(
    r"(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)
SOURCE_REVISION = re.compile(r"[0-9a-f]{40}")
TARGET = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,127}")
MODE = re.compile(r"0[0-7]{3}")
SHA256 = re.compile(r"[0-9a-f]{64}")


class PolicyError(RuntimeError):
    """Input is not a valid current release identity or release tree."""


def _exact_keys(value: dict[str, Any], expected: set[str], context: str) -> None:
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        details = []
        if missing:
            details.append(f"missing keys {missing}")
        if unknown:
            details.append(f"unknown keys {unknown}")
        raise PolicyError(f"{context}: {', '.join(details)}")


def _literal_string(value: Any, pattern: re.Pattern[str], context: str) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise PolicyError(f"{context}: invalid literal")
    return value


def _release_path(value: Any, context: str) -> str:
    if not isinstance(value, str):
        raise PolicyError(f"{context}: path must be a string")
    if not value or len(value.encode("utf-8")) > MAX_PATH_BYTES:
        raise PolicyError(f"{context}: path is empty or exceeds {MAX_PATH_BYTES} bytes")
    if "\\" in value or "\x00" in value:
        raise PolicyError(f"{context}: backslashes and NUL bytes are forbidden")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts):
        raise PolicyError(f"{context}: path must be canonical and relative")
    if path.as_posix() != value:
        raise PolicyError(f"{context}: path must use canonical POSIX spelling")
    return value


@dataclass(frozen=True)
class BuildIdentity:
    product: str
    version: str
    source_revision: str
    target: str
    state_contract_sha256: str

    def __post_init__(self) -> None:
        _literal_string(self.product, PRODUCT, "identity.product")
        _literal_string(self.version, SEMVER, "identity.version")
        _literal_string(
            self.source_revision, SOURCE_REVISION, "identity.source_revision"
        )
        _literal_string(self.target, TARGET, "identity.target")
        if "-" not in self.target:
            raise PolicyError("identity.target: expected an explicit target triple")
        _literal_string(
            self.state_contract_sha256,
            SHA256,
            "identity.state_contract_sha256",
        )

    def to_dict(self) -> dict[str, str]:
        return {
            "product": self.product,
            "version": self.version,
            "source_revision": self.source_revision,
            "target": self.target,
            "state_contract_sha256": self.state_contract_sha256,
        }

    @classmethod
    def from_dict(cls, value: Any) -> BuildIdentity:
        if not isinstance(value, dict):
            raise PolicyError("identity: expected an object")
        _exact_keys(
            value,
            {
                "product",
                "version",
                "source_revision",
                "target",
                "state_contract_sha256",
            },
            "identity",
        )
        return cls(
            product=value["product"],
            version=value["version"],
            source_revision=value["source_revision"],
            target=value["target"],
            state_contract_sha256=value["state_contract_sha256"],
        )


@dataclass(frozen=True)
class ReleaseFile:
    path: str
    mode: str
    size: int
    sha256: str

    def __post_init__(self) -> None:
        _release_path(self.path, "file.path")
        _literal_string(self.mode, MODE, f"{self.path}.mode")
        if isinstance(self.size, bool) or not isinstance(self.size, int):
            raise PolicyError(f"{self.path}.size: expected an integer")
        if not 0 <= self.size <= MAX_FILE_BYTES:
            raise PolicyError(
                f"{self.path}.size: expected 0..{MAX_FILE_BYTES} bytes"
            )
        _literal_string(self.sha256, SHA256, f"{self.path}.sha256")

    def to_dict(self) -> dict[str, str | int]:
        return {
            "path": self.path,
            "mode": self.mode,
            "size": self.size,
            "sha256": self.sha256,
        }

    @classmethod
    def from_dict(cls, value: Any, index: int) -> ReleaseFile:
        if not isinstance(value, dict):
            raise PolicyError(f"files[{index}]: expected an object")
        _exact_keys(value, {"path", "mode", "size", "sha256"}, f"files[{index}]")
        return cls(
            path=value["path"],
            mode=value["mode"],
            size=value["size"],
            sha256=value["sha256"],
        )


@dataclass(frozen=True)
class ReleaseManifest:
    identity: BuildIdentity
    files: tuple[ReleaseFile, ...]
    format: str = FORMAT

    def __post_init__(self) -> None:
        if self.format != FORMAT:
            raise PolicyError(f"format: expected {FORMAT!r}")
        if not self.files:
            raise PolicyError("files: release tree must contain at least one file")
        if len(self.files) > MAX_FILES:
            raise PolicyError(f"files: exceeds the {MAX_FILES}-file policy limit")
        paths = [entry.path for entry in self.files]
        if paths != sorted(paths):
            raise PolicyError("files: entries must be sorted by canonical path")
        if len(paths) != len(set(paths)):
            raise PolicyError("files: duplicate paths are forbidden")

    def to_dict(self) -> dict[str, Any]:
        return {
            "format": self.format,
            "identity": self.identity.to_dict(),
            "files": [entry.to_dict() for entry in self.files],
        }


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PolicyError(f"JSON: duplicate object key {key!r}")
        result[key] = value
    return result


def parse_manifest(data: bytes | str, source: str = "manifest") -> ReleaseManifest:
    raw = data.encode("utf-8") if isinstance(data, str) else data
    if len(raw) > MAX_MANIFEST_BYTES:
        raise PolicyError(
            f"{source}: exceeds the {MAX_MANIFEST_BYTES}-byte policy limit"
        )
    try:
        text = raw.decode("utf-8")
        value = json.loads(
            text,
            object_pairs_hook=_unique_object,
            parse_constant=lambda constant: (_ for _ in ()).throw(
                PolicyError(f"JSON: invalid numeric constant {constant}")
            ),
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PolicyError(f"{source}: invalid UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise PolicyError(f"{source}: expected one JSON object")
    _exact_keys(value, {"format", "identity", "files"}, source)
    if value["format"] != FORMAT:
        raise PolicyError(f"{source}.format: expected {FORMAT!r}")
    if not isinstance(value["files"], list):
        raise PolicyError(f"{source}.files: expected an array")
    identity = BuildIdentity.from_dict(value["identity"])
    files = tuple(
        ReleaseFile.from_dict(entry, index)
        for index, entry in enumerate(value["files"])
    )
    return ReleaseManifest(identity=identity, files=files)


def render_manifest(manifest: ReleaseManifest) -> bytes:
    return (
        json.dumps(
            manifest.to_dict(),
            ensure_ascii=False,
            indent=2,
            separators=(",", ": "),
        )
        + "\n"
    ).encode("utf-8")


def _hash_regular_file(path: Path, expected_stat: os.stat_result) -> str:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or opened.st_nlink != 1:
            raise PolicyError(f"{path}: file must be regular and have exactly one link")
        if (opened.st_dev, opened.st_ino) != (expected_stat.st_dev, expected_stat.st_ino):
            raise PolicyError(f"{path}: file changed while it was inspected")
        digest = hashlib.sha256()
        bytes_read = 0
        while chunk := os.read(descriptor, HASH_CHUNK_BYTES):
            bytes_read += len(chunk)
            if bytes_read > MAX_FILE_BYTES:
                raise PolicyError(f"{path}: file grew beyond the size policy limit")
            digest.update(chunk)
        finished = os.fstat(descriptor)
        if (
            finished.st_size != opened.st_size
            or finished.st_mtime_ns != opened.st_mtime_ns
            or finished.st_ctime_ns != opened.st_ctime_ns
        ):
            raise PolicyError(f"{path}: file changed while it was hashed")
        try:
            current_path = path.lstat()
        except OSError as error:
            raise PolicyError(f"{path}: path changed while it was hashed: {error}") from error
        if (
            not stat.S_ISREG(current_path.st_mode)
            or current_path.st_nlink != 1
            or (current_path.st_dev, current_path.st_ino, current_path.st_size)
            != (finished.st_dev, finished.st_ino, finished.st_size)
        ):
            raise PolicyError(f"{path}: path was replaced while it was hashed")
        return digest.hexdigest()
    finally:
        os.close(descriptor)


def scan_release_tree(root: Path) -> tuple[ReleaseFile, ...]:
    try:
        metadata = root.lstat()
    except OSError as error:
        raise PolicyError(f"{root}: cannot inspect release root: {error}") from error
    if not stat.S_ISDIR(metadata.st_mode) or root.is_symlink():
        raise PolicyError(f"{root}: release root must be a real directory")
    files: list[ReleaseFile] = []
    pending: list[tuple[Path, PurePosixPath]] = [(root, PurePosixPath())]
    entries_seen = 0
    while pending:
        current, prefix = pending.pop()
        try:
            entries = sorted(
                os.scandir(current), key=lambda entry: entry.name.encode("utf-8")
            )
        except (OSError, UnicodeError) as error:
            raise PolicyError(f"{current}: cannot enumerate release tree: {error}") from error
        directories: list[tuple[Path, PurePosixPath]] = []
        for entry in entries:
            entries_seen += 1
            if entries_seen > MAX_TREE_ENTRIES:
                raise PolicyError(
                    f"release tree exceeds the {MAX_TREE_ENTRIES}-entry policy limit"
                )
            relative = prefix / entry.name
            canonical = _release_path(relative.as_posix(), "release tree")
            try:
                entry_metadata = entry.stat(follow_symlinks=False)
            except OSError as error:
                raise PolicyError(f"{canonical}: cannot inspect entry: {error}") from error
            if stat.S_ISLNK(entry_metadata.st_mode):
                raise PolicyError(f"{canonical}: symbolic links are forbidden")
            if stat.S_ISDIR(entry_metadata.st_mode):
                directories.append((Path(entry.path), relative))
                continue
            if not stat.S_ISREG(entry_metadata.st_mode):
                raise PolicyError(
                    f"{canonical}: only regular files and directories are allowed"
                )
            if entry_metadata.st_nlink != 1:
                raise PolicyError(f"{canonical}: hard-linked files are forbidden")
            if entry_metadata.st_size > MAX_FILE_BYTES:
                raise PolicyError(f"{canonical}: file exceeds the size policy limit")
            if len(files) >= MAX_FILES:
                raise PolicyError(f"release tree exceeds the {MAX_FILES}-file policy limit")
            digest = _hash_regular_file(Path(entry.path), entry_metadata)
            files.append(
                ReleaseFile(
                    path=canonical,
                    mode=f"0{stat.S_IMODE(entry_metadata.st_mode):03o}",
                    size=entry_metadata.st_size,
                    sha256=digest,
                )
            )
        # Reverse so the lexicographically first directory is inspected first.
        pending.extend(reversed(directories))
    return tuple(sorted(files, key=lambda entry: entry.path))


def build_manifest(root: Path, identity: BuildIdentity) -> ReleaseManifest:
    return ReleaseManifest(identity=identity, files=scan_release_tree(root))


def verify_release_tree(root: Path, manifest: ReleaseManifest) -> None:
    actual = scan_release_tree(root)
    expected_by_path = {entry.path: entry for entry in manifest.files}
    actual_by_path = {entry.path: entry for entry in actual}
    missing = sorted(set(expected_by_path) - set(actual_by_path))
    unexpected = sorted(set(actual_by_path) - set(expected_by_path))
    if missing or unexpected:
        raise PolicyError(
            f"release tree paths differ: missing={missing}, unexpected={unexpected}"
        )
    for path in sorted(expected_by_path):
        expected = expected_by_path[path]
        observed = actual_by_path[path]
        for field in ("mode", "size", "sha256"):
            if getattr(expected, field) != getattr(observed, field):
                raise PolicyError(
                    f"{path}: {field} mismatch: expected {getattr(expected, field)!r}, "
                    f"observed {getattr(observed, field)!r}"
                )
