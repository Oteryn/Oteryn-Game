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
    "life_leech_reported_equal_hit_ceiling": "DATED_COMMUNITY_GAMEPLAY_REPORTED_EXAMPLES",
    "mana_leech_wheel_equipment_example": "DATED_COMMUNITY_EXPLICIT_WHEEL_MANA_EXAMPLE",
    "leech_elemental_parry_wound_reported_exclusions": "DATED_COMMUNITY_REPORTED_CHARM_TEST",
    "ranged_elemental_ammo_reported_cases": "DATED_COMMUNITY_REPORTED_RANGED_AMMO_FIELD_STUDY",
    "life_leech_damage_prey_exclusion": "ARCHIVED_PRE_TARGET_COMMUNITY_EXPLICIT_LIFE_ONLY",
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
LIFE_PREY_VALUE = {
    "scope": "LIFE_LEECH_REFERENCE_ONLY", "damage_prey_bonus_included": False,
    "all_damage_modifier_order": None, "overkill_basis": None,
    "rounding": None, "current_target_continuity": None,
}
PAIR_VALUE = {
    "scope": "ONLY_TWO_POWERFUL_VOID_OR_TWO_POWERFUL_VAMPIRISM_INSTANCES",
    "observed_pairs": [
        {"family": "Void", "tier": "powerful", "instance_share_bps": [800, 800], "combined_share_bps": 1600},
        {"family": "Vampirism", "tier": "powerful", "instance_share_bps": [2500, 2500], "combined_share_bps": 5000},
    ],
    "other_equipment_composition": None,
}
LIFE_CASES = [(1, 438, 110), (2, 387, 54), (3, 383, 39), (4, 438, 36)]
LIFE_VALUE = {
    "scope": "ONLY_REPORTED_2020_AVALANCHE_EQUAL_HIT_CASES",
    "interpretation": "REPORTED_EXAMPLES_FIT_PER_TARGET_CEILING",
    "setup": {"family": "Vampirism", "tier": "powerful", "instance_count": 1,
              "leech_share_bps": 2500, "equipment_slot": "armor", "attack": "Avalanche",
              "wand_equipped": False, "targets": "Dragon Lords", "same_gear": True},
    "reported_examples": [
        {"hit_targets": n, "damage_per_target": damage,
         "healed_each_target": healed, "total_healed": n * healed}
        for n, damage, healed in LIFE_CASES],
    "universal_rounding": None, "unequal_target_rounding": None,
    "current_target_continuity": None, "cap_bps": None, "overkill_behavior": None,
}
WHEEL_VALUE = {
    "scope": "ONLY_POWERFUL_VOID_PLUS_WHEEL_50_BPS_EXAMPLE",
    "family": "Void", "tier": "powerful", "equipment_share_bps": 800,
    "wheel_share_bps": 50, "combined_share_bps": 850, "chance_bps": 10000,
    "other_combinations": None, "life_leech_composition": None,
    "cap_bps": None, "current_target_continuity": None,
}
CHARM_VALUE = {
    "scope": "ONLY_REPORTED_ELEMENTAL_CHARM_PARRY_WOUND_TEST",
    "reported_to_trigger_imbuement_leech": {
        "elemental_charms": False, "parry": False, "wound": False},
    "other_charms": None, "current_target_continuity": None, "gameplay_calendar_date": None,
}
RANGED_AMMO_VALUE = {
    "scope": "ONLY_REPORTED_BASIC_FROST_ELVISH_BOW_FLASH_AND_SHIVER_CASES",
    "interpretation": "HISTORICAL_REPORTED_APPROXIMATE_SPLITS",
    "bow": "Elvish Bow", "imbuement": "Basic Frost", "conversion_bps": 1000,
    "reported_cases": [
        {"ammo": "Flash Arrow", "ammo_element": "energy",
         "base_physical_attack": 14, "base_elemental_attack": 14,
         "reported_damage_split_bps": {"physical": 5000, "energy": 5000, "ice": 0}},
        {"ammo": "Shiver Arrow", "ammo_element": "ice",
         "base_physical_attack": 14, "base_elemental_attack": 14,
         "reported_damage_split_bps": {"physical": 4500, "ice": 5500}},
    ],
    "reported_arrows_total": 200, "arrows_per_ammo": None,
    "universal_conversion_order": None, "critical_order": None, "armor_order": None,
    "integer_rounding": None, "damage_cap": None,
    "universal_damage_conservation": None, "current_target_continuity": None,
    "source_conflict_profile": "ranged_native_ammunition_reference_conflict",
}
RANGED_REFERENCE_CONFLICT = {
    "id": "ranged_native_ammunition_reference_conflict",
    "rule": "ranged_elemental_ammo_reported_cases",
    "sources": ["tibiaqa_basic_frost_elemental_ammo_2021", "fandom_imbuing_full"],
    "status": "PUBLIC_REFERENCE_CONFLICT_UNRESOLVED",
    "current_reference_claim": "NO_EFFECT_WITH_AREA_OR_NATIVE_ELEMENTAL_AMMUNITION",
    "corroborated_historical_case": "Flash Arrow: no ice component",
    "conflicting_historical_case": "Shiver Arrow: reported45/55 physical/ice versus reference blanket native-ammo exclusion",
    "resolution": "KEEP_HISTORICAL_EXAMPLES_ONLY_NO_CURRENT_ALGORITHM_SELECTED",
    "current_native_ammo_rule": None, "current_timer_rule": None,
}
NEW_RULES = {
    "life_leech_damage_prey_exclusion": (
        LIFE_PREY_VALUE, "ARCHIVE_2026_01_13_BEFORE_TARGET_NOT_EXACT_TARGET_OBSERVATION",
        {"fandom_life_archive_2026"}),
    "vibrancy_reflection_removed_at_release": (
        {"reflect_to_attacker": False, "scope": "LIVE_RELEASE_2018_12_03_ONLY"},
        "HISTORICAL_RELEASE_CONFIRMED_CURRENT_CONTINUITY_UNQUALIFIED",
        {"official_vibrancy_release_4828"}),
    "mana_leech_current_reference_formula": (
        MANA_VALUE, "POST_TARGET_COMMUNITY_REVISION_NOT_TARGET_CERTIFIED", {"fandom_formulae_1205374"}),
    "leech_two_powerful_equipment_pairs": (
        PAIR_VALUE, "DATED_PRE_TARGET_COMMUNITY_NOT_EXACT_TARGET_OBSERVATION",
        {"tibiaqa_two_mana_2018", "tibiaqa_two_leech_pairs_2021"}),
    "life_leech_reported_equal_hit_ceiling": (
        LIFE_VALUE, "HISTORICAL_2020_REPORT_NOT_EXACT_TARGET_OBSERVATION",
        {"tibiaqa_life_equal_hit_2020", "tibiaqa_life_answer_revisions_14497"}),
    "mana_leech_wheel_equipment_example": (
        WHEEL_VALUE, "DATED_2022_COMMUNITY_EXAMPLE_NOT_EXACT_TARGET_OBSERVATION",
        {"tibiaqa_wheel_void_example_2022"}),
    "leech_elemental_parry_wound_reported_exclusions": (
        CHARM_VALUE, "HISTORICAL_2020_2021_REPORT_NOT_EXACT_TARGET_OBSERVATION",
        {"tibiaqa_charm_leech_test_2020"}),
    "ranged_elemental_ammo_reported_cases": (
        RANGED_AMMO_VALUE, "HISTORICAL_2021_REPORTED_AMMO_CASES_NOT_EXACT_TARGET_OBSERVATION",
        {"tibiaqa_basic_frost_elemental_ammo_2021", "fandom_imbuing_full"}),
}
SOURCE_PROFILES = {
    'fandom_vibrancy_archive_2025': (
        'https://web.archive.org/web/20250518214227/https://tibia.fandom.com/wiki/Vibrancy',
        'a6145d0d62d71867d6793e92efa51898c87cf2459f5e2185b923d186e854d5ba',
        "FULL_PUBLIC_ARCHIVED_COMMUNITY_HTML", "COMMUNITY_GLOBAL_REFERENCE"),
    'fandom_life_archive_2026': (
        'https://web.archive.org/web/20260113015644/https://tibia.fandom.com/wiki/Life_Leech',
        '87ad436a20cbba484a44d65b39a7518afe65b2cabf1df8c12d99b47556f1020f',
        "FULL_PUBLIC_ARCHIVED_COMMUNITY_HTML", "COMMUNITY_GLOBAL_REFERENCE"),
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
    "tibiaqa_life_equal_hit_2020": (
        "https://www.tibiaqa.com/14483/how-does-life-leech-work-when-we-attack-more-than-one-monster?show=14483#q14483",
        "ddcefe4dd18fc91445a0102d86608bd33796a2d38c587bb32d4c631bd6e2aabb",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
    "tibiaqa_life_answer_revisions_14497": (
        "https://www.tibiaqa.com/?qa=revisions/14497",
        "46d78939b4bedfd0f83a8b5e0ddc0ed4600859676bf1c5615c08158c87789510",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
    "tibiaqa_wheel_void_example_2022": (
        "https://www.tibiaqa.com/31830/does-the-life-leech-and-mana-leech-stack-in-the-wheel-of-destiny?show=31830#q31830",
        "65356afbab0814650fb043ab5883d16fe91f8eb2b3b1d66459bd91c3d04e2e91",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
    "tibiaqa_charm_leech_test_2020": (
        "https://www.tibiaqa.com/16927/does-damage-dealt-when-charm-activated-triggers-your-leech-imbuements?show=16927#q16927",
        "e996cd92175cc41528a6d95ef9b0dcd94bf30eb3432e2f7525e8909a5b8947d1",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
    "tibiaqa_basic_frost_elemental_ammo_2021": (
        "https://www.tibiaqa.com/23754/what-happens-you-attempt-use-elemental-arrows-with-elemental-imbued-bow",
        "8d18d563a7a40539080de54bedc1cd9c4f05ea4fabca85dec2bb3556caf686dd",
        "FULL_PUBLIC_HTML_OR_SCRIPT", "COMMUNITY_GLOBAL_REFERENCE"),
    "fandom_imbuing_full": (
        "https://tibia.fandom.com/wiki/Imbuing",
        "e07d4be8d37844eeaf4f078ad7de1242047e18466cc999a6686c22c05657c82b",
        "FULL_REVISIONED_COMMUNITY_BROWSER_TEXT", "COMMUNITY_GLOBAL_REFERENCE"),
}
# Pin full normalized source records, not only their self-reported digests.
# These offline checks protect captured bytes, claims, revision links and dates;
# they do not fetch the source or turn a community report into Global telemetry.
BOUNDED_SOURCE_RECORD_SHA256 = {
    'fandom_vibrancy_archive_2025': 'caa29e1f6e6c1c5b1711ed1cbc4c36da56f922d4e68ad70780ca51087f42dac0',
    'fandom_life_archive_2026': 'ed34805d032fd136f0ea834370aee980f9497c854d22eaf28ced21639247f978',
    "tibiaqa_life_equal_hit_2020": "690881ab2e0897cab0949e43505083d398325dac6da0589dc735dc6045b6f238",
    "tibiaqa_life_answer_revisions_14497": "4693b05ac634043ecd67fb0bf428c40bfc2ea18596c82fa055e8fe8042e51d16",
    "tibiaqa_wheel_void_example_2022": "7c8871b4dc4435324b9848b7887ff0345506e03d882f3bbc595067fece6a5a0c",
    "tibiaqa_charm_leech_test_2020": "68182e8300f29d3c7a8f30156ca41a552f2e83276ec00661bb25b7d8b907be00",
    "tibiaqa_basic_frost_elemental_ammo_2021": "70863489377139ec9c4b04be527b31c01ce4db8ff38bbe04c1774719e77b459e",
    "fandom_imbuing_full": "24a02f33245590d7f7abce0f86ac211d42b9586af90ff53cb9a59b23d2523ead",
}
BOUNDED_LINKS = {
    "leech_rounding": ["mana_leech_current_reference_formula", "life_leech_reported_equal_hit_ceiling"],
    "leech_equipment_composition": ["leech_two_powerful_equipment_pairs", "mana_leech_wheel_equipment_example"],
    "leech_unequal_damage_and_overkill_order": [
        "mana_leech_current_reference_formula", "life_leech_reported_equal_hit_ceiling",
        "leech_elemental_parry_wound_reported_exclusions", "life_leech_damage_prey_exclusion"],
}
RULE_TIME_STATUSES = {
    **{name: "CURRENT_SOURCE_NO_EXACT_TARGET_CERTIFICATION" for name in RULE_STATUSES},
    "vibrancy_sequence": "PRE_TARGET_INTRODUCTION_WITH_CURRENT_CORROBORATION_NOT_FULL_SNAPSHOT",
    "vibrancy_pvp_gate": "PRE_TARGET_ARCHIVED_QUALIFIED_WORDING_NO_EXACT_GATE_OR_TARGET_CERTIFICATION",
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


def life_reported_example(damage_per_target, hit_targets):
    """Interpret only the four recorded historical cases; not a Life runtime."""
    if (type(damage_per_target) is not int or type(hit_targets) is not int
            or (hit_targets, damage_per_target) not in {(n, d) for n, d, _ in LIFE_CASES}):
        raise ValueError("Life interpretation is limited to the recorded equal-hit cases")
    numerator = damage_per_target * 2500 * (hit_targets + 9)
    denominator = 10000 * 10 * hit_targets
    ceil_each = -(-numerator // denominator)
    return {
        "ceil_each": ceil_each, "total_if_ceil_each": ceil_each * hit_targets,
        "nearest_each": (2 * numerator + denominator) // (2 * denominator),
        "ceil_aggregated_once": -(-(numerator * hit_targets) // denominator),
    }


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
    for source_id in (set().union(*(rule[2] for rule in NEW_RULES.values()))
                      | {"fandom_vibrancy_archive_2025"}):
        source = sources[source_id]
        if tuple(source[key] for key in ("url", "sha256", "access_status", "role")) != SOURCE_PROFILES[source_id]:
            raise ValueError("bounded combat sources must retain their exact identity and qualification")
        selected = source["selected_claims"]
        sha = hashlib.sha256(json.dumps(selected, sort_keys=True, ensure_ascii=False,
                                        separators=(",", ":")).encode()).hexdigest()
        if source["selected_claims_sha256"] != sha:
            raise ValueError("selected combat claims digest disagrees")
    for source_id, expected_sha in BOUNDED_SOURCE_RECORD_SHA256.items():
        source = sources[source_id]
        digest = hashlib.sha256(json.dumps(source, sort_keys=True, ensure_ascii=False,
                                          separators=(",", ":")).encode()).hexdigest()
        if digest != expected_sha:
            raise ValueError("reported combat provenance cannot be rewritten or self-certified")
    for name, profiles in BOUNDED_LINKS.items():
        if rules[name].get("bounded_profiles") != profiles:
            raise ValueError("unresolved rules must preserve qualified bounded profile links")
    ranged_conflicts = [c for c in packet["source_conflicts"]
                       if c.get("id") == RANGED_REFERENCE_CONFLICT["id"]]
    if (len(ranged_conflicts) != 1 or json.dumps(ranged_conflicts[0], sort_keys=True)
            != json.dumps(RANGED_REFERENCE_CONFLICT, sort_keys=True)):
        raise ValueError("historical Shiver report must retain unresolved current-reference conflict")
    for example in rules["life_leech_reported_equal_hit_ceiling"]["value"]["reported_examples"]:
        derived = life_reported_example(example["damage_per_target"], example["hit_targets"])
        if (derived["ceil_each"] != example["healed_each_target"]
                or derived["total_if_ceil_each"] != example["total_healed"]):
            raise ValueError("Life interpretation differs from the recorded healing logs")
    wheel = rules["mana_leech_wheel_equipment_example"]["value"]
    if wheel["combined_share_bps"] != wheel["equipment_share_bps"] + wheel["wheel_share_bps"]:
        raise ValueError("bounded Wheel example must preserve its additive calculation")
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
    for name in ("vibrancy_sequence", "vibrancy_pvp_gate"):
        if "fandom_vibrancy_archive_2025" not in rules[name]["evidence"]:
            raise ValueError("Vibrancy profiles must retain the complete pre-target conditional reference")
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
