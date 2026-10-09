from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "xcss_workflow_policy",
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

    def test_release_requires_native_windows_logging_job(self) -> None:
        text = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
        for old, new, reason in [
            ("needs: windows-log", "needs: []", "successful windows-log"),
            ("runs-on: windows-2025", "runs-on: windows-latest", "fixed runner"),
            ("cargo test --locked -p xcss-log --all-features", "cargo check --locked -p xcss-log --all-features", "locked native tests"),
        ]:
            with self.subTest(reason=reason), self.assertRaisesRegex(MODULE.PolicyError, reason):
                MODULE.validate_workflow(".github/workflows/release.yml", text.replace(old, new))


if __name__ == "__main__":
    unittest.main()
