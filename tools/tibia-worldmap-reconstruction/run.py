#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from tibia_worldmap_reconstruction.core import (
    ValidationError,
    build_otbm_export_plan,
    compare_documents,
    merge_documents,
    validate_document,
)


def load_json(path: str) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValidationError(f"{path}: top-level JSON must be an object")
    return value


def write_json(value: dict[str, Any], path: str | None) -> None:
    encoded = json.dumps(value, indent=2, sort_keys=True) + "\n"
    if path is None:
        print(encoded, end="")
    else:
        Path(path).write_text(encoded, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Normalize and compare decoded Tibia worldmap evidence without guessing missing data.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("document")

    compare_parser = subparsers.add_parser("compare")
    compare_parser.add_argument("reconstruction")
    compare_parser.add_argument("reference")
    compare_parser.add_argument("--output")

    merge_parser = subparsers.add_parser("merge")
    merge_parser.add_argument("base")
    merge_parser.add_argument("update")
    merge_parser.add_argument("--output")

    plan_parser = subparsers.add_parser("otbm-plan")
    plan_parser.add_argument("document")
    plan_parser.add_argument("--output")

    args = parser.parse_args()
    try:
        if args.command == "validate":
            validate_document(load_json(args.document))
            print("VALID")
        elif args.command == "compare":
            write_json(compare_documents(load_json(args.reconstruction), load_json(args.reference)), args.output)
        elif args.command == "merge":
            write_json(merge_documents(load_json(args.base), load_json(args.update)), args.output)
        elif args.command == "otbm-plan":
            plan = build_otbm_export_plan(load_json(args.document))
            write_json(plan, args.output)
            return 0 if plan["ready"] else 2
    except (OSError, json.JSONDecodeError, ValidationError) as exc:
        parser.error(str(exc))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
