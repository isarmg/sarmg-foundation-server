"""Command-line entry point for the narrow release-tree policy."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .release import (
    BuildIdentity,
    PolicyError,
    build_manifest,
    parse_manifest,
    render_manifest,
    verify_release_tree,
)


CLI_VERSION = "0.3.1"


def _identity(arguments: argparse.Namespace) -> BuildIdentity:
    return BuildIdentity(
        product=arguments.product,
        version=arguments.release_version,
        source_revision=arguments.source_revision,
        target=arguments.target,
        state_contract_sha256=arguments.state_contract_sha256,
    )


def _add_identity_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--product", required=True)
    parser.add_argument("--release-version", required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--state-contract-sha256", required=True)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        prog="sarmg-release",
        description="Create identities and verify exact, link-free release trees.",
    )
    result.add_argument("--version", action="version", version=f"%(prog)s {CLI_VERSION}")
    commands = result.add_subparsers(dest="command", required=True)

    identity = commands.add_parser("identity", help="print one validated build identity")
    _add_identity_arguments(identity)

    create = commands.add_parser("create", help="create a canonical release-tree manifest")
    create.add_argument("root", type=Path)
    _add_identity_arguments(create)

    verify = commands.add_parser("verify", help="verify a tree against a strict manifest")
    verify.add_argument("root", type=Path)
    verify.add_argument("manifest", type=Path)
    return result


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        if arguments.command == "identity":
            print(json.dumps(_identity(arguments).to_dict(), sort_keys=True))
        elif arguments.command == "create":
            sys.stdout.buffer.write(render_manifest(build_manifest(arguments.root, _identity(arguments))))
        elif arguments.command == "verify":
            manifest = parse_manifest(arguments.manifest.read_bytes(), str(arguments.manifest))
            verify_release_tree(arguments.root, manifest)
            print(f"release tree: passed {arguments.root}")
        else:  # pragma: no cover - argparse requires one known command
            raise AssertionError(arguments.command)
        return 0
    except (OSError, UnicodeError, PolicyError) as error:
        print(f"release tree: FAILED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
