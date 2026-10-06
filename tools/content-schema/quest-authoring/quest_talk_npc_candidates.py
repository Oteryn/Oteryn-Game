"""Build fail-closed source-derived Quest talk -> NPC/Dialogue candidates.

Only exact canonical NPC identity aliases are considered:
- the canonical NPC key tail;
- exact source-binding external_id values already attached to that NPC.

No fuzzy/name similarity matching is allowed. A talk stage is bindable only as an
offline candidate when exactly one canonical NPC matches and that NPC has a
canonical Dialogue ref. Dialogue branch selection and runtime admission remain held.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

PLAN = "content/quests/missions/completion-binding-plan.json"
NPC_INDEX = "content/npcs/definitions/index.json"
OUTPUT = "tools/content-schema/quest-authoring/samples/server-completion/talk-npc-candidates.json"
SCHEMA = "OTERYN_SOURCE_TALK_NPC_CANDIDATES/v1"


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


def npc_index(root: Path):
    index = read(root, NPC_INDEX)
    aliases = defaultdict(lambda: defaultdict(set))
    metadata = {}
    inputs = [{"path": NPC_INDEX, "sha256": digest((root / NPC_INDEX).read_bytes())}]
    for relative in index["shards"]:
        raw = (root / relative).read_bytes()
        inputs.append({"path": relative, "sha256": digest(raw)})
        payload = json.loads(raw)
        for row in payload["records"]:
            declaration = row["declaration"]
            identity = declaration["identity"]
            key = identity["key"]
            metadata[key] = {
                "npc_ref": {
                    "family": "NPC",
                    "key": key,
                    "revision": identity["revision"],
                },
                "dialogue_ref": declaration.get("dialogue"),
            }
            tail = key.removeprefix("oteryn:npc.")
            aliases[norm(tail)][key].add("CANONICAL_KEY_TAIL")
            for binding in row.get("source_bindings") or []:
                if binding.get("disposition") != "EXACT":
                    continue
                external_id = binding.get("external_id")
                if isinstance(external_id, str) and external_id.strip():
                    aliases[norm(external_id)][key].add(
                        "EXACT_SOURCE_BINDING_EXTERNAL_ID"
                    )
    if len(metadata) != index["record_count"]:
        raise ValueError("NPC index count differs from shards")
    return aliases, metadata, inputs


def classify(raw_targets, aliases, metadata):
    matched = {}
    for target in raw_targets:
        for key, bases in aliases.get(norm(target), {}).items():
            row = matched.setdefault(
                key,
                {
                    "npc_ref": metadata[key]["npc_ref"],
                    "dialogue_ref": metadata[key]["dialogue_ref"],
                    "evidence": [],
                },
            )
            for basis in sorted(bases):
                row["evidence"].append({"target": target, "basis": basis})
    matches = [matched[key] for key in sorted(matched)]
    if len(matches) == 1:
        if matches[0]["dialogue_ref"] is not None:
            status = "EXACT_NPC_WITH_DIALOGUE"
            holds = [
                "DIALOGUE_BRANCH_SELECTION_PENDING",
                "NATIVE_NPC_QUEST_DISPATCH_BINDING_PENDING",
            ]
        else:
            status = "EXACT_NPC_NO_DIALOGUE"
            holds = ["CANONICAL_NPC_HAS_NO_DIALOGUE"]
    elif len(matches) > 1:
        status = "AMBIGUOUS_MULTIPLE_NPCS"
        holds = ["MULTIPLE_CANONICAL_NPCS_MATCH_STAGE_TARGETS"]
    else:
        status = "NO_EXACT_NPC"
        holds = ["NO_EXACT_CANONICAL_NPC_MATCH"]
    return status, matches, holds


def expected(root: Path):
    plan_raw = (root / PLAN).read_bytes()
    plan = json.loads(plan_raw)
    aliases, metadata, npc_inputs = npc_index(root)
    records = []

    for quest in plan["records"]:
        for stage in quest["stages"]:
            event = stage["event_identity_associations"]
            if event["kind"] != "talk":
                continue
            # Authored talk rows already have their own NPC packet/candidate lane.
            if stage.get("NPC_dialogue_candidates") is not None:
                continue
            raw_targets = target_texts(event)
            status, matches, holds = classify(
                raw_targets, aliases, metadata
            )
            candidate = matches[0] if status == "EXACT_NPC_WITH_DIALOGUE" else None
            records.append(
                {
                    "quest": quest["quest"],
                    "stage_key": stage["stage_key"],
                    "quest_transition_key": stage["quest_transition_key"],
                    "raw_targets": raw_targets,
                    "status": status,
                    "matches": matches,
                    "candidate": candidate,
                    "holds": holds,
                    "selected_NPC_branch": None,
                    "native_dispatch_binding": None,
                    "runtime_admitted": False,
                }
            )

    records.sort(key=lambda row: (row["quest"], row["stage_key"]))
    counts = Counter(row["status"] for row in records)
    if len(records) != 247:
        raise ValueError("source-derived talk stage population changed")
    expected_counts = Counter(
        {
            "EXACT_NPC_WITH_DIALOGUE": 171,
            "EXACT_NPC_NO_DIALOGUE": 23,
            "AMBIGUOUS_MULTIPLE_NPCS": 36,
            "NO_EXACT_NPC": 17,
        }
    )
    if counts != expected_counts:
        raise ValueError(
            f"source-derived talk classification changed: {dict(counts)}"
        )
    if any(row["runtime_admitted"] for row in records):
        raise ValueError("runtime admission forbidden")
    if any(
        row["selected_NPC_branch"] is not None
        or row["native_dispatch_binding"] is not None
        for row in records
    ):
        raise ValueError("native talk binding promotion forbidden")

    inputs = [
        {"path": PLAN, "sha256": digest(plan_raw)},
        *npc_inputs,
        {
            "path": "tools/content-schema/quest-authoring/quest_talk_npc_candidates.py",
            "sha256": digest(
                (
                    root
                    / "tools/content-schema/quest-authoring/quest_talk_npc_candidates.py"
                ).read_bytes()
            ),
        },
    ]
    return {
        "schema": SCHEMA,
        "classification": "EXACT_CANONICAL_NPC_DIALOGUE_CANDIDATES_FAIL_CLOSED",
        "runtime_admitted": False,
        "summary": {
            "talk_stages": len(records),
            "statuses": dict(sorted(counts.items())),
            "dialogue_candidates": counts["EXACT_NPC_WITH_DIALOGUE"],
            "native_dispatch_bindings": 0,
            "selected_dialogue_branches": 0,
        },
        "matching_contract": {
            "allowed_aliases": [
                "canonical NPC key tail",
                "EXACT NPC source_binding external_id",
            ],
            "fuzzy_matching": False,
            "display_name_guessing": False,
            "multiple_NPC_selection": False,
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
            raise ValueError("talk NPC candidate packet drift")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
    print(json.dumps(json.loads(raw)["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
