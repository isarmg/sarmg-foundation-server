from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "sarmg_workflow_policy",
    ROOT / "scripts" / "check-workflow-supply-chain.py",
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class WorkflowPolicyTests(unittest.TestCase):
    def test_only_exact_release_path_gets_tag_scoped_write_permission(self) -> None:
        text = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
        MODULE.validate_workflow(".github/workflows/release.yml", text)
        with self.assertRaisesRegex(MODULE.PolicyError, "only contents: read"):
            MODULE.validate_workflow(".github/workflows/not-release.yml", text)

    def test_release_write_permission_requires_only_v_star_tag_push(self) -> None:
        text = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
        changed = text.replace('      - "v*"', '      - "*"')
        with self.assertRaisesRegex(MODULE.PolicyError, "only push tags v"):
            MODULE.validate_workflow(".github/workflows/release.yml", changed)


if __name__ == "__main__":
    unittest.main()
