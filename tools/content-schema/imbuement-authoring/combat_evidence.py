#!/usr/bin/env python3
"""Check the qualified combat evidence used by the offline authoring catalogue."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import capture_bounds

HERE = Path(__file__).resolve().parent
PACKET = HERE / "samples/imbuement-combat.json"
TARGET = "global-tibia-current-2026-10-01"
RULE_STATUSES = {
    "wand_rod_strike_limitations_removed_at_release": "PRIMARY_OFFICIAL_RELEASED_BOUNDED_CHANGE",
    "critical_healing_scope": "PRIMARY_OFFICIAL_NAMED_LIVE_RELEASE",
    "native_equipment_percentage_reduction_example": "CURRENT_DATED_COMMUNITY_EXPLICIT_BOUNDED_NATIVE_EXAMPLE",
    "vibrancy_sequence": "PRIMARY_INDEXED_WITH_COMMUNITY_CORROBORATION",
    "leech_equal_damage_aoe_scaling": "COMMUNITY_EXPLICIT",
    "vibrancy_reflection_removed_at_release": "PRIMARY_OFFICIAL_HISTORICAL_RELEASE",
    "mana_leech_current_reference_formula": "CURRENT_DATED_COMMUNITY_EXPLICIT_MANA_ONLY",
    "leech_two_powerful_equipment_pairs": "DATED_COMMUNITY_EXPLICIT_BOUNDED_PAIRS",
    "life_leech_reported_equal_hit_ceiling": "DATED_COMMUNITY_GAMEPLAY_REPORTED_EXAMPLES",
    "mana_leech_wheel_equipment_example": "DATED_COMMUNITY_EXPLICIT_WHEEL_MANA_EXAMPLE",
    "leech_elemental_parry_wound_reported_exclusions": "DATED_COMMUNITY_REPORTED_CHARM_TEST",
    "ranged_elemental_ammo_reported_cases": "DATED_COMMUNITY_REPORTED_RANGED_AMMO_FIELD_STUDY",
    "life_leech_damage_prey_exclusion": "CURRENT_COMMUNITY_EXPLICIT_LIFE_PREY_ONLY",
    **{name: "PUBLIC_EVIDENCE_UNRESOLVED" for name in (
        "leech_rounding", "leech_unequal_damage_and_overkill_order",
        "vibrancy_reflection_current",
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
NATIVE_PERCENTAGE_EXAMPLE = {
    "scope": "ONLY_CURRENT_REFERENCE_ZAOAN_HELMET_AND_PROTECTION_AMULET_EXAMPLE",
    "original_damage": 200,
    "percentage_steps": [
        {"item": "Zaoan Helmet", "reduction_bps": 500, "damage_after_step": 190},
        {"item": "Protection Amulet", "reduction_bps": 600, "damage_after_step": 178},
    ],
    "reference_percentage_rounding": "FLOOR_REMAINING_DAMAGE_AFTER_EACH_ITEM",
    "total_armor": 9, "reference_armor_reduction_range": [4, 7],
    "reference_damage_after_armor_range": [171, 174],
    "reference_armor_stage": "AFTER_PERCENTAGE_REDUCTION_IN_THIS_EXAMPLE",
    "imbuement_and_native_composition": None, "wheel_order": None,
    "arbitrary_equipment_order": None, "current_global_runtime_applicability": None,
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
    "wand_rod_strike_limitations_removed_at_release": ({'scope': 'WANDS_AND_RODS_NAMED_OFFICIAL_RELEASE_CHANGE', 'numerous_strike_limitations_removed': True, 'all_items_unrestricted': False, 'specific_removed_item_allowlist': None, 'innate_critical_and_imbuement_composition': None}, "LIVE_RELEASE_2026_06_16_LINKED_PRIMARY_RELEASE_STATE", {'official_vocation_release_8849', 'official_vocation_release_8833'}),
    "critical_healing_scope": ({'scope': 'DRUID_BLESSING_OF_THE_GROVE_HEALING_SPELLS', 'critical_healing_permitted': True, 'uses_critical_hit_chance': True, 'uses_critical_extra_damage': True, 'all_vocations_unconditional': False, 'independent_critical_roll_order': None, 'native_and_imbuement_source_composition': None}, "LIVE_RELEASE_2026_06_16_LINKED_PRIMARY_RELEASE_STATE", {'official_vocation_release_8833', 'official_vocation_release_8849'}),
    "native_equipment_percentage_reduction_example": (
        NATIVE_PERCENTAGE_EXAMPLE, "CURRENT_DATED_COMMUNITY_EXAMPLE_NOT_GLOBAL_RUNTIME_OBSERVATION",
        {"fandom_formulae_1205374"}),
    "life_leech_damage_prey_exclusion": (
        LIFE_PREY_VALUE, "CURRENT_LIFE_AND_DATED_HEALTH_LEECH_PREY_REFERENCE_WITH_HISTORICAL_CORROBORATION",
        {"fandom_life_archive_2026", "fandom_formulae_1205374", "fandom_life_current_1101811"}),
    "vibrancy_reflection_removed_at_release": (
        {"reflect_to_attacker": False, "scope": "LIVE_RELEASE_2018_12_03_ONLY"},
        "HISTORICAL_RELEASE_CONFIRMED_CURRENT_CONTINUITY_UNQUALIFIED",
        {"official_vibrancy_release_4828"}),
    "mana_leech_current_reference_formula": (
        MANA_VALUE, "CURRENT_DATED_COMMUNITY_REVISION_NOT_GLOBAL_RUNTIME_OBSERVATION", {"fandom_formulae_1205374"}),
    "leech_two_powerful_equipment_pairs": (
        PAIR_VALUE, "HISTORICAL_2018_2021_COMMUNITY_NOT_CURRENT_RUNTIME_OBSERVATION",
        {"tibiaqa_two_mana_2018", "tibiaqa_two_leech_pairs_2021"}),
    "life_leech_reported_equal_hit_ceiling": (
        LIFE_VALUE, "HISTORICAL_2020_REPORT_NOT_CURRENT_RUNTIME_OBSERVATION",
        {"tibiaqa_life_equal_hit_2020", "tibiaqa_life_answer_revisions_14497"}),
    "mana_leech_wheel_equipment_example": (
        WHEEL_VALUE, "HISTORICAL_2022_COMMUNITY_EXAMPLE_NOT_CURRENT_RUNTIME_OBSERVATION",
        {"tibiaqa_wheel_void_example_2022"}),
    "leech_elemental_parry_wound_reported_exclusions": (
        CHARM_VALUE, "HISTORICAL_2020_2021_REPORT_NOT_CURRENT_RUNTIME_OBSERVATION",
        {"tibiaqa_charm_leech_test_2020"}),
    "ranged_elemental_ammo_reported_cases": (
        RANGED_AMMO_VALUE, "HISTORICAL_2021_REPORTED_AMMO_CASES_NOT_CURRENT_RUNTIME_OBSERVATION",
        {"tibiaqa_basic_frost_elemental_ammo_2021", "fandom_imbuing_full"}),
}
SOURCE_PROFILES = {
    'official_vocation_release_8833': ('https://www.tibia.com/news/?subtopic=newsarchive&id=8833#druid', 'dafbb14827689c54768baea055855edaf122e20606ad35b67c3385ea45ffa2ba', 'FULL_PRIMARY_BROWSER_TEXT', 'PRIMARY_OFFICIAL'),
    'official_vocation_release_8849': ('https://www.tibia.com/news/?subtopic=newsarchive&id=8849', '12b6053217bc635927417f1aa0381042b862699d3383ce1896d84bcebea1cb29', 'FULL_PRIMARY_BROWSER_TEXT', 'PRIMARY_OFFICIAL'),
    "fandom_vibrancy_current_1194726": (
        "https://tibia.fandom.com/wiki/Vibrancy",
        "a8a2f0587dd651848c74f4e50400a50467dc7b92aea50b8ed8f248614a504315",
        "FULL_REVISIONED_COMMUNITY_BROWSER_TEXT", "COMMUNITY_GLOBAL_REFERENCE"),
    "fandom_life_current_1101811": (
        "https://tibia.fandom.com/wiki/Life_Leech",
        "039565d317590cea0d35826a70ecf83532285efa3b48f7a17f1ebc395122e83b",
        "FULL_REVISIONED_COMMUNITY_BROWSER_TEXT", "COMMUNITY_GLOBAL_REFERENCE"),
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
    'official_vocation_release_8833': '92092db32cd7e50a11abc2360639dd8a9f52351f302183e0ea9f12bccb3a7795',
    'official_vocation_release_8849': '5b785d458d1997be72056fab20209e2e23e0888cd50fd0875da4368b5b82da04',
    "fandom_vibrancy_current_1194726": "61b5f6a74f687c6d89f1e2ccaff33098ba37026db7a4709e2e228fbb449c58a4",
    "fandom_life_current_1101811": "fed3fd2df619f5e95f458e7efddfd2e2959741c189955874732959f9a2b8adbb",
    "fandom_formulae_1205374": "ee456878b9f9b0f35db6343081213b6aa34befe9f6c37cb1d9ab1f1119fb2cbe",
    'fandom_vibrancy_archive_2025': 'a808e488e02ca1de01f2b5c5727ecdc0e02e4c07f87393f01a4502c6a290c91c',
    'fandom_life_archive_2026': 'fe6695794393258c41af4ae90a1515d56cc9791f23f7b0f8fd24e1a11dddfef5',
    "tibiaqa_life_equal_hit_2020": "94f1f5bd9de7817102011e5e67dd2e8190964c6b31be6a8fe4eaa697abe730b1",
    "tibiaqa_life_answer_revisions_14497": "f7b394be74de12298949ef92345fdc9e6b906341af5c73b84debe9e7485be10c",
    "tibiaqa_wheel_void_example_2022": "1e252cf5ec76ebdd93f9a54c496cb0839e79edbd9941614b85ebe8d7f863c192",
    "tibiaqa_charm_leech_test_2020": "2e259cb41975504a73d299bcd9f20184b63fee1e371b858ac9a63bf8e641ecb7",
    "tibiaqa_basic_frost_elemental_ammo_2021": "22ce48210f88200145d853b037fe681078ff18d07c79a8f61e75d5b79ff1050b",
    "fandom_imbuing_full": "0e5e7f209e6b85a3491578b94606857a2b7dc9e86b7c5076e73c55ec3f4cb663",
}
NATIVE_REFERENCE_CONFLICT = {'id': 'native_percentage_reduction_reference_vs_engine', 'bounded_profile': 'native_equipment_percentage_reduction_example', 'public_reference': 'Formulae1205374 named native pair: floor remaining damage at each item, then armor subtraction', 'engine_behavior': 'All pinned engines call Creature::blockHit before player item reduction; imbuement reduction uses ceil removed damage and native same-item absorb uses std::round removed damage', 'resolution': 'Preserve bounded public example and OTS source algorithms separately; no universal Global pipeline selected', 'current_global_pipeline': None}
BOUNDED_LINKS = {
    "protection_equipment_composition": ["native_equipment_percentage_reduction_example"],
    "leech_rounding": ["mana_leech_current_reference_formula", "life_leech_reported_equal_hit_ceiling"],
    "leech_equipment_composition": ["leech_two_powerful_equipment_pairs", "mana_leech_wheel_equipment_example"],
    "leech_unequal_damage_and_overkill_order": [
        "mana_leech_current_reference_formula", "life_leech_reported_equal_hit_ceiling",
        "leech_elemental_parry_wound_reported_exclusions", "life_leech_damage_prey_exclusion"],
}
RULE_TIME_STATUSES = {
    **{name: "CURRENT_REFERENCE_NOT_GLOBAL_RUNTIME_OBSERVATION" for name in RULE_STATUSES},
    "vibrancy_sequence": "HISTORICAL_INTRODUCTION_WITH_CURRENT_COMMUNITY_CORROBORATION",
    "vibrancy_pvp_gate": "CURRENT_COMMUNITY_CONDITIONAL_PVP_WORDING_STATE_GATE_UNRESOLVED",
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
    if packet["target"] != TARGET:
        raise ValueError("combat reference must use the owner-selected current evaluation date")
    if packet["activation"] != "DRAFT_NOT_RUNTIME_READY":
        raise ValueError("combat evidence cannot activate runtime")
    capture_bounds.validate(packet, "imbuement-combat.json")
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
                      | {"fandom_vibrancy_archive_2025", "fandom_vibrancy_current_1194726"}):
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
    native_conflicts = [c for c in packet["source_conflicts"]
                        if c.get("id") == NATIVE_REFERENCE_CONFLICT["id"]]
    if (len(native_conflicts) != 1 or json.dumps(native_conflicts[0], sort_keys=True)
            != json.dumps(NATIVE_REFERENCE_CONFLICT, sort_keys=True)):
        raise ValueError("native reference example must retain unresolved engine-order discrepancy")
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
        raise ValueError("mana formula must retain its current dated community provenance")
    for source_id, date in (("tibiaqa_two_mana_2018", "2018-09-27"),
                            ("tibiaqa_two_leech_pairs_2021", "2021-05-27")):
        if sources[source_id]["published_on"] != date:
            raise ValueError("bounded equipment observations must retain their captured dates")
    if any(h["status"] != "OTS_HYPOTHESIS_ONLY" for h in packet["engine_combat_hypotheses"]):
        raise ValueError("engine hypotheses cannot be promoted to Global evidence")
    for name in ("vibrancy_sequence", "vibrancy_pvp_gate"):
        if not {"fandom_vibrancy_archive_2025", "fandom_vibrancy_current_1194726"}.issubset(rules[name]["evidence"]):
            raise ValueError("Vibrancy profiles must retain current and historical conditional references")
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
