"""Offline Quest native-binding preflight for every canonical Quest owner.

This reports missing consumer work, not runtime admission or playability. It joins
only committed canonical definitions, completion candidate, selected binding plan
and the pinned all-373 wiki-title matrix; no donor or network access is required.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path

DEFINITIONS = "content/quests/definitions/index.json"
CANDIDATE = "content/quests/missions/quest-state-completion-candidate.json"
PLAN = "content/quests/missions/completion-binding-plan.json"
MATRIX = "tools/content-schema/quest-authoring/samples/completion-matrix/all373.json"
OUTPUT = "tools/content-schema/quest-authoring/samples/native-binding-preflight/all352.json"


def read(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def check(condition: bool, message: str):
    if not condition:
        raise ValueError(message)


def counter(value: Counter):
    return dict(sorted(value.items()))


def state(quest):
    completion = quest["completion"]
    return completion if isinstance(completion, str) else completion["state"]


def collect(root: Path):
    index = read(root, DEFINITIONS)
    definitions = {}
    for path in index["shards"]:
        for row in read(root, path)["records"]:
            definition = row["definition"]
            key = definition["identity"]["key"]
            check(key not in definitions, f"Duplicate canonical Quest {key}")
            definitions[key] = definition
    check(len(definitions) == index["record_count"], "Quest index count mismatch")

    candidate = read(root, CANDIDATE)
    plan = read(root, PLAN)
    matrix = read(root, MATRIX)
    typed = {row["quest"]: row for row in candidate["quests"]}
    planned = {row["quest"]: row for row in plan["records"]}
    check(len(typed) == len(candidate["quests"]), "Duplicate typed Quest owner")
    check(len(planned) == len(plan["records"]), "Duplicate binding plan owner")
    check(set(typed) <= set(definitions), "Unknown typed Quest owner")
    check(set(planned) <= set(typed), "Binding plan owner absent from candidate")
    check(len(matrix["records"]) == matrix["summary"]["wiki_titles"], "Wiki title count mismatch")

    wiki_titles = defaultdict(list)
    for row in matrix["records"]:
        check(row["canonical"], "Unmapped wiki Quest")
        for canonical in row["canonical"]:
            key = canonical["key"]
            check(key in definitions, f"Unknown wiki Quest owner {key}")
            wiki_titles[key].append(row["wiki_title"])

    totals = Counter()
    kinds = Counter()
    identity_status = Counter()
    seams = Counter()
    stage_holds = Counter()
    reward_holds = Counter()
    records = []

    for key, definition in sorted(definitions.items()):
        quest = typed.get(key)
        binding = planned.get(key)
        check(not binding or quest, "Binding plan without typed state")

        per_kinds = Counter()
        per_identities = Counter()
        per_seams = Counter()
        per_stage_holds = Counter()
        per_reward_holds = Counter()
        plan_stages = 0
        transitions_without_binding = 0
        stage_bindings = 0
        reward_bindings = 0
        target_count = 0
        reward_intents = 0

        if binding is not None:
            lane = "CHOSEN_STAGE_CONSUMERS"
            transitions = {transition["key"]: transition for transition in quest["transitions"]}
            check(len(transitions) == len(quest["transitions"]), f"Duplicate transition {key}")
            plan_stages = len(binding["stages"])
            check(plan_stages > 0, f"Empty binding plan {key}")
            check(not binding.get("runtime_enabled"), f"Runtime already enabled {key}")

            for stage in binding["stages"]:
                transition_key = stage["quest_transition_key"]
                check(transition_key in transitions, f"Unknown plan transition {transition_key}")
                transition = transitions[transition_key]
                check(transition["quest"] == key, f"Foreign plan transition {transition_key}")
                check(
                    transition["source"]["chosen_stage"]["key"] == stage["stage_key"],
                    f"Stage/transition mismatch {transition_key}",
                )
                intent = stage["event_identity_associations"]
                kind = intent["kind"]
                per_kinds[kind] += 1
                kinds[kind] += 1
                check(not stage.get("runtime_enabled"), f"Unexpected runtime enabled {key}")
                if stage.get("native_dispatch_binding") is None:
                    transitions_without_binding += 1
                else:
                    stage_bindings += 1
                for association in intent.get("targets") or []:
                    status = association.get("status", "UNSPECIFIED")
                    per_identities[status] += 1
                    identity_status[status] += 1
                    target_count += 1
                for seam in intent.get("consumer_seams") or []:
                    status = seam.get("status", "UNSPECIFIED")
                    per_seams[status] += 1
                    seams[status] += 1
                for hold in intent.get("unresolved") or []:
                    per_stage_holds[str(hold)] += 1
                    stage_holds[str(hold)] += 1
            reward_bindings = int(binding.get("native_reward_delivery_binding") is not None)
            for reward in binding.get("reward_identity_associations") or []:
                reward_intents += 1
                for hold in reward.get("unresolved") or []:
                    per_reward_holds[str(hold)] += 1
                    reward_holds[str(hold)] += 1
        elif quest is not None:
            lane = "SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN"
        else:
            lane = "REWARD_ONLY_NO_TYPED_PROGRESS_CANDIDATE"

        totals[lane] += 1
        totals["plan_stages"] += plan_stages
        totals["unbound_dispatch_stages"] += transitions_without_binding
        totals["native_dispatch_bindings"] += stage_bindings
        totals["native_reward_delivery_bindings"] += reward_bindings
        totals["target_associations"] += target_count
        totals["reward_intents"] += reward_intents

        records.append({
            "quest": key,
            "display_name": definition["display_name"],
            "wiki_titles": sorted(set(wiki_titles[key])),
            "lane": lane,
            "typed_completion_state": state(quest) if quest else None,
            "plan_stage_count": plan_stages,
            "unbound_dispatch_stage_count": transitions_without_binding,
            "native_dispatch_binding_count": stage_bindings,
            "native_reward_delivery_binding_count": reward_bindings,
            "event_kinds": counter(per_kinds),
            "target_association_statuses": counter(per_identities),
            "consumer_seam_statuses": counter(per_seams),
            "stage_hold_codes": counter(per_stage_holds),
            "reward_intents": reward_intents,
            "reward_hold_codes": counter(per_reward_holds),
            "playable_verification": "NOT_ASSESSED",
        })

    check(len(records) == index["record_count"], "Incomplete native preflight")
    check(
        totals["CHOSEN_STAGE_CONSUMERS"] == len(planned)
        and totals["SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN"] == len(typed) - len(planned)
        and totals["REWARD_ONLY_NO_TYPED_PROGRESS_CANDIDATE"] == len(definitions) - len(typed),
        "Partition does not cover every canonical Quest",
    )
    return {
        "schema": "OTERYN_QUEST_NATIVE_BINDING_PREFLIGHT/v1",
        "classification": "DERIVED_EVIDENCE_NOT_RUNTIME_AUTHORITY",
        "inputs": {
            "canonical_definitions": DEFINITIONS,
            "typed_completion_candidate": CANDIDATE,
            "selected_binding_plan": PLAN,
            "wiki_title_mapping": MATRIX,
        },
        "scope": "All canonical Quest owners; no gameplay or production runtime run",
        "summary": {
            "canonical_quests": len(definitions),
            "wiki_titles": len(matrix["records"]),
            "typed_candidate_quests": len(typed),
            "binding_plan_quests": len(planned),
            "lanes": {
                lane: totals[lane] for lane in (
                    "CHOSEN_STAGE_CONSUMERS",
                    "SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN",
                    "REWARD_ONLY_NO_TYPED_PROGRESS_CANDIDATE",
                )
            },
            "plan_stages": totals["plan_stages"],
            "unbound_dispatch_stages": totals["unbound_dispatch_stages"],
            "native_dispatch_bindings": totals["native_dispatch_bindings"],
            "native_reward_delivery_bindings": totals["native_reward_delivery_bindings"],
            "target_associations": totals["target_associations"],
            "reward_intents": totals["reward_intents"],
            "event_kinds": counter(kinds),
            "target_association_statuses": counter(identity_status),
            "consumer_seam_statuses": counter(seams),
            "stage_hold_codes": counter(stage_holds),
            "reward_hold_codes": counter(reward_holds),
            "playable_verified": 0,
            "playability_claim": "NOT_ASSESSED_BY_THIS_REPORT",
        },
        "records": records,
    }


def serialize(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    p.add_argument("--check", action="store_true")
    args = p.parse_args()
    root = args.root.resolve()
    expected = serialize(collect(root))
    target = root / OUTPUT
    if args.check:
        check(target.exists() and target.read_bytes() == expected, "Native binding preflight drift")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(expected)
    summary = json.loads(expected)["summary"]
    print(json.dumps(summary, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
