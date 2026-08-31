#!/usr/bin/env python3
"""Repository-local launcher for the versioned Sarmg release-tree CLI."""

from __future__ import annotations

import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

from sarmg_release.cli import main  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(main())
