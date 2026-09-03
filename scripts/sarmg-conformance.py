#!/usr/bin/env python3
"""Stable source-tree launcher for sarmg-conformance."""

from __future__ import annotations

import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

from sarmg_conformance.cli import main  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(main())
