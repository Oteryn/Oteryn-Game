#!/usr/bin/env python3
"""Reproduce a bounded Creature source candidate without promoting source truth."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "docs/agents/evidence/OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings.json"
DEFAULT_OUTPUT = ROOT / "docs/agents/evidence/OTV2-20260926-creature-rat-source-pilot.json"
DEFINITION_OUTPUT = ROOT / "content/creatures/definitions/rat.json"
INDEX_OUTPUT = ROOT / "content/creatures/definitions/index.json"
SOURCE_REF = "monster:0692"
SOURCE_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
SOURCE_PRODUCT_DIGEST = "f69a941a4953c9f3b2532e90ab6589c7a89126b1e8123b0ff2e8d88777f7853f"
RAT_BLOB = "81b9057c51cd36b665b83e644cbd68166ef9961f"
# Explicit Game-owned authoring choice; never computed from the source name or ID.
RAT_IDENTITY = {"family": "Creature", "key": "oteryn:creature.rat", "revision": "definition-r1"}


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
        "proposed_game_owned_identity": RAT_IDENTITY,
        "identity_admission": "DRAFT_NOT_ADMITTED",
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
            "source_batch_native_binding": "UNRESOLVED",
            "loot_item_references": "PRESERVE_SOURCE_RESOLUTION_STATES",
            "runtime_switch": False,
            "world_placements_changed": False,
        },
    }


def definition(pilot: dict) -> dict:
    observation = pilot["definition_observation"]
    source = pilot["source"]
    return {
        "schema": "OTERYN_CREATURE_AUTHORING_DEFINITION/v1",
        "identity": RAT_IDENTITY,
        "authoring_state": "DRAFT_SOURCE_CANDIDATE",
        "display_name": observation["display_name"],
        "source_binding": {
            "source_repository": source["repository"],
            "source_revision": source["revision"],
            "source_identity": source["source_identity"],
            "source_path": source["file"]["path"],
            "source_blob": source["file"]["blob"],
            "evidence": source["evidence"],
        },
        "gameplay_semantics": {"state": "UNKNOWN"},
        "source_candidate_profile_id": observation["candidate_profile_id"],
        "runtime_authority": False,
    }


def index() -> dict:
    return {
        "schema": "OTERYN_GAME_TREE_DIRECTORY/v1",
        "path": "content/creatures/definitions/",
        "kind": "static_content",
        "owner": "Creature",
        "repository": "Oteryn/Oteryn-Game",
        "population_state": "DRAFT_SOURCE_CANDIDATE",
        "contract": "docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md",
        "definitions": ["content/creatures/definitions/rat.json"],
        "record_count": 1,
        "notes": "One source-bound authoring draft; gameplay truth and runtime promotion remain unqualified.",
    }


def encode(value: dict) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare with the committed pilot")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    pilot = candidate()
    outputs = {args.output: encode(pilot), DEFINITION_OUTPUT: encode(definition(pilot)), INDEX_OUTPUT: encode(index())}
    for path, payload in outputs.items():
        if args.check:
            if path.read_bytes() != payload:
                raise SystemExit(f"creature pilot differs from pinned evidence: {path}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
    if args.check:
        print("creature source pilot and draft definition match pinned evidence")


if __name__ == "__main__":
    main()
