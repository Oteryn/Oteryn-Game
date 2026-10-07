"""Build fail-closed Quest explore -> canonical Area candidates.

Only exact canonical Area names and canonical Area key tails are considered.
No fuzzy matching, coordinate inference, containment inference, or runtime admission
is performed. A canonical Area identity is not by itself proof of the actual
player-entry/spatial occurrence required by a Quest stage.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

PLAN = "content/quests/missions/completion-binding-plan.json"
OUTPUT = "tools/content-schema/quest-authoring/samples/server-completion/explore-area-candidates.json"
SCHEMA = "OTERYN_QUEST_EXPLORE_AREA_CANDIDATES/v1"

CITY_FILE = "content/world/areas/cities/areas-00000-00021.json"
REGION_FILE = "content/world/areas/regions/areas-00000-00442.json"
HUNTING_INDEX = "content/world/areas/hunting-places/index.json"
ISLAND_INDEX = "content/world/areas/islands/index.json"


def read(root: Path, relative: str):
    return json.loads((root / relative).read_text(encoding="utf-8"))


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def compact(value) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def norm(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "", value.casefold())


def target_texts(event):
    values = []
    for target in event.get("targets") or []:
        value = (
            target.get("target")
            or target.get("source_text")
            or target.get("value")
            or target.get("name")
        )
        if isinstance(value, str) and value.strip():
            values.append(value.strip())
    return values


def build_area_index(root: Path):
    aliases = defaultdict(lambda: defaultdict(set))
    metadata = {}
    inputs = []

    def ingest(declaration, source_file):
        identity = declaration.get("identity") or {}
        key = identity.get("key")
        if not key:
            return
        name = declaration.get("name") or declaration.get("display_name")
        metadata[key] = {
            "area_ref": {
                "family": "Area",
                "key": key,
                "revision": identity.get("revision"),
            },
            "name": name,
            "area_kind": declaration.get("kind") or declaration.get("area_kind"),
            "source_file": source_file,
        }
        tail = key.rsplit(".", 1)[-1]
        aliases[norm(tail)][key].add("CANONICAL_KEY_TAIL")
        if isinstance(name, str) and name.strip():
            aliases[norm(name)][key].add("CANONICAL_AREA_NAME")

    for relative in [CITY_FILE, REGION_FILE]:
        raw = (root / relative).read_bytes()
        inputs.append({"path": relative, "sha256": digest(raw)})
        payload = json.loads(raw)
        for declaration in payload["areas"]:
            ingest(declaration, relative)

    for index_relative in [HUNTING_INDEX, ISLAND_INDEX]:
        index_raw = (root / index_relative).read_bytes()
        inputs.append({"path": index_relative, "sha256": digest(index_raw)})
        index = json.loads(index_raw)
        for relative in index["shards"]:
            raw = (root / relative).read_bytes()
            inputs.append({"path": relative, "sha256": digest(raw)})
            payload = json.loads(raw)
            for row in payload["records"]:
                ingest(row["declaration"], relative)

    if len(metadata) != 970:
        raise ValueError(f"canonical Area population changed: {len(metadata)}")
    return aliases, metadata, inputs


def classify(raw_targets, aliases, metadata):
    target_matches = {}
    matches = {}
    for target in raw_targets:
        keys = sorted(aliases.get(norm(target), {}))
        target_matches[target] = keys
        for key in keys:
            row = matches.setdefault(
                key,
                {
                    **metadata[key],
                    "evidence": [],
                },
            )
            for basis in sorted(aliases[norm(target)][key]):
                row["evidence"].append({"target": target, "basis": basis})

    unique_keys = sorted(matches)
    unmatched_targets = [
        target for target, keys in target_matches.items() if not keys
    ]
    ambiguous_targets = [
        target for target, keys in target_matches.items() if len(keys) > 1
    ]

    if len(unique_keys) == 1:
        candidate = matches[unique_keys[0]]
        if len(raw_targets) == 1 and not unmatched_targets and not ambiguous_targets:
            status = "EXACT_SINGLE_AREA_ONLY_TARGET"
            holds = [
                "AREA_ENTRY_SEMANTICS_NOT_PROVEN",
                "SPATIAL_OCCURRENCE_BINDING_PENDING",
            ]
        else:
            status = "EXACT_SINGLE_AREA_WITH_OTHER_TARGETS"
            holds = [
                "AREA_ENTRY_SEMANTICS_NOT_PROVEN",
                "OTHER_STAGE_TARGETS_UNRESOLVED",
                "SPATIAL_OCCURRENCE_BINDING_PENDING",
            ]
    elif len(unique_keys) > 1:
        status = "AMBIGUOUS_MULTIPLE_AREAS"
        candidate = None
        holds = ["MULTIPLE_CANONICAL_AREAS_MATCH_STAGE_TARGETS"]
    elif raw_targets:
        status = "NO_EXACT_AREA"
        candidate = None
        holds = ["NO_EXACT_CANONICAL_AREA_MATCH"]
    else:
        status = "NO_TARGET"
        candidate = None
        holds = ["NO_EXPLORE_STAGE_TARGET"]

    return {
        "status": status,
        "candidate": candidate,
        "matches": [matches[key] for key in unique_keys],
        "target_matches": target_matches,
        "unmatched_targets": unmatched_targets,
        "ambiguous_targets": ambiguous_targets,
        "holds": holds,
    }


def expected(root: Path):
    plan_raw = (root / PLAN).read_bytes()
    plan = json.loads(plan_raw)
    aliases, metadata, area_inputs = build_area_index(root)
    records = []

    for quest in plan["records"]:
        for stage in quest["stages"]:
            event = stage["event_identity_associations"]
            if event["kind"] != "explore":
                continue
            raw_targets = target_texts(event)
            result = classify(raw_targets, aliases, metadata)
            records.append(
                {
                    "quest": quest["quest"],
                    "stage_key": stage["stage_key"],
                    "quest_transition_key": stage["quest_transition_key"],
                    "raw_targets": raw_targets,
                    **result,
                    "native_spatial_binding": None,
                    "runtime_admitted": False,
                }
            )

    records.sort(key=lambda row: (row["quest"], row["stage_key"]))
    counts = Counter(row["status"] for row in records)
    expected_counts = Counter(
        {
            "EXACT_SINGLE_AREA_ONLY_TARGET": 10,
            "EXACT_SINGLE_AREA_WITH_OTHER_TARGETS": 10,
            "AMBIGUOUS_MULTIPLE_AREAS": 36,
            "NO_EXACT_AREA": 192,
        }
    )
    if len(records) != 248 or counts != expected_counts:
        raise ValueError(
            f"explore Area candidate population changed: {len(records)} {dict(counts)}"
        )
    if any(row["runtime_admitted"] for row in records):
        raise ValueError("runtime admission forbidden")
    if any(row["native_spatial_binding"] is not None for row in records):
        raise ValueError("native spatial binding promotion forbidden")

    tool_relative = (
        "tools/content-schema/quest-authoring/quest_explore_area_candidates.py"
    )
    inputs = [
        {"path": PLAN, "sha256": digest(plan_raw)},
        *area_inputs,
        {
            "path": tool_relative,
            "sha256": digest((root / tool_relative).read_bytes()),
        },
    ]
    return {
        "schema": SCHEMA,
        "classification": "EXACT_CANONICAL_AREA_CANDIDATES_FAIL_CLOSED",
        "runtime_admitted": False,
        "summary": {
            "explore_stages": len(records),
            "canonical_areas": len(metadata),
            "statuses": dict(sorted(counts.items())),
            "exact_area_candidates": (
                counts["EXACT_SINGLE_AREA_ONLY_TARGET"]
                + counts["EXACT_SINGLE_AREA_WITH_OTHER_TARGETS"]
            ),
            "clean_single_target_candidates": counts[
                "EXACT_SINGLE_AREA_ONLY_TARGET"
            ],
            "native_spatial_bindings": 0,
        },
        "matching_contract": {
            "allowed_aliases": [
                "canonical Area name",
                "canonical Area key tail",
            ],
            "fuzzy_matching": False,
            "coordinate_inference": False,
            "containment_inference": False,
            "area_identity_proves_stage_occurrence": False,
        },
        "inputs": inputs,
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[3]
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = compact(expected(args.root))
    target = args.root / OUTPUT
    if args.check:
        if not target.is_file() or target.read_bytes() != raw:
            raise ValueError("explore Area candidate packet drift")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
    print(json.dumps(json.loads(raw)["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
