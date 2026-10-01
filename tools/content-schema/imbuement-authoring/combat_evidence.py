#!/usr/bin/env python3
"""Check the qualified combat evidence used by the offline authoring catalogue."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
PACKET = HERE / "samples/imbuement-combat.json"
RULE_STATUSES = {
    "vibrancy_sequence": "PRIMARY_INDEXED_WITH_COMMUNITY_CORROBORATION",
    "leech_equal_damage_aoe_scaling": "COMMUNITY_EXPLICIT",
    **{name: "PUBLIC_EVIDENCE_UNRESOLVED" for name in (
        "leech_rounding", "leech_unequal_damage_and_overkill_order",
        "vibrancy_reflection_current", "critical_healing_scope",
        "leech_equipment_composition", "protection_equipment_composition",
        "vibrancy_pvp_gate")},
}


def validate(packet):
    if packet["activation"] != "DRAFT_NOT_RUNTIME_READY":
        raise ValueError("combat evidence cannot activate runtime")
    sources = {s["id"]: s for s in packet["sources"]}
    if len(sources) != len(packet["sources"]):
        raise ValueError("duplicate combat source")
    rules = {r["id"]: r for r in packet["rules"]}
    if len(rules) != len(packet["rules"]):
        raise ValueError("duplicate combat rule")
    if set(rules) != set(RULE_STATUSES):
        raise ValueError("combat evidence must cover exactly the qualified rule set")
    for rule in rules.values():
        if rule["status"] != RULE_STATUSES[rule["id"]]:
            raise ValueError("combat confidence cannot be promoted without new qualified evidence")
        if not rule["evidence"] or any(s not in sources for s in rule["evidence"]):
            raise ValueError("combat rule has an unresolved source reference")
        if RULE_STATUSES[rule["id"]] == "PUBLIC_EVIDENCE_UNRESOLVED" and rule["value"] is not None:
            raise ValueError("an unresolved rule cannot select an invented value")
    sequence = rules["vibrancy_sequence"]["value"]
    if sequence["trigger"] != "additional_paralysis_attack_while_paralysed":
        raise ValueError("Vibrancy cannot be modelled as initial paralysis immunity")
    if sequence["remove_chance_bps_by_tier"] != [1500, 2500, 5000]:
        raise ValueError("Vibrancy recovery probabilities differ from the evidence")
    if sequence["initial_paralysis_intercepted"] is not False:
        raise ValueError("initial paralysis interception is not supported")
    if sequence["reflect_to_attacker"] is not None:
        raise ValueError("the obsolete reflection teaser cannot establish current reflection")
    if (set(sequence) != {"trigger", "initial_paralysis_intercepted",
                         "remove_chance_bps_by_tier", "recovery_sources",
                         "deflect_additional_pvp_paralysis", "reflect_to_attacker",
                         "pvp_gate_profile"}
            or sequence["recovery_sources"] != ["monster", "player"]
            or sequence["pvp_gate_profile"] != "vibrancy_pvp_gate"):
        raise ValueError("Vibrancy sequence differs from qualified source predicates")
    if sequence["deflect_additional_pvp_paralysis"] is not None or rules["vibrancy_pvp_gate"]["value"] is not None:
        raise ValueError("the unresolved PvP success-state gate cannot become unconditional protection")
    aoe = rules["leech_equal_damage_aoe_scaling"]["value"]
    if aoe["scope"] != "EQUAL_DAMAGE_PER_TARGET_EXAMPLES":
        raise ValueError("equal-hit calculator examples cannot prove the unequal-hit pipeline")
    if aoe["first_target_factor_bps"] != 10000 or aoe["additional_target_factor_bps"] != 1000:
        raise ValueError("unsupported AoE leech coefficients")
    expected_examples = [
        {"damage_per_target": 1000, "leech_share_bps": 2500,
         "hit_targets": targets, "unrounded_amount": amount}
        for targets, amount in ((1, 250), (6, 375))]
    if (set(aoe) != {"scope", "first_target_factor_bps",
                    "additional_target_factor_bps", "formula", "examples"}
            or aoe["examples"] != expected_examples
            or aoe["formula"] != "damage_per_target * leech_share * (1 + 0.1 * (hit_targets - 1))"):
        raise ValueError("AoE examples or formula differ from captured calculator evidence")
    if packet["excluded_test_server_changes"][0]["admission"] != "NOT_ADMITTED_WITHOUT_LIVE_RELEASE_EVIDENCE":
        raise ValueError("a vocation test announcement cannot be admitted as a live rule")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.parse_args()
    validate(json.loads(PACKET.read_bytes()))
    print("PASS: combat evidence, sequence semantics and source qualification")


if __name__ == "__main__":
    main()
