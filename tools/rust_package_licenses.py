"""Verify that every Foundation Rust package distributes its audited license."""

from __future__ import annotations

import subprocess
from pathlib import Path

from foundation_policy import RUST_PACKAGES


MAX_PACKAGE_LIST_BYTES = 1024 * 1024


class RustPackageLicenseError(RuntimeError):
    """A Cargo package omitted or duplicated its root license text."""


def require_root_license(package: str, listing: str) -> None:
    entries = listing.splitlines()
    if entries.count("LICENSE") != 1:
        raise RustPackageLicenseError(
            f"{package}: Cargo package must contain exactly one root LICENSE"
        )


def check_rust_package_licenses(root: Path) -> None:
    root = root.resolve(strict=True)
    for package in RUST_PACKAGES:
        completed = subprocess.run(
            [
                "cargo",
                "package",
                "--locked",
                "--allow-dirty",
                "--list",
                "-p",
                package,
            ],
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if len(completed.stdout) > MAX_PACKAGE_LIST_BYTES:
            raise RustPackageLicenseError(f"{package}: Cargo package list is too large")
        try:
            listing = completed.stdout.decode("utf-8", errors="strict")
        except UnicodeDecodeError as error:
            raise RustPackageLicenseError(
                f"{package}: Cargo package list is not UTF-8"
            ) from error
        require_root_license(package, listing)
        print(f"rust package license: passed {package}")
