#!/usr/bin/env python3
"""Reproduce a bounded Creature source candidate without promoting source truth."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "docs/agents/evidence/OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings.json"
DEFAULT_OUTPUT = ROOT / "docs/agents/evidence/OTV2-20260926-creature-rat-source-pilot.json"
SOURCE_REF = "monster:0692"
SOURCE_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
SOURCE_PRODUCT_DIGEST = "f69a941a4953c9f3b2532e90ab6589c7a89126b1e8123b0ff2e8d88777f7853f"
RAT_BLOB = "81b9057c51cd36b665b83e644cbd68166ef9961f"


def unique(rows: list[dict], field: str, value: str) -> dict:
    matches = [row for row in rows if row.get(field) == value]
    if len(matches) != 1:
        raise ValueError(f"expected one {field}={value}, got {len(matches)}")
    return matches[0]


def candidate() -> dict:
    source = json.loads(SOURCE.read_text(encoding="utf-8"))
    snapshot = source["source_snapshot"]
    if (snapshot["repository"], snapshot["revision"]) != ("blakinio/Otheryn", SOURCE_REVISION):
        raise ValueError("pinned source revision changed")
    if source["product_digest_sha256"] != SOURCE_PRODUCT_DIGEST:
        raise ValueError("pinned evidence product changed")
    definitions = source["monster_definitions"]
    definition = unique(definitions["records"], "source_file_ref", SOURCE_REF)
    if definition["native_mapping"]["disposition"] != "UNRESOLVED":
        raise ValueError("native mapping changed; reassess admission")
    profile = unique(definitions["candidate_profiles"], "profile_id", definition["candidate_profile_id"])
    source_file = unique(snapshot["files"], "source_file_ref", SOURCE_REF)
    if source_file["role"] != "MONSTER_DEFINITION_LUA":
        raise ValueError("source file role changed")
    if source_file["blob"] != RAT_BLOB:
        raise ValueError("pinned Rat Lua blob changed")
    return {
        "schema": "OTERYN_CREATURE_SOURCE_PLACEMENT_PILOT/v1",
        "status": "SOURCE_CANDIDATE_ONLY_NO_NATIVE_PROMOTION",
        "target_family": "Creature",
        "intended_directory": "content/creatures/definitions/",
        "canonical_identity": None,
        "source": {
            "repository": snapshot["repository"],
            "revision": snapshot["revision"],
            "source_identity": definition["source_identity"],
            "file": source_file,
            "evidence": str(SOURCE.relative_to(ROOT)),
            "product_digest_sha256": source["product_digest_sha256"],
        },
        "definition_observation": definition,
        "candidate_profile": profile,
        "constraints": {
            "native_binding": "UNRESOLVED",
            "loot_item_references": "PRESERVE_SOURCE_RESOLUTION_STATES",
            "runtime_switch": False,
            "world_placements_changed": False,
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare with the committed pilot")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    payload = (json.dumps(candidate(), ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()
    if args.check:
        if args.output.read_bytes() != payload:
            raise SystemExit("creature source pilot differs from pinned evidence")
        print("creature source pilot matches pinned evidence")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(payload)


if __name__ == "__main__":
    main()
