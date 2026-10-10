"""Build fail-closed source USE-trigger candidates for chosen Quest use stages.

A12 makes the CipSoft Tibia item id canonical identity in keys of the form
`oteryn:item.tibia.i<id>`. This tool uses that accepted identity rule only to
compare exact canonical Item targets with literal donor Action `id(...)`
registrations already associated with the same canonical Quest.

A matching registration is evidence of a possible source trigger. It does not
prove the stage's branch semantics, native placement, CommandRef, runtime
consumer, or Quest transition dispatch.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
from pathlib import Path

PLAN = "content/quests/missions/completion-binding-plan.json"
CROSSWALK = "tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json"
IDENTITY_DECISION = "docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md"
OUTPUT = "tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json"

CLASS_EXACT = "ONE_SAME_QUEST_LITERAL_ID_USE_TRIGGER_COVERS_ALL_TARGETS"
CLASS_NO_MATCH = "NO_SAME_QUEST_LITERAL_ID_USE_TRIGGER_MATCH"
CLASS_TARGET_INCOMPLETE = "TARGET_IDENTITY_INCOMPLETE"
CLASS_AMBIGUOUS = "MULTIPLE_SAME_QUEST_LITERAL_ID_USE_TRIGGERS_COVER_TARGETS"

ITEM_KEY = re.compile(r"^oteryn:item\.tibia\.i([1-9][0-9]*)$")
LITERAL_ID = re.compile(r"^id\(\s*([0-9]+(?:\s*,\s*[0-9]+)*)\s*\)$")


def read(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encoded(value) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def literal_registration_ids(trigger):
    ids = set()
    matched_registrations = []
    for registration in trigger.get("target_registrations") or []:
        match = LITERAL_ID.fullmatch(registration)
        if not match:
            continue
        values = [int(value.strip()) for value in match.group(1).split(",")]
        ids.update(values)
        matched_registrations.append(
            {
                "registration": registration,
                "item_ids": values,
            }
        )
    return ids, matched_registrations


def exact_item_targets(intent):
    exact = []
    incomplete = []
    for target in intent.get("targets") or []:
        if target.get("status") != "EXACT_CANONICAL_IDENTITY_ASSOCIATION":
            incomplete.append(
                {
                    "target": target.get("target"),
                    "status": target.get("status"),
                }
            )
            continue
        refs = target.get("refs") or []
        if len(refs) != 1 or refs[0].get("family") != "Item":
            incomplete.append(
                {
                    "target": target.get("target"),
                    "status": "EXACT_TARGET_WITHOUT_SINGLE_ITEM_REF",
                }
            )
            continue
        key = refs[0].get("key", "")
        match = ITEM_KEY.fullmatch(key)
        if not match:
            raise ValueError("Canonical Item key violates A12 identity form: " + key)
        exact.append(
            {
                "declared_target": target.get("target"),
                "item": dict(refs[0]),
                "tibia_item_id": int(match.group(1)),
                "identity_basis": "A12_CANONICAL_TIBIA_ITEM_ID",
                "materializable": target.get("materializable"),
                "stack_class": target.get("stack_class"),
            }
        )
    return exact, incomplete


def build(root: Path):
    root = root.resolve()
    plan = read(root, PLAN)
    crosswalk = read(root, CROSSWALK)
    by_quest = {row["quest_key"]: row for row in crosswalk["quests"]}

    records = []
    for quest in plan["records"]:
        owner = quest["quest"]
        source = by_quest.get(owner) or {}
        use_triggers = [
            trigger
            for trigger in source.get("trigger_bindings") or []
            if trigger.get("edge") == "USE"
        ]
        for stage in quest["stages"]:
            intent = stage["event_identity_associations"]
            if intent.get("kind") != "use":
                continue

            exact_targets, incomplete_targets = exact_item_targets(intent)
            target_ids = {row["tibia_item_id"] for row in exact_targets}
            candidates = []
            if not incomplete_targets and target_ids:
                for trigger in use_triggers:
                    registration_ids, literal_regs = literal_registration_ids(trigger)
                    if target_ids <= registration_ids:
                        candidates.append(
                            {
                                "source_graph": trigger["source_graph"],
                                "callback": trigger.get("callback"),
                                "edge": trigger.get("edge"),
                                "sources": trigger.get("sources") or [],
                                "target_registrations": trigger.get(
                                    "target_registrations"
                                )
                                or [],
                                "matched_literal_id_registrations": literal_regs,
                                "literal_registration_item_ids": sorted(
                                    registration_ids
                                ),
                                "registration_covers_stage_targets": True,
                                "registration_has_extra_item_ids": bool(
                                    registration_ids - target_ids
                                ),
                                "native_trigger_admission": trigger.get(
                                    "native_trigger_admission"
                                ),
                            }
                        )
            candidates.sort(key=lambda row: row["source_graph"])

            if incomplete_targets:
                classification = CLASS_TARGET_INCOMPLETE
            elif len(candidates) == 1:
                classification = CLASS_EXACT
            elif len(candidates) > 1:
                classification = CLASS_AMBIGUOUS
            else:
                classification = CLASS_NO_MATCH

            record = {
                "quest": owner,
                "stage_key": stage["stage_key"],
                "quest_transition_key": stage["quest_transition_key"],
                "chosen_occurrence_count": intent.get("count"),
                "classification": classification,
                "exact_item_targets": exact_targets,
                "incomplete_targets": incomplete_targets,
                "same_quest_literal_id_use_trigger_candidates": candidates,
                "native_dispatch_binding": None,
                "runtime_admitted": False,
                "holds": [
                    "SOURCE_ACTION_REGISTRATION_DOES_NOT_PROVE_NATIVE_PLACEMENT_OR_COMMANDREF",
                    "SOURCE_TRIGGER_MATCH_DOES_NOT_PROVE_STAGE_BRANCH_SEMANTICS",
                    "QUEST_TRIGGER_RUNTIME_NOT_ADMITTED",
                ],
            }
            if classification == CLASS_EXACT:
                candidate = candidates[0]
                record["candidate"] = {
                    "kind": "SAME_QUEST_SOURCE_USE_TRIGGER_CANDIDATE",
                    "source_graph": candidate["source_graph"],
                    "stage_branch_semantics_proven": False,
                    "native_placement_binding_proven": False,
                    "command_occurrence_binding_proven": False,
                    "native_dispatch_binding": None,
                    "runtime_admitted": False,
                }
            records.append(record)

    records.sort(key=lambda row: (row["quest"], row["stage_key"]))
    counts = collections.Counter(row["classification"] for row in records)
    expected = {
        CLASS_TARGET_INCOMPLETE: 412,
        CLASS_NO_MATCH: 42,
        CLASS_EXACT: 4,
    }
    if dict(counts) != expected:
        raise ValueError(f"USE trigger candidate population changed: {dict(counts)}")

    exact = [row for row in records if row["classification"] == CLASS_EXACT]
    expected_exact = {
        (
            "oteryn:quest.the_cursed_crystal",
            "s5",
            "canary:interaction/the_cursed_crystal/actions_medusa_oil",
        ),
        (
            "oteryn:quest.the_cursed_crystal",
            "s7",
            "canary:interaction/the_cursed_crystal/actions_medusa_oil",
        ),
        (
            "oteryn:quest.brotherhood_outfits_quest",
            "s3",
            "canary:interaction/dreamers_challenge_quest/actions_documents",
        ),
        (
            "oteryn:quest.brotherhood_outfits_quest",
            "s4",
            "canary:interaction/dreamers_challenge_quest/actions_documents",
        ),
    }
    actual_exact = {
        (row["quest"], row["stage_key"], row["candidate"]["source_graph"])
        for row in exact
    }
    if actual_exact != expected_exact:
        raise ValueError(f"Exact USE trigger candidate set changed: {actual_exact}")

    return {
        "schema": "OTERYN_QUEST_USE_TRIGGER_CANDIDATES/v1",
        "classification": "EVIDENCE_ONLY_NO_RUNTIME_ADMISSION",
        "runtime_admitted": False,
        "native_dispatch_bindings": 0,
        "identity_rule": {
            "decision": "A12_ITEM_IDENTITY_TIBIA_ID",
            "canonical_form": "oteryn:item.tibia.i<id>",
            "scope": "CipSoft Tibia Item ID only",
        },
        "inputs": [
            {"path": PLAN, "sha256": sha256(root / PLAN)},
            {"path": CROSSWALK, "sha256": sha256(root / CROSSWALK)},
            {
                "path": IDENTITY_DECISION,
                "sha256": sha256(root / IDENTITY_DECISION),
            },
        ],
        "summary": {
            "use_stages": len(records),
            "classification_counts": dict(sorted(counts.items())),
            "exact_same_quest_literal_id_trigger_candidates": len(exact),
            "runtime_admitted": False,
            "native_dispatch_bindings": 0,
        },
        "limits": [
            "A12 permits exact Tibia Item id comparison; no other donor numeric id is promoted.",
            "Only literal Action id(...) registrations are considered; aid, uid, positions and symbols are excluded.",
            "A source registration that contains the stage target ids does not prove which internal source branch owns the chosen stage.",
            "Source trigger association does not prove native LocalObject placement, CommandRef or QUEST-TRIGGER-1 dispatch.",
            "No candidate in this packet selects or invokes a Quest transition.",
        ],
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[3]
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    payload = encoded(build(root))
    output = root / OUTPUT
    if args.check:
        if not output.is_file() or output.read_bytes() != payload:
            raise ValueError("USE trigger candidate packet drift: " + OUTPUT)
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(payload)
    print(json.dumps(json.loads(payload)["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
