"""Build a fail-closed binding work plan for Source-lowered Quest transitions.

The chosen completion binding plan intentionally covers chosen-stage recipes. A small set of
canonical Quest owners is already fully Source-lowered by QUEST-LOWER-1 and therefore has no
chosen-stage record. This packet closes that planning gap without creating runtime bindings.

It copies only existing QuestState transition keys and their retained source/request evidence.
No transition, keyword, NPC, placement or trigger identity is invented here.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

QUEST_STATE = "content/quests/missions/quest-state.json"
CHOSEN_PLAN = "content/quests/missions/completion-binding-plan.json"
OUTPUT = "content/quests/missions/source-lowered-binding-plan.json"

NPC_LANE = "NPC_QUEST_1"
TRIGGER_LANE = "QUEST_TRIGGER_1"


def read(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def completion_state(quest: dict) -> str:
    value = quest["completion"]
    return value if isinstance(value, str) else value["state"]


def classify_transition(transition: dict) -> tuple[str, str]:
    source = transition.get("source") or {}
    owner = source.get("owner")
    requested_by = transition.get("requested_by")
    if owner == "npc":
        if not isinstance(requested_by, dict) or not requested_by.get("npc"):
            raise ValueError(
                f"{transition['key']}: Source NPC transition lacks exact requested_by NPC evidence"
            )
        return NPC_LANE, "QUEST_STATE_REQUESTED_BY_EVIDENCE"
    if owner in {"action", "movement"}:
        if requested_by is not None:
            raise ValueError(
                f"{transition['key']}: Source {owner} transition unexpectedly has requested_by"
            )
        callback = source.get("callback")
        if not isinstance(callback, str) or not callback.strip():
            raise ValueError(
                f"{transition['key']}: Source {owner} transition lacks callback evidence"
            )
        return TRIGGER_LANE, "QUEST_STATE_SOURCE_CALLBACK_EVIDENCE"
    raise ValueError(f"{transition['key']}: unsupported Source transition owner {owner!r}")


def expected(root: Path) -> dict:
    state = read(root, QUEST_STATE)
    chosen = read(root, CHOSEN_PLAN)
    chosen_owners = {row["quest"] for row in chosen["records"]}

    records = []
    lane_counts: Counter[str] = Counter()
    source_owner_counts: Counter[str] = Counter()
    requested = 0
    completes = 0

    for quest in state["quests"]:
        if completion_state(quest) != "LOWERED" or quest["quest"] in chosen_owners:
            continue
        transitions = []
        for transition in quest["transitions"]:
            lane, basis = classify_transition(transition)
            source = transition.get("source") or {}
            lane_counts[lane] += 1
            source_owner_counts[source["owner"]] += 1
            requested += transition.get("requested_by") is not None
            completes += bool(transition.get("completes"))
            transitions.append(
                {
                    "quest_transition_key": transition["key"],
                    "completes": bool(transition["completes"]),
                    "consumer_lane": lane,
                    "binding_basis": basis,
                    "requested_by": transition.get("requested_by"),
                    "source": source,
                    "native_binding": None,
                    "runtime_enabled": False,
                }
            )
        if not transitions:
            raise ValueError(f"{quest['quest']}: Source-lowered owner has no transitions")
        records.append(
            {
                "quest": quest["quest"],
                "source_completion_state": "LOWERED",
                "transition_count": len(transitions),
                "native_binding_count": 0,
                "runtime_enabled": False,
                "transitions": transitions,
            }
        )

    records.sort(key=lambda row: row["quest"])
    if set(chosen_owners) & {row["quest"] for row in records}:
        raise ValueError("Source-lowered plan overlaps the chosen completion binding plan")

    result = {
        "schema": "OTERYN_QUEST_SOURCE_LOWERED_BINDING_PLAN/v1",
        "classification": "BINDING_WORK_QUEUE_NOT_RUNTIME_AUTHORITY",
        "runtime_enabled": False,
        "inputs": {
            "quest_state": QUEST_STATE,
            "chosen_binding_plan": CHOSEN_PLAN,
        },
        "summary": {
            "quests": len(records),
            "transitions": sum(row["transition_count"] for row in records),
            "completion_transitions": completes,
            "requested_by_transitions": requested,
            "native_bindings": 0,
            "consumer_lanes": dict(sorted(lane_counts.items())),
            "source_owners": dict(sorted(source_owner_counts.items())),
        },
        "limits": [
            "Every row names an existing QuestState transition; no transition key is invented.",
            "NPC requested_by evidence is a binding candidate for NPC-QUEST-1, not an executable dialogue binding.",
            "Source action/movement ownership routes work to QUEST-TRIGGER-1 but does not bind a placement or occurrence root.",
            "runtime_enabled remains false until the owning runtime child admits the exact binding.",
        ],
        "records": records,
    }
    return result


def encoded(value: dict) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    target = root / OUTPUT
    data = encoded(expected(root))
    if args.check:
        if not target.is_file() or target.read_bytes() != data:
            raise ValueError("Source-lowered binding plan drift: " + OUTPUT)
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    print(json.dumps(json.loads(data)["summary"], ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
