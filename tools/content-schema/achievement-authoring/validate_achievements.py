#!/usr/bin/env python3
"""Validate Achievement catalogue records (OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1 §2).

Usage: python validate_achievements.py RECORDS.json [RECORDS.json ...]
Each file holds one record or a list of records; all files together form one catalogue, so keys and
slugs must be unique across them. Prints one JSON report; the exit code is 1 when a record is invalid.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys

import jsonschema

ROOT = Path(__file__).resolve().parent
SCHEMA = json.loads((ROOT / "achievement.schema.json").read_text(encoding="utf-8"))
KEY_PREFIX = "oteryn:achievement/"
# Grade -> point range (TibiaWiki "Achievements"; tibia.com manual 5.6).
GRADE_POINTS = {1: range(1, 4), 2: range(4, 7), 3: range(7, 10), 4: range(10, 11)}


def slug(text: str) -> str:
    """The quest tooling's slug (quest-authoring/ots_chests.py), so candidate refs bind by slug."""
    return re.sub(
        r"_+",
        "_",
        re.sub(
            r"[^a-z0-9]+", "_", re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", text).lower()
        ),
    ).strip("_")


def record_errors(record: dict) -> list[str]:
    errors = [
        f"schema: {e.json_path}: {e.message}"
        for e in jsonschema.Draft202012Validator(SCHEMA).iter_errors(record)
    ]
    if errors:
        return errors
    expected = KEY_PREFIX + slug(record["name"])
    if record["identity"]["key"] != expected:
        errors.append(f"key must be {expected}")
    if record["points"] not in GRADE_POINTS[record["grade"]]:
        errors.append(
            f"points {record['points']} outside the grade {record['grade']} range"
        )
    return errors


def validate(records: list[dict]) -> dict:
    invalid, seen = [], {}
    for index, record in enumerate(records):
        errors = record_errors(record)
        identity = record.get("identity") if isinstance(record, dict) else None
        key = identity.get("key") if isinstance(identity, dict) else None
        if isinstance(key, str):
            if key in seen:
                errors.append(f"duplicate key, first at record {seen[key]}")
            seen.setdefault(key, index)
        if errors:
            invalid.append({"index": index, "key": key, "errors": errors})
    return {
        "records": len(records),
        "valid": len(records) - len(invalid),
        "invalid": invalid,
    }


def load(paths: list[Path]) -> list[dict]:
    records = []
    for path in paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        records += data if isinstance(data, list) else [data]
    return records


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("records", nargs="+", type=Path)
    report = validate(load(parser.parse_args(argv).records))
    print(json.dumps(report, indent=1, sort_keys=True))
    return 1 if report["invalid"] else 0


if __name__ == "__main__":
    sys.exit(main())
