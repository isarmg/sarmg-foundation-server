#!/usr/bin/env python3
"""CLI for checking the license text in each publishable Rust package."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from foundation_policy import CURRENT_VERSION  # noqa: E402
from rust_package_licenses import (  # noqa: E402
    RustPackageLicenseError,
    check_rust_package_licenses,
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="check-rust-package-licenses.py")
    parser.add_argument("--version", action="version", version=f"%(prog)s {CURRENT_VERSION}")
    parser.add_argument("--root", type=Path, default=ROOT)
    arguments = parser.parse_args(argv)
    try:
        check_rust_package_licenses(arguments.root)
        return 0
    except (OSError, subprocess.SubprocessError, RustPackageLicenseError) as error:
        print(f"rust package license: FAILED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
