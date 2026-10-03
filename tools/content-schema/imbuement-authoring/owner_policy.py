"""Validate approved authoring choices without promoting them to Global evidence."""
from __future__ import annotations

import json
from pathlib import Path

PACKET = Path(__file__).parent / "samples/owner-authoring-policy.json"
DECISION_IDS = {
    "etcher_consumption", "filled_scroll_consumption",
    "scroll_application_equipped_target", "etcher_npc_purchase_requires_premium",
    "transfer_preserves_imbuement_state_and_remaining_duration",
    "life_leech_damage_basis", "life_leech_aoe_zero_damage_targets",
    "life_leech_aoe_formula_and_rounding", "imbuement_timer_pz_rules",
    "physical_armor_and_percentage_order",
}


def illustrated_life_heal(damages, share_bps, missing_hp):
    """Exact arithmetic for policy fixtures only; this is not a runtime effect."""
    if (any(type(d) is not int or d < 0 for d in damages)
            or type(share_bps) is not int or not 0 <= share_bps <= 10000
            or type(missing_hp) is not int or missing_hp < 0):
        raise ValueError("invalid policy illustration inputs")
    positive = [d for d in damages if d > 0]
    n = len(positive)
    if not n:
        return 0
    denominator = 100000 * n
    total = sum((d * share_bps * (n + 9) + denominator - 1) // denominator
                for d in positive)
    return min(total, missing_hp)


def validate(packet, packets=None):
    def require(condition, message):
        if not condition:
            raise ValueError(message)

    require(packet["schema"] == "OTERYN_IMBUEMENT_OWNER_AUTHORING_POLICY/v1"
            and packet["activation"] == "DRAFT_NOT_RUNTIME_READY"
            and packet["policy_status"] == "OWNER_APPROVED_AUTHORING_POLICY"
            and packet["full_global_parity_proven"] is False,
            "owner approval cannot activate runtime or certify Global")
    require(packet["approved_on"] == "2026-10-02"
            and packet["approval_timezone"] == "Europe/Warsaw"
            and packet["runtime_owner"] == "ARCHITECT_COORDINATOR_162",
            "owner policy date or runtime ownership changed")
    d = packet["decisions"]
    require(set(d) == DECISION_IDS and packet["counts"] == {"owner_approved_decisions": 10},
            "approved decision inventory changed")
    for row in d.values():
        require(row["status"] == "OWNER_APPROVED_AUTHORING_POLICY"
                and row["global_public_confirmation"] is False
                and row["approval_source"] == "EXPLICIT_USER_DECISION_IN_CURRENT_CONVERSATION"
                and row["approved_on"] == packet["approved_on"] and row["basis"],
                "owner policy cannot become a public Global observation")
    for name in ("etcher_consumption", "filled_scroll_consumption"):
        row = d[name]
        require(type(row["success_units"]) is int and row["success_units"] == 1
                and row["consumption_scope"] == "SUCCESS_OR_ORDINARY_PRE_DEBIT_VALIDATION_REJECTION"
                and row["late_internal_failure_or_crash_net_consumption"] is None,
                "consumption cannot generalize validation to late failure")
    require(type(d["etcher_consumption"]["ordinary_rejection_units"]) is int
            and d["etcher_consumption"]["ordinary_rejection_units"] == 0
            and d["etcher_consumption"]["units_independent_of_removed_slot_count"] is True
            and d["filled_scroll_consumption"]["rejection_units"]
            == {"full_slots": 0, "incompatible_item": 0, "duplicate_family": 0},
            "approved ordinary consumption values changed")
    require(d["scroll_application_equipped_target"]["allowed"] is True
            and d["scroll_application_equipped_target"]["requires_unequipping"] is False
            and d["scroll_application_equipped_target"]["normal_item_slot_and_family_validation_applies"] is True,
            "equipped scroll policy cannot bypass compatibility")
    require(d["etcher_npc_purchase_requires_premium"]["premium_required"] is False
            and d["etcher_npc_purchase_requires_premium"]["price_gold"] == 30000
            and d["etcher_npc_purchase_requires_premium"]["other_offer_access_requirements_preserved"] is True,
            "Etcher purchase policy changed")
    transfer = d["transfer_preserves_imbuement_state_and_remaining_duration"]
    require(all(transfer[k] is True for k in ("preserves_imbuement_family_and_tier",
                "preserves_exact_remaining_duration", "normal_eligible_use_timer_rules_remain_applicable"))
            and transfer["ownership_change_resets_duration"] is False
            and transfer["ownership_change_deducts_duration"] is False,
            "ownership transfer cannot reset, deduct or freeze eligible use")
    basis = d["life_leech_damage_basis"]
    zero = d["life_leech_aoe_zero_damage_targets"]
    formula = d["life_leech_aoe_formula_and_rounding"]
    require(basis["damage_basis"] == "ATTACK_DAMAGE_BEFORE_TARGET_REMAINING_HP_CAP"
            and basis["overkill_damage_counts"] is True
            and basis["based_on_target_maximum_hp"] is False
            and basis["healing_capped_by_caster_missing_hp"] is True
            and basis["single_target_vampirism_share_bps"] == {"basic": 500, "intricate": 1000, "powerful": 2500},
            "approved Life damage basis changed")
    require(zero["zero_damage_target_counts_in_N"] is False
            and zero["zero_damage_target_generates_leech"] is False
            and zero["empty_positive_target_set_total_healing"] == 0
            and formula["N"] == "COUNT_OF_POSITIVE_DAMAGE_TARGETS",
            "zero damage must not increase the Life divisor")
    require(formula["scope"] == "LIFE_LEECH"
            and formula["formula"] == "sum(ceil(D_i * P * (0.9 + 0.1 * N) / N))"
            and formula["integer_reference_expression"] == "sum(ceil_div(D_i * share_bps * (N + 9), 100000 * N))"
            and formula["rounding"] == "CEIL_EACH_TARGET_BEFORE_SUM"
            and formula["healing_capped_by_caster_missing_hp"] is True,
            "Life formula cannot change rounding or become a Mana policy")
    timers = d["imbuement_timer_pz_rules"]
    combat = timers["combat_imbuements"]
    utility = timers["non_combat_imbuements"]
    require(combat["requires_equipped"] is True and combat["requires_active_combat_state"] is True
            and combat["entering_pz_immediately_pauses_timer"] is False
            and type(combat["ordinary_combat_residual_seconds"]) is int
            and combat["ordinary_combat_residual_seconds"] == 60
            and combat["residual_origin"] == "LAST_EVENT_REFRESHING_ORDINARY_COMBAT_STATE"
            and combat["entering_pz_resets_residual_deadline"] is False
            and combat["special_pvp_state_rules_not_generalized"] is True,
            "ordinary timer deadline cannot start at PZ entry")
    require(set(utility["families"]) == {"Swiftness", "Featherweight", "Vibrancy"}
            and utility["requires_equipped"] is True and utility["requires_online_character"] is True
            and utility["combat_required"] is False and utility["continues_in_pz"] is True
            and timers["unequipped_use_budget_paused"] is True,
            "utility timers must remain independent of combat/PZ")
    armor = d["physical_armor_and_percentage_order"]
    require(armor["stage_order"] == ["DEFENSE_AND_FLAT_ARMOR_WHEN_ENABLED", "EQUIPPED_ITEM_ABSORPTION", "WHEEL_RESISTANCE"]
            and armor["item_absorption_order"] == ["IMBUEMENT_ABSORB", "APPLICABLE_NATIVE_ITEM_ABSORB"]
            and armor["imbuement_reduction_rounding"] == "CEIL_REDUCTION_AMOUNT"
            and armor["native_reduction_rounding"] == "ROUND_REDUCTION_AMOUNT"
            and armor["percentage_basis"] == "CURRENT_REMAINING_DAMAGE_AT_EACH_STEP"
            and armor["equipment_percentages_summed_across_items"] is False
            and armor["armor_applies_only_if_attack_enables_armor_check"] is True
            and armor["unrelated_mantra_proficiency_critical_and_wheel_formulas_selected"] is False,
            "approved armor order or rounding changed")
    for case in packet["life_reference_cases"]:
        require(illustrated_life_heal(case["damage_by_target"], case["share_bps"], case["missing_hp"])
                == case["expected_heal"], "policy arithmetic illustration mismatch")
    if packets is not None:
        closure = packets["global-research-closure.json"]
        require(closure["full_global_parity_proven"] is False, "owner policy cannot rewrite public parity")
        for group in closure["groups"]:
            expected = {"owner-authoring-policy.json#" + name for name, row in d.items()
                        if group["id"] in row["research_group_refs"]}
            require(set(group.get("owner_selected_policy_refs", [])) == expected,
                    "public research/owner policy links changed")
        native = next(q for q in packets["current-behavior-answers.json"]["questions"]
                      if q["id"] == "native_effect_composition")
        expected = [{"engine": engine, "revision": a["revision"], "source_ref": anchor["source_ref"], "url": anchor["url"]}
                    for engine, a in native["engine_answers"].items() for anchor in a["code_anchors"][:3]]
        require(packet["armor_code_refs"] == expected, "pinned armor code references changed")


if __name__ == "__main__":
    validate(json.loads(PACKET.read_text()))
    print("PASS: ten owner-approved draft policies; Global certification unchanged")
