#!/usr/bin/env python3
"""Validate that xcss describes exactly one current release."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

from xcss_policy import (  # noqa: E402
    CURRENT_VERSION,
    XcssPolicyError,
    check_repository,
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="check-xcss.py")
    parser.add_argument("--version", action="version", version=f"%(prog)s {CURRENT_VERSION}")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    arguments = parser.parse_args(argv)
    try:
        check_repository(arguments.root)
        print(f"xcss policy: passed {arguments.root}")
        return 0
    except (OSError, XcssPolicyError) as error:
        print(f"xcss policy: FAILED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
