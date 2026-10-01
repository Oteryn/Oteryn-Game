#!/usr/bin/env python3
"""Check the qualified combat evidence used by the offline authoring catalogue."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
PACKET = HERE / "samples/imbuement-combat.json"
RULE_STATUSES = {
    "vibrancy_sequence": "PRIMARY_INDEXED_WITH_COMMUNITY_CORROBORATION",
    "leech_equal_damage_aoe_scaling": "COMMUNITY_EXPLICIT",
    "vibrancy_reflection_removed_at_release": "PRIMARY_OFFICIAL_HISTORICAL_RELEASE",
    "mana_leech_current_reference_formula": "POST_TARGET_COMMUNITY_EXPLICIT_MANA_ONLY",
    "leech_two_powerful_equipment_pairs": "DATED_COMMUNITY_EXPLICIT_BOUNDED_PAIRS",
    **{name: "PUBLIC_EVIDENCE_UNRESOLVED" for name in (
        "leech_rounding", "leech_unequal_damage_and_overkill_order",
        "vibrancy_reflection_current", "critical_healing_scope",
        "leech_equipment_composition", "protection_equipment_composition",
        "vibrancy_pvp_gate")},
}
MANA_FORMULA = "sum(ceil(Dmg_i * leech_share * (0.1 * N + 0.9) / N))"
MANA_VALUE = {
    "scope": "MANA_LEECH_ONLY", "formula": MANA_FORMULA,
    "rounding": "CEIL_EACH_TARGET_BEFORE_SUM",
    "critical_damage_included": True, "damage_prey_bonus_included": False,
    "overkill_damage_counts": True, "zero_damage_target_count": None,
    "damage_basis": "reported_damage_including_overkill",
    "examples": [
        {"damage_by_target": [101], "leech_share_bps": 800, "total_mana": 9},
        {"damage_by_target": [100, 900], "leech_share_bps": 800, "total_mana": 45},
    ],
}
PAIR_VALUE = {
    "scope": "ONLY_TWO_POWERFUL_VOID_OR_TWO_POWERFUL_VAMPIRISM_INSTANCES",
    "observed_pairs": [
        {"family": "Void", "tier": "powerful", "instance_share_bps": [800, 800], "combined_share_bps": 1600},
        {"family": "Vampirism", "tier": "powerful", "instance_share_bps": [2500, 2500], "combined_share_bps": 5000},
    ],
    "other_equipment_composition": None,
}
NEW_RULES = {
    "vibrancy_reflection_removed_at_release": (
        {"reflect_to_attacker": False, "scope": "LIVE_RELEASE_2018_12_03_ONLY"},
        "HISTORICAL_RELEASE_CONFIRMED_CURRENT_CONTINUITY_UNQUALIFIED",
        {"official_vibrancy_release_4828"}),
    "mana_leech_current_reference_formula": (
        MANA_VALUE, "POST_TARGET_COMMUNITY_REVISION_NOT_TARGET_CERTIFIED", {"fandom_formulae_1205374"}),
    "leech_two_powerful_equipment_pairs": (
        PAIR_VALUE, "DATED_PRE_TARGET_COMMUNITY_NOT_EXACT_TARGET_OBSERVATION",
        {"tibiaqa_two_mana_2018", "tibiaqa_two_leech_pairs_2021"}),
}
SOURCE_PROFILES = {
    "official_vibrancy_release_4828": (
        "https://www.tibia.com/news/?subtopic=newsarchive&id=4828",
        "65d3a5344a42fd18a319d99a49d3c6ef004e1d42ae8946cf2b5981f071c023ee",
        "FULL_PRIMARY_BROWSER_TEXT", "PRIMARY_OFFICIAL"),
    "fandom_formulae_1205374": (
        "https://tibia.fandom.com/wiki/Formulae",
        "7781dc9afe6ef4fe3eb1bdc40862e81bd42c33a1a77d30b79ed0cffe5dff7ad5",
        "FULL_REVISIONED_COMMUNITY_BROWSER_TEXT", "COMMUNITY_GLOBAL_REFERENCE"),
    "tibiaqa_two_mana_2018": (
        "https://www.tibiaqa.com/676/is-double-mana-leech-imbuement-worth-it",
        "7569a6bd815eaf6b2ddce5a9094b741117234917982de91c2e2afdba359ab331",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
    "tibiaqa_two_leech_pairs_2021": (
        "https://www.tibiaqa.com/18345/how-does-the-mana-leech-imbuement-work-for-knights",
        "5b19e5b23ac43808947ec7eee5e275a933910e1603224beacb2d42821ea55243",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
}
RULE_TIME_STATUSES = {
    **{name: "CURRENT_SOURCE_NO_EXACT_TARGET_CERTIFICATION" for name in RULE_STATUSES},
    "vibrancy_sequence": "PRE_TARGET_INTRODUCTION_WITH_CURRENT_CORROBORATION_NOT_FULL_SNAPSHOT",
    "vibrancy_pvp_gate": "CURRENT_INDEXED_SOURCE_CONFLICT_NO_EXACT_TARGET_CERTIFICATION",
    **{name: profile[1] for name, profile in NEW_RULES.items()},
}


def mana_reference_example(damages, share_bps):
    """Illustrate the captured formula; not a runtime or zero-hit policy."""
    if (not damages or any(type(d) is not int or d <= 0 for d in damages)
            or type(share_bps) is not int or not 0 < share_bps <= 10000):
        raise ValueError("reference examples require positive integer damage and share")
    count = len(damages)
    denominator = 10000 * 10 * count
    return sum(-(-(damage * share_bps * (count + 9)) // denominator) for damage in damages)


def validate(packet):
    if packet["schema"] != "OTERYN_IMBUEMENT_COMBAT_EVIDENCE/v1":
        raise ValueError("unknown combat evidence schema")
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
        if rule["target_time_status"] != RULE_TIME_STATUSES[rule["id"]]:
            raise ValueError("combat facts must retain their exact temporal qualification")
        if not rule["evidence"] or any(s not in sources for s in rule["evidence"]):
            raise ValueError("combat rule has an unresolved source reference")
        if len(rule["evidence"]) != len(set(rule["evidence"])):
            raise ValueError("duplicate combat evidence reference")
        if RULE_STATUSES[rule["id"]] == "PUBLIC_EVIDENCE_UNRESOLVED" and rule["value"] is not None:
            raise ValueError("an unresolved rule cannot select an invented value")
    for name, (value, time_status, evidence) in NEW_RULES.items():
        rule = rules[name]
        # Canonical JSON comparison distinguishes true/false from integers.
        if (json.dumps(rule["value"], sort_keys=True) != json.dumps(value, sort_keys=True)
                or rule["target_time_status"] != time_status
                or set(rule["evidence"]) != evidence):
            raise ValueError("bounded combat facts cannot become generic or target-certified rules")
    for source_id in set().union(*(rule[2] for rule in NEW_RULES.values())):
        source = sources[source_id]
        if tuple(source[key] for key in ("url", "sha256", "access_status", "role")) != SOURCE_PROFILES[source_id]:
            raise ValueError("bounded combat sources must retain their exact identity and qualification")
        selected = source["selected_claims"]
        sha = hashlib.sha256(json.dumps(selected, sort_keys=True, ensure_ascii=False,
                                        separators=(",", ":")).encode()).hexdigest()
        if source["selected_claims_sha256"] != sha:
            raise ValueError("selected combat claims digest disagrees")
    release = sources["official_vibrancy_release_4828"]
    if release["published_on"] != "2018-12-03" or release["source_stage"] != "LIVE_RELEASE":
        raise ValueError("the reflection-removal release cannot be replaced by the teaser")
    mana_source = sources["fandom_formulae_1205374"]
    if (mana_source["revision"] != 1205374
            or mana_source["revision_timestamp"] != "2026-09-07T17:28:39Z"
            or mana_source["role"] != "COMMUNITY_GLOBAL_REFERENCE"):
        raise ValueError("mana formula must retain its post-target community provenance")
    for source_id, date in (("tibiaqa_two_mana_2018", "2018-09-27"),
                            ("tibiaqa_two_leech_pairs_2021", "2021-05-27")):
        if sources[source_id]["published_on"] != date:
            raise ValueError("bounded equipment observations must retain their captured dates")
    if any(h["status"] != "OTS_HYPOTHESIS_ONLY" for h in packet["engine_combat_hypotheses"]):
        raise ValueError("engine hypotheses cannot be promoted to Global evidence")
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
