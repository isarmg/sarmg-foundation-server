from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

from foundation_policy import (  # noqa: E402
    FoundationPolicyError,
    read_audited_root_license,
    require_license_copy,
)
from rust_package_licenses import (  # noqa: E402
    RustPackageLicenseError,
    require_root_license,
)


class RustPackageLicenseTests(unittest.TestCase):
    def test_exact_root_license_is_accepted(self) -> None:
        require_root_license("example", "Cargo.toml\nLICENSE\nsrc/lib.rs\n")

    def test_missing_root_license_is_rejected(self) -> None:
        with self.assertRaises(RustPackageLicenseError):
            require_root_license("example", "Cargo.toml\nsrc/lib.rs\n")

    def test_nested_or_duplicate_license_is_rejected(self) -> None:
        for listing in (
            "Cargo.toml\nlicenses/LICENSE\nsrc/lib.rs\n",
            "Cargo.toml\nLICENSE\nLICENSE\nsrc/lib.rs\n",
        ):
            with self.subTest(listing=listing):
                with self.assertRaises(RustPackageLicenseError):
                    require_root_license("example", listing)

    def test_audited_license_copy_requires_exact_regular_unlinked_bytes(self) -> None:
        root_license = TOOLS.parent / "LICENSE"
        expected = read_audited_root_license(root_license)
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            valid = temporary / "LICENSE"
            valid.write_bytes(expected)
            require_license_copy(valid, expected)

            wrong = temporary / "WRONG"
            wrong.write_text("not Apache-2.0\n", encoding="utf-8")
            with self.assertRaises(FoundationPolicyError):
                require_license_copy(wrong, expected)

            linked = temporary / "LINKED"
            os.link(valid, linked)
            with self.assertRaises(FoundationPolicyError):
                require_license_copy(linked, expected)

            symbolic = temporary / "SYMBOLIC"
            symbolic.symlink_to(root_license)
            with self.assertRaises(FoundationPolicyError):
                require_license_copy(symbolic, expected)

    def test_root_license_digest_is_fixed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            changed = Path(directory) / "LICENSE"
            changed.write_text("Apache-2.0 label is not its license text\n", encoding="utf-8")
            with self.assertRaises(FoundationPolicyError):
                read_audited_root_license(changed)


if __name__ == "__main__":
    unittest.main()
