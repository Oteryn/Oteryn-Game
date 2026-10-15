#!/usr/bin/env python3
"""Validate Achievement catalogue records (OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1 §2).

Usage: python validate_achievements.py RECORDS.json [RECORDS.json ...] [--chest-claims CLAIMS.json]
Each file holds one record, a list of records or a catalogue shard ({"family": "Achievement", "records": [...]}); all files together form one catalogue, so keys must be
unique across them. The key is checked for format only: it is allocated once (allocate_key) and kept. With --chest-claims, every
placement achievement of those RewardClaim documents must bind to a catalogue key (contract §2.2, §3.3). Prints one JSON report;
the exit code is 1 when a record is invalid or a chest ref is unbound. Coverage is scoped to these supplied
inputs, not world admission or runtime earnability. A missing binding is UNEARNABLE_PENDING (§4 of
ACHIEVEMENT0); it does not invalidate an otherwise valid catalogue. Transition, encounter and threshold
inputs are not admitted by this tool yet. Earning families cannot be inferred from catalogue text.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

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


def allocate_key(name: str) -> str:
    """The key of a new record (contract §2.1); later revisions keep it even when the name changes."""
    return KEY_PREFIX + slug(name)


def record_errors(record: dict) -> list[str]:
    errors = [
        f"schema: {e.json_path}: {e.message}"
        for e in jsonschema.Draft202012Validator(SCHEMA).iter_errors(record)
    ]
    if errors:
        return errors
    if record.get("retired", False):
        if record["points"] != 0:
            errors.append("a retired achievement has 0 points")
    elif record["points"] not in GRADE_POINTS[record["grade"]]:
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


# Contract §2: source-derived candidate refs bind to catalogue keys by slug; a slug that differs
# binds only by an explicit owner decision (§2.2). A ref that binds to no key is listed, never guessed.
SOURCE_NAMESPACES = ("canary", "crystal", "crystalserver")
EXPLICIT_BINDINGS = {"the_professors_nut": "the_professor_s_nut"}


def bind_ref(ref: str, keys: set[str]) -> str | None:
    """The catalogue key an `oteryn:` key or a `<source>:achievement/<slug>` ref binds to, or None."""
    namespace, separator, ref_slug = ref.partition(":achievement/")
    if not separator or not ref_slug:
        return None
    if namespace == "oteryn":
        key = ref
    elif namespace in SOURCE_NAMESPACES:
        key = KEY_PREFIX + EXPLICIT_BINDINGS.get(ref_slug, ref_slug)
    else:
        return None
    return key if key in keys else None


def chest_refs(claims_doc: dict) -> list[str]:
    """Every `placements[].achievement` ref of a RewardClaim document (quest authoring format §4)."""
    return sorted(
        {
            placement["achievement"]["key"]
            for claim in claims_doc["claims"]
            for placement in claim["placements"]
            if placement.get("achievement")
        }
    )


def unbound_refs(refs: list[str], records: list[dict]) -> list[str]:
    """Refs no catalogue record binds (contract §3.3: content validation before runtime)."""
    keys = {
        record["identity"].get("key")
        for record in records
        if isinstance(record, dict) and isinstance(record.get("identity"), dict)
    }
    return [ref for ref in refs if bind_ref(ref, keys) is None]


def coverage_report(records: list[dict], claim_refs: list[str]) -> dict:
    """ACHIEVEMENT0 §4: missing granters in the supplied, supported content inputs.

    `claim_refs` comes only from RewardClaim placements (the existing --chest-claims
    input). A bound key proves a content reference, not a runnable/admitted world
    or a durable grant. Retired keys still bind but can never earn (§3.5).

    §2's earning families are listed explicitly. The catalogue has no earning
    family field, so selecting one from a name, description or provenance would
    invent a mapping. Every pending reason therefore remains unclassified until
    the owning content supplies it. Unsupported granters cannot count as covered.
    """
    if validate(records)["invalid"]:
        return {"status": "INVALID_CATALOGUE"}
    keys = {record["identity"]["key"] for record in records}
    bound = {key for ref in claim_refs if (key := bind_ref(ref, keys)) is not None}
    return {
        "scope": "SUPPLIED_INPUTS_ONLY_NOT_RUNTIME_EARNABILITY",
        "supported_granters": ["reward-claim"],
        "unsupported_granters": [
            "quest-transition",
            "encounter-binding",
            "counter-threshold",
        ],
        "reason_families": [
            "quest_or_quest_step",
            "boss_or_encounter",
            "counter",
            "one_time_action",
        ],
        "bound_keys": sorted(bound),
        "retired_keys": sorted(
            record["identity"]["key"]
            for record in records
            if record.get("retired", False)
        ),
        "unearnable_pending": [
            {
                "key": key,
                "status": "UNEARNABLE_PENDING",
                "reason": "NO_ADMITTED_GRANTER; EARNING_FAMILY_UNCLASSIFIED",
            }
            for key in sorted(keys - bound)
        ],
    }


def load(paths: list[Path]) -> list[dict]:
    records = []
    for path in paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data, dict) and data.get("family") == "Achievement":
            data = data["records"]
        records += data if isinstance(data, list) else [data]
    return records


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("records", nargs="+", type=Path)
    parser.add_argument(
        "--chest-claims",
        action="append",
        type=Path,
        default=[],
        help="a RewardClaim document whose placement achievements must bind to the catalogue",
    )
    args = parser.parse_args(argv)
    records = load(args.records)
    report = validate(records)
    refs = []
    if args.chest_claims:
        refs = sorted(
            {
                ref
                for path in args.chest_claims
                for ref in chest_refs(json.loads(path.read_text(encoding="utf-8")))
            }
        )
        report["chest_refs"] = len(refs)
        report["unbound_chest_refs"] = unbound_refs(refs, records)
    report["coverage"] = coverage_report(records, refs)
    print(json.dumps(report, indent=1, sort_keys=True))
    return 1 if report["invalid"] or report.get("unbound_chest_refs") else 0


if __name__ == "__main__":
    sys.exit(main())
