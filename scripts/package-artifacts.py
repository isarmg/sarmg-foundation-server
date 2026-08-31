#!/usr/bin/env python3
"""CLI for strict package manifests, clean builds, and tarball smoke tests."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

from sarmg_package_artifacts import (  # noqa: E402
    CLI_VERSION,
    PackagePolicyError,
    check_builds,
    clean_dist,
    discover_packages,
    package_smoke,
    package_release,
)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        prog="package-artifacts.py",
        description="Validate the current package manifests and reproducible package contents.",
    )
    result.add_argument("--version", action="version", version=f"%(prog)s {CLI_VERSION}")
    result.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="repository root (defaults to the launcher's repository)",
    )
    result.add_argument(
        "command",
        choices=("check", "clean", "smoke", "release"),
        help="check existing dist trees, clean them, or rebuild/pack/install-smoke all packages",
    )
    result.add_argument(
        "--output",
        type=Path,
        help="empty output directory required by the release command",
    )
    return result


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        packages = discover_packages(arguments.root)
        if arguments.command == "check":
            check_builds(packages)
        elif arguments.command == "clean":
            clean_dist(packages)
        elif arguments.command == "smoke":
            package_smoke(arguments.root.resolve(strict=True), packages)
        else:
            if arguments.output is None:
                raise PackagePolicyError("release requires --output")
            package_release(
                arguments.root.resolve(strict=True),
                packages,
                arguments.output,
            )
        return 0
    except (OSError, UnicodeError, subprocess.SubprocessError, PackagePolicyError) as error:
        print(f"package artifacts: FAILED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
