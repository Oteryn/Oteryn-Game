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

from native_talk_binding_evidence import verify_talk_binding, registered_talk_bindings

DEFINITIONS = "content/quests/definitions/index.json"
CANDIDATE = "content/quests/missions/quest-state-completion-candidate.json"
PLAN = "content/quests/missions/completion-binding-plan.json"
SOURCE_PLAN = "content/quests/missions/source-lowered-binding-plan.json"
MATRIX = "tools/content-schema/quest-authoring/samples/completion-matrix/all373.json"
OUTPUT = "tools/content-schema/quest-authoring/samples/native-binding-preflight/all352.json"


def read(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def check(condition: bool, message: str):
    if not condition:
        raise ValueError(message)


def verify_stage_runtime(root, definition, stage):
    """TALK stage activation requires the exact document consumed at server boot."""
    reference = stage.get("native_dispatch_binding")
    if stage["event_identity_associations"]["kind"] != "talk":
        check(not stage.get("runtime_enabled"), "Unsupported runtime stage kind")
        return
    if reference is None:
        check(not stage.get("runtime_enabled"), "Runtime TALK stage has no native binding")
        return
    check(stage.get("runtime_enabled") is True, "Registered TALK binding is not marked enabled")
    callbacks = verify_talk_binding(root, definition, stage["stage_key"], reference)
    document = read(root, reference["document"])
    quest = next(q for q in document["quests"] if q["quest"] == definition["identity"]["key"])
    verify_talk_state_bounds(quest, callbacks)



def verify_talk_state_bounds(quest, callbacks):
    """Mirror authored_bindings/state_operations.rs without granting write authority."""
    native_i64 = lambda value: type(value) is int and -(2 ** 63) <= value < 2 ** 63
    tracks = {track['key']: track for track in quest['tracks']}
    for track in tracks.values():
        check(all(native_i64(track.get(field)) for field in ['minimum', 'initial', 'maximum'])
              and -(2 ** 63) <= track['minimum'] <= track['initial'] <= track['maximum'] < 2 ** 63,
              'Native TALK track has invalid signed bounds')
    for callback in callbacks:
        check(all(all(native_i64(state[field]) for field in ['minimum', 'maximum', 'set'])
                  and not state.get('increment') and not state.get('at_least')
                  for state in callback['storage']), 'Native TALK has unsupported storage mutation')
        for operation in callback.get('state_ops', []):
            track = tracks[operation['track']]
            condition, effect = operation['from'], operation['effect']
            bounds = None
            if condition['kind'] == 'between':
                bounds = (condition['minimum'], condition['maximum'])
                check(all(native_i64(value) for value in bounds)
                      and track['minimum'] <= bounds[0] <= bounds[1] <= track['maximum'],
                      'Native TALK condition is outside its owned track')
            elif condition['kind'] == 'elapsed_at_least':
                check(type(condition['seconds']) is int and track['minimum'] >= 0
                      and condition['seconds'] <= 366 * 24 * 60 * 60,
                      'Native TALK elapsed condition is outside engine bounds')
            if effect['kind'] == 'add':
                delta = effect['value']
                check(native_i64(delta) and ((bounds is None and delta == 0)
                      or (bounds is not None and all(track['minimum'] <= value + delta <= track['maximum']
                                                    for value in bounds))),
                      'Native TALK mutation can overflow its owned track')
            else:
                check(effect['kind'] == 'set_now' and track['minimum'] == 0
                      and track['maximum'] == 2 ** 63 - 1,
                      'Native TALK timestamp needs the engine timestamp bounds')


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
    source_plan = read(root, SOURCE_PLAN)
    matrix = read(root, MATRIX)
    typed = {row["quest"]: row for row in candidate["quests"]}
    planned = {row["quest"]: row for row in plan["records"]}
    check(len(typed) == len(candidate["quests"]), "Duplicate typed Quest owner")
    check(len(planned) == len(plan["records"]), "Duplicate binding plan owner")
    source_planned = {row["quest"]: row for row in source_plan["records"]}
    check(len(source_planned) == len(source_plan["records"]), "Duplicate source-lowered plan owner")
    check(set(source_planned) <= set(typed), "Source-lowered plan owner absent from candidate")
    check(not set(source_planned) & set(planned), "Quest in both binding plans")
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
    source_lane_totals = Counter()
    records = []
    registered = registered_talk_bindings(root)
    plan_stage_keys = {(row["quest"], stage["stage_key"]) for row in plan["records"] for stage in row["stages"]}
    check(set(registered) <= plan_stage_keys, "Registered NPC binding has no stage plan")

    for key, definition in sorted(definitions.items()):
        quest = typed.get(key)
        binding = planned.get(key)
        check(not binding or quest, "Binding plan without typed state")

        per_kinds = Counter()
        per_identities = Counter()
        per_seams = Counter()
        per_stage_holds = Counter()
        per_reward_holds = Counter()
        source_lanes = Counter()
        source_transitions = 0
        source_unbound = 0
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
                if kind == "talk" and (key, stage["stage_key"]) in registered:
                    check(stage.get("native_dispatch_binding") == registered[(key, stage["stage_key"])], "Registered TALK binding is absent from activation metadata")
                verify_stage_runtime(root, definition, stage)
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
        elif key in source_planned:
            lane = "SOURCE_LOWERED_TRANSITION_CONSUMERS"
            source = source_planned[key]
            check(not source.get("runtime_enabled"), f"Runtime already enabled {key}")
            check(source["transition_count"] == len(source["transitions"]), f"Source plan count {key}")
            typed_keys = {transition["key"] for transition in quest["transitions"]}
            seen_source = set()
            for transition in source["transitions"]:
                transition_key = transition["quest_transition_key"]
                check(transition_key in typed_keys, f"Unknown source-plan transition {transition_key}")
                check(transition_key not in seen_source, f"Duplicate source-plan transition {transition_key}")
                seen_source.add(transition_key)
                source_transitions += 1
                source_lanes[transition["consumer_lane"]] += 1
                if transition.get("native_binding") is None:
                    source_unbound += 1
        elif quest is not None:
            lane = "SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN"
        else:
            check(
                definition.get("kind") == "reward_only",
                f"Non-reward Quest without typed progress candidate {key}",
            )
            lane = "REWARD_ONLY_NO_TYPED_PROGRESS_CANDIDATE"

        totals[lane] += 1
        totals["plan_stages"] += plan_stages
        totals["unbound_dispatch_stages"] += transitions_without_binding
        totals["native_dispatch_bindings"] += stage_bindings
        totals["native_reward_delivery_bindings"] += reward_bindings
        totals["target_associations"] += target_count
        totals["reward_intents"] += reward_intents
        totals["source_lowered_transitions"] += source_transitions
        totals["source_lowered_unbound_transitions"] += source_unbound
        source_lane_totals.update(source_lanes)

        records.append({
            "quest": key,
            "display_name": definition["display_name"],
            "wiki_titles": sorted(set(wiki_titles[key])),
            "lane": lane,
            "typed_completion_state": state(quest) if quest else None,
            "plan_stage_count": plan_stages,
            "unbound_dispatch_stage_count": transitions_without_binding,
            "source_lowered_transition_count": source_transitions,
            "source_lowered_unbound_transition_count": source_unbound,
            "source_lowered_consumer_lanes": counter(source_lanes),
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
        and totals["SOURCE_LOWERED_TRANSITION_CONSUMERS"] == len(source_planned)
        and totals["SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN"]
        == len(typed) - len(planned) - len(source_planned)
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
            "source_lowered_binding_plan": SOURCE_PLAN,
            "wiki_title_mapping": MATRIX,
        },
        "scope": "All canonical Quest owners; no gameplay or production runtime run",
        "summary": {
            "canonical_quests": len(definitions),
            "wiki_titles": len(matrix["records"]),
            "typed_candidate_quests": len(typed),
            "binding_plan_quests": len(planned),
            "source_lowered_plan_quests": len(source_planned),
            "lanes": {
                lane: totals[lane] for lane in (
                    "CHOSEN_STAGE_CONSUMERS",
                    "SOURCE_LOWERED_TRANSITION_CONSUMERS",
                    "SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN",
                    "REWARD_ONLY_NO_TYPED_PROGRESS_CANDIDATE",
                )
            },
            "plan_stages": totals["plan_stages"],
            "unbound_dispatch_stages": totals["unbound_dispatch_stages"],
            "source_lowered_transitions": totals["source_lowered_transitions"],
            "source_lowered_unbound_transitions": totals["source_lowered_unbound_transitions"],
            "source_lowered_consumer_lanes": counter(source_lane_totals),
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
