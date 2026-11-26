"""Build the offline QBIND unit 1 report and its terminal-journey control."""
from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path

from native_talk_binding_evidence import canonical_definitions, verify_talk_binding
from quest_completion_import import (
    EVENTS, EVENTS_SHA, EVENT_CORRECTIONS, EVENT_CORRECTIONS_SHA, NPC, NPC_SHA,
    OUTPUT, PLAN, RECEIPT, SOURCE_EVENTS, SOURCE_EVENTS_SHA, encode, pinned, sha,
)

_ROOT = Path(__file__).resolve().parents[3]
_OUTPUT = Path(__file__).relative_to(_ROOT).parent / "samples/server-completion/qbind-admission.json"
_CONTROL = "oteryn:quest.pilgrimage_of_ashes"
_BANK = "oteryn:quest.authored.bank_robbery_mini_world_change"
_BINDINGS = "content/quests/authored-bindings"
_PACKETS = {
    EVENTS: EVENTS_SHA,
    EVENT_CORRECTIONS: EVENT_CORRECTIONS_SHA,
    SOURCE_EVENTS: SOURCE_EVENTS_SHA,
    NPC: NPC_SHA,
}


def _encode(report):
    return (json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()


def _require(condition, reason):
    if not condition:
        raise ValueError(reason)


def _digest(value):
    return sha(encode(value).removesuffix(b"\n"))


def _load_control(root, record, candidate):
    """Project one exact, already-registered linear NPC journey for test injection."""
    quests = [
        (path.relative_to(root).as_posix(), quest)
        for path in sorted((root / _BINDINGS).glob("*.json"))
        for quest in json.loads(path.read_bytes())["quests"]
        if quest["quest"] == record["quest"]
    ]
    _require(len(quests) == 1, "missing or ambiguous registered quest owner")
    document, owner = quests[0]
    definition = canonical_definitions(root)[owner["quest"]]
    chosen = definition["oteryn_recipe"]
    _require(chosen["profile"] == "chosen_source_completion_v1"
             and chosen["payload"]["identity"] == definition["identity"]
             and chosen["chosen_data_complete"] is True, "stale or incomplete chosen recipe")
    recipe = chosen["payload"]["recipe"]
    _require(owner["recipe_sha256"] == _digest(recipe), "registered recipe digest mismatch")
    req = recipe["requirements"]
    _require(req["prerequisites"] == []
             and owner["admission"] == {"minimum_level": req["min_level"], "premium": req["premium"]}
             and recipe["repeat"]["kind"] == "once" and not owner.get("occurrence"),
             "unsupported actor gate or occurrence")

    projections = [row for row in candidate["quests"] if row["quest"] == owner["quest"]]
    _require(len(projections) == 1, "missing or ambiguous chosen transition owner")
    native, planned, stages = owner["stages"], record["stages"], recipe["stages"]
    transitions = projections[0]["transitions"]
    _require(len(native) == len(planned) == len(stages) == len(transitions) and len(stages) >= 2,
             "stale stage coverage")
    canonical_ref = transitions[-1]["source"]["canonical_ref"]
    _require(canonical_ref["definition_sha256"] == _digest(definition)
             and sha((root / canonical_ref["path"]).read_bytes()) == canonical_ref["packet_sha256"],
             "canonical definition digest mismatch")
    _require(len(owner["tracks"]) == 1, "ambiguous terminal progress track")
    track = owner["tracks"][0]
    _require(track == {"key": track["key"], "minimum": 0, "initial": 0, "maximum": len(stages)},
             "nonlinear terminal progress track")

    for i, (stage, row, selected, transition) in enumerate(zip(native, planned, stages, transitions)):
        terminal = i + 1 == len(stages)
        key = selected["key"]
        event = row["event_identity_associations"]
        source = transition["source"]
        _require(stage["key"] == row["stage_key"] == event["stage_key"] == key
                 and row["quest_transition_key"] == transition["key"]
                 and source["chosen_stage"] == selected
                 and source["canonical_ref"] == canonical_ref
                 and transition["completes"] is terminal
                 and selected["kind"] == event["kind"] == ("complete" if terminal else "talk")
                 and selected["count"] == event["count"] == 1
                 and selected["next"] == ([] if terminal else [stages[i + 1]["key"]]),
                 "stale chosen transition, definition or terminal graph")
        trigger = stage["trigger"]
        _require(trigger["kind"] == "npc_dialogue" and not trigger.get("alternatives"),
                 "ambiguous terminal journey NPC selector")
        callback = trigger["callback"]
        _require(stage["completes"] is terminal and callback["completes"] is terminal
                 and callback["quest"] == owner["quest"]
                 and callback["storage"] == [
                     {"track": track["key"], "minimum": i, "maximum": i, "set": i + 1}
                 ], "stale terminal predecessor or callback owner")
        if terminal:
            seams = event["consumer_seams"]
            _require(len(seams) == 1 and seams[0]["incoming_stage_keys"] == [stages[i - 1]["key"]]
                     and seams[0]["next_stage_keys"] == [], "ambiguous terminal predecessor")
        else:
            ref = row["native_dispatch_binding"]
            _require(row["runtime_enabled"] is True and ref is not None
                     and ref["document"] == document, "missing qualified TALK reference")
            _require(verify_talk_binding(root, definition, key, ref) == [callback],
                     "stale TALK callback")
            _require(callback["give"] is None, "unexpected pre-terminal item reward")
        _require(not callback["take"] and callback["outfit"] is None
                 and callback["achievement"] is None, "unqualified dialogue effect")

    intents = record["reward_identity_associations"]
    chosen_rewards = recipe["reward_intents"]
    grant = native[-1]["trigger"]["callback"]["give"]
    _require(len(intents) == len(chosen_rewards) == 1 and grant is not None,
             "missing or ambiguous terminal reward")
    intent, reward = intents[0], chosen_rewards[0]
    _require(intent["kind"] == reward["kind"] == "item"
             and intent["count"] == reward["count"] == grant["count"]
             and intent["mapping"]["refs"] == [reward["item"]]
             and reward["item"]["key"] == grant["item"]
             and not grant["contents"], "terminal reward identity or quantity mismatch")
    return {"quest": owner["quest"], "registry_document": document,
            "terminal_stage": native[-1]["key"], "scope": "existing_owner_test_control",
            "bindings": {"schema": 1, "quests": [owner]}}


def _admit(root, plan, candidate, input_error=None):
    records = plan["records"]
    owners = Counter(record["quest"] for record in records)
    admitted, held = [], []
    for record in sorted(records, key=lambda row: row["quest"]):
        quest = record["quest"]
        if quest == _BANK:
            reason = "Bank Robbery occurrence unavailable"
        elif input_error:
            reason = input_error
        elif owners[quest] != 1:
            reason = "ambiguous completion plan quest owner"
        elif len({s["stage_key"] for s in record["stages"]}) != len(record["stages"]):
            reason = "ambiguous stage identity"
        elif quest == _CONTROL:
            try:
                admitted.append(_load_control(root, record, candidate))
            except (ValueError, OSError, KeyError, TypeError) as error:
                reason = str(error)
            else:
                continue
        else:
            missing = [s["stage_key"] for s in record["stages"]
                       if not s["runtime_enabled"] or s["native_dispatch_binding"] is None]
            access = [r["mapping"]["target"] for r in record["reward_identity_associations"]
                      if r["kind"] == "access" and not r["runtime_admitted"]]
            reason = "no qualified terminal release binding"
            if missing:
                reason += "; native stage evidence missing for " + ", ".join(missing)
            if record["native_reward_delivery_binding"] is None:
                reason += "; reward delivery binding missing"
            if access:
                reason += "; exact access targets and gates missing for " + ", ".join(access)
        held.append({"quest": quest, "reason": reason})
    return {
        "schema": "OTERYN_QBIND_TERMINAL_ADMISSION/v1",
        "counts": {"admitted": len(admitted), "held": len(held), "production": 0},
        "admitted": admitted,
        "held": held,
        "production_bindings": {"schema": 1, "quests": []},
    }


def _report(root):
    try:
        receipt = json.loads((root / RECEIPT).read_bytes())
        plan_raw = (root / PLAN).read_bytes()
        _require(receipt["binding_plan"]["path"] == PLAN
                 and sha(plan_raw) == receipt["binding_plan"]["sha256"],
                 f"digest mismatch: {PLAN}")
        plan = json.loads(plan_raw)
        records = plan["records"]
        _require(isinstance(records, list) and records
                 and all(isinstance(row["quest"], str) for row in records), "invalid completion plan roster")
    except (ValueError, OSError, KeyError, TypeError) as error:
        # The two pinned event packets retain the cohort identities even if
        # the derived plan is missing owners or no longer parses.
        roster = {"records": [
            {"quest": row["quest_ref"]["key"]}
            for path in (EVENTS, SOURCE_EVENTS)
            for row in pinned(root, path, _PACKETS[path])["records"]
        ]}
        return _admit(root, roster, None, input_error=str(error))
    try:
        candidate_raw = (root / OUTPUT).read_bytes()
        _require(receipt["path"] == OUTPUT and sha(candidate_raw) == receipt["sha256"],
                 f"digest mismatch: {OUTPUT}")
        _require(plan["schema"] == "OTERYN_QUEST_COMPLETION_BINDING_PLAN/v1"
                 and {pin["path"]: pin["sha256"] for pin in plan["input_packets"]} == _PACKETS
                 and len(plan["input_packets"]) == len(_PACKETS), "stale completion packet pins")
        # The importer pins the exact authoritative native owners independently
        # of historical event associations. Terminal identity comes from that pin.
        control = next(row for row in plan["records"] if row["quest"] == _CONTROL)
        path = control["stages"][0]["native_dispatch_binding"]["document"]
        owner_pins = [ref["sha256"] for ref in receipt["input_provenance"] if ref["path"] == path]
        _require(owner_pins == [sha((root / path).read_bytes())], f"digest mismatch: {path}")
        for path, digest in _PACKETS.items():
            _require(sha((root / path).read_bytes()) == digest, f"digest mismatch: {path}")
        candidate = json.loads(candidate_raw)
    except (ValueError, OSError, KeyError, TypeError, IndexError, StopIteration) as error:
        return _admit(root, plan, None, input_error=str(error) or "missing control quest owner")
    return _admit(root, plan, candidate)


def _main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    target = _ROOT / _OUTPUT
    raw = _encode(_report(_ROOT))
    if args.check:
        if target.read_bytes() != raw:
            parser.exit(1, f"Stale report: {_OUTPUT}\n")
    else:
        target.write_bytes(raw)


if __name__ == "__main__":
    _main()
