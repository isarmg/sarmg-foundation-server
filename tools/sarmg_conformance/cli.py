"""Command-line entry point for Sarmg conformance policy."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Callable

from .policy import (
    ConformanceError,
    generate_consumer_matrix,
    verify_consumer_registry,
    verify_foundation,
    verify_manifest,
    verify_release,
    verify_schema,
    verify_source,
    verify_web,
)


Check = Callable[[Path, Path], dict[str, Any]]


def _write_matrix(root: Path, *, check: bool) -> dict[str, Any]:
    generated = generate_consumer_matrix(root)
    path = root / "consumers" / "consumer-matrix.json"
    encoded = json.dumps(generated, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not path.is_file() or json.loads(path.read_text(encoding="utf-8")) != generated:
            raise ConformanceError("consumer-matrix.json is stale; run generate-consumer-matrix")
    else:
        path.write_text(encoded, encoding="utf-8")
    return generated


def _product_check(arguments: argparse.Namespace, check: Check) -> dict[str, Any]:
    return check(arguments.product_root.resolve(strict=True), arguments.foundation_root.resolve(strict=True))


def _report(arguments: argparse.Namespace) -> tuple[dict[str, Any], bool]:
    checks: dict[str, Any] = {}
    ok = True
    for name, check in (
        ("manifest", verify_manifest),
        ("source", verify_source),
        ("schema", verify_schema),
        ("web", verify_web),
        ("release", verify_release),
    ):
        try:
            checks[name] = {"ok": True, "result": _product_check(arguments, check)}
        except (OSError, ConformanceError) as error:
            ok = False
            checks[name] = {"ok": False, "error": str(error)}
    return {"ok": ok, "checks": checks}, ok


def main(argv: list[str] | None = None) -> int:
    default_foundation = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(prog="sarmg-conformance")
    parser.add_argument("--foundation-root", type=Path, default=default_foundation)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name in ("verify-manifest", "verify-source", "verify-schema", "verify-web", "verify-release", "report"):
        child = subparsers.add_parser(name)
        child.add_argument("--product-root", type=Path, default=Path.cwd())
        if name == "report":
            child.add_argument("--json", action="store_true", dest="as_json")
    subparsers.add_parser("verify-foundation")
    subparsers.add_parser("verify-consumers")
    generate = subparsers.add_parser("generate-consumer-matrix")
    generate.add_argument("--check", action="store_true")
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "verify-foundation":
            result = verify_foundation(arguments.foundation_root.resolve(strict=True))
        elif arguments.command == "verify-consumers":
            result = verify_consumer_registry(arguments.foundation_root.resolve(strict=True))
        elif arguments.command == "generate-consumer-matrix":
            result = _write_matrix(arguments.foundation_root.resolve(strict=True), check=arguments.check)
        elif arguments.command == "report":
            result, ok = _report(arguments)
            print(json.dumps(result, ensure_ascii=False, indent=2))
            return 0 if ok else 1
        else:
            checks: dict[str, Check] = {
                "verify-manifest": verify_manifest,
                "verify-source": verify_source,
                "verify-schema": verify_schema,
                "verify-web": verify_web,
                "verify-release": verify_release,
            }
            result = _product_check(arguments, checks[arguments.command])
        print(json.dumps(result, ensure_ascii=False, sort_keys=True))
        return 0
    except (OSError, ConformanceError) as error:
        print(f"sarmg-conformance: FAILED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
