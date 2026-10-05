"""The ordinary importer retains policy facts and rejects unresolved mechanics."""
import copy
import json
from pathlib import Path
import unittest

from current_spell_adapter import DONORS
from source_model_lowering import lower_combat_controller_roles, lower_party_source_model, lower_source_model


# Authoring tests use the immutable qualified baseline: regenerating product
# imports may legitimately change effect execution order in the live catalog.
CATALOG = json.loads((Path(__file__).with_name("samples") / "executable-spell-catalog.json").read_text())


def entry(name, carrier="instant"):
    return copy.deepcopy(next(row for row in CATALOG["bundles"]
                              if row["bundle"]["spell"]["name"] == name and
                              row["bundle"]["spell"]["carrier"] == carrier))


def source(current):
    bundle, deps = copy.deepcopy(current["bundle"]), copy.deepcopy(current["dependencies"])
    bundle["spell"]["identity"] = {"key": "candidate:spell/source/test/a", "revision": "source-r1"}
    return bundle, deps


def combined_dot(current):
    bundle, deps = source(current)
    condition = next(effect for effect in deps["effects"] if effect["operation"] == "condition")
    presentation = next(effect for effect in deps["effects"] if effect["operation"] == "presentation_only")
    condition["presentation"] = copy.deepcopy(presentation["presentation"])
    condition["identity"] = {"key": "candidate:effect/source/combined", "revision": "source-r1"}
    deps["effects"] = [condition]
    deps["abilities"][0]["effects"] = [{"family": "Effect", **condition["identity"]}]
    deps["abilities"][0]["zero_damage_health_path"] = True
    return bundle, deps


def private_party(current):
    bundle, _ = source(current)
    name = current["bundle"]["spell"]["name"]
    values = {"Enchant Party": {"stat_magicpoints": 1}, "Protect Party": {"skill_shield": 3},
              "Train Party": {"skill_distance": 3, "skill_melee": 3},
              "Heal Party": {"healthgain": 20, "healthticks": 2000},
              "Enlighten Party": {"managain": 50, "manaticks": 3000}}[name]
    native = current["bundle"]["spell"]["execution"]["native_behavior"]["parameters"]
    bundle["spell"]["execution"]["native_behavior"]["parameters"] = {
        "source_model": "r61-party_buff",
        "combat_presentation": {"aggressive": False, "areas": [], "condition_application_independent_of_area": True,
                                "effect": "CONST_ME_MAGIC_BLUE", "execute_failure_effect": "poff",
                                "execute_failure_message": "not_possible"},
        "conditions": [{"condition_type": "CONDITION_REGENERATION" if "Party" in name and name in
                        {"Heal Party", "Enlighten Party"} else "CONDITION_ATTRIBUTES",
                        "parameters": {"buff_spell": 1, "subid": 1, "ticks": 300000 if name == "Enlighten Party" else 120000,
                                       **values}, "replacement": "source_condition_type_id_subid_rules"}],
        "membership": {"deduplicate": False, "distance_lte": 36, "distance_metric": "max_abs_xyz",
                       "min_selected_count": 2, "requires_party": True, "requires_source_list_size_greater_than_one": True,
                       "same_floor_required": False, "source": "members_then_append_leader",
                       "source_list_type_guard_after_leader_append": True},
        "mana": {"add_mana_spent": "total_minus_registrar_base", "additional_debit": "total_minus_registrar_base",
                 "check_total_before_presentation": True, "count_multiplier": True, "debit_notify": False,
                 "extra_delta_can_be_negative": True, "geometric_ratio": 0.9, "mana_spent_numeric_input_type": "uint64_t",
                 "mode": "ceil_geometric_party_size", "power_offset": -1, "refund_on_failed_cast": False,
                 "registrar_base": 120 if name == "Enlighten Party" else native["mana"]["base"],
                 "registrar_cost_after_success": True},
        "commit_order": ["select_members", "reject_small_selection", "compute_total_mana", "check_total_mana",
                         "execute_combat_or_fail", "debit_extra_mana", "record_extra_mana_spent",
                         "apply_per_member_commits_in_member_list_order", "return_true", "registrar_success_cost"],
        "per_member_commit_order": ["add_condition"], "member_success_presentation": None,
        "no_party_or_insufficient_members": {"effect": "poff", "message": "No party members in range.", "return": False},
        "insufficient_mana": {"effect": "poff", "message": "RETURNVALUE_NOTENOUGHMANA",
                              "order": ["cancel_message", "caster_poff", "return_false"], "return": False}}
    return bundle, {"abilities": [], "effects": [], "formulas": []}


def controller_cues():
    aliases = {}
    existing = {}
    for token, number, name in (("CONST_ME_WHITE_TIGERCLASH", 282, "claw_white"),
                                ("CONST_ME_MAGIC_GREEN", 15, "magic_green"),
                                ("CONST_ME_MAGIC_BLUE", 13, "magic_blue")):
        alias = "canary.appearance:effect/" + name
        aliases[token] = {"alias": alias, "source_numeric_value": number, "proof": {
            "revision": DONORS["crystal-summer-current"][2], "path": "src/utils/utils_definitions.hpp", "sha256": "a" * 64}}
        existing[alias] = {"kind": "effect", "value": number}
    return aliases, existing


def private_equipment(current):
    bundle, deps = source(current)
    bundle["spell"]["execution"] = {"native_behavior": {"key": "equipment_attack", "parameters": {
        "source_model": "r60_equipment_source_v1", "combat": {}, "consumer_contract_pending": True,
        "default_cost_commit": "source_cast_return_true_only", "equipment": {},
        "harmony": {"gain": 1, "spend_all": False}, "routes": [{
            "ability": {"family": "Ability", **deps["abilities"][0]["identity"]}, "area_variant": "default",
            "damage_type": "physical", "hit_order": 0, "source_combat_index": 0, "source_phase": "primary"}]}}}
    deps["effects"][0]["presentation"] = {"impact_asset_binding": "source:effect/const_me_white_tigerclash"}
    deps["formulas"][0]["minimum"] = {"const": "100"}
    deps["formulas"][0]["maximum"] = {"const": "200"}
    return bundle, deps


def private_conservation(current):
    bundle, _ = source(current)
    combat = {"aggressive": False, "area": None, "block_armor": False,
        "callback_bindings": [{"function": "onGetFormulaValues", "kind": "CALLBACK_PARAM_LEVELMAGICVALUE"}],
        "chain_effect": "CONST_ME_NONE", "condition_declarations": [], "dispel_condition": "CONDITION_PARALYZE",
        "health_type": "COMBAT_HEALING", "impact_effect": "CONST_ME_MAGIC_GREEN", "projectile_effect": "CONST_ANI_NONE",
        "source_combat_index": 0, "source_parameter_bindings": {"COMBAT_PARAM_AGGRESSIVE": False,
            "COMBAT_PARAM_DISPEL": "CONDITION_PARALYZE", "COMBAT_PARAM_EFFECT": "CONST_ME_MAGIC_GREEN",
            "COMBAT_PARAM_TYPE": "COMBAT_HEALING"}, "use_charges": False}
    ast = {"minimum": {"const": "111"}, "maximum": {"const": "222"},
        "evaluation": "lua_Number_then_source_LuaCombat_binding_int32",
        "pair_semantics": "raw_source_signed_callback_pair_no_absolute_value_projection",
        "sampling": "source_normal_random_inclusive"}
    bundle["spell"]["execution"] = {"native_behavior": {"key": "shared_conservation", "parameters": {
        "caster_pre_primary_blue": True, "combat_bindings": {"primary": 0, "secondary_shared_conservation": 1},
        "combat_definitions": [combat], "execution_order": ["primary_execute", "secondary_helper", "return_primary_result"],
        "formula_input_binding": {}, "formula_math_semantics": "Lua_binary64_with_ordered_left_fold_for_nary_arithmetic",
        "helper_definitions": [], "party_selector": {}, "primary_formula": ast, "secondary_formula": {},
        "secondary_attempted_even_primary_false": True, "self_refusal": False, "source_cast_return": "combat_result",
        "source_costs_unchanged": True, "source_function_scopes": [], "source_model": "r62/heal_friend",
        "source_monk_spell_type": None, "source_sound_bindings": []}}}
    return bundle, {"abilities": [], "effects": [], "formulas": []}


class LoweringTests(unittest.TestCase):
    def test_equipment_roles_keep_operation_ids_and_real_alias_with_numeric_hold(self):
        current = entry("Double Jab")
        bundle, deps = private_equipment(current)
        original = copy.deepcopy((bundle, deps, current))
        result = lower_combat_controller_roles(bundle, deps, current, *controller_cues())
        self.assertIsNotNone(result["bundle"])
        self.assertEqual(result["dependencies"]["formulas"], current["dependencies"]["formulas"])
        self.assertEqual(result["dependencies"]["effects"][0]["identity"], current["dependencies"]["effects"][0]["identity"])
        self.assertEqual(result["dependencies"]["effects"][0]["presentation"]["impact_asset_binding"],
                         "canary.appearance:effect/claw_white")
        self.assertEqual(result["bundle"]["spell"]["harmony_role"], "builder")
        self.assertTrue(any(row["missing_class"] == "SOURCE_CONFLICT_KEPT" for row in result["remaining"]))
        self.assertTrue(result["partial_source_scope"])
        self.assertEqual((bundle, deps, current), original)

    def test_equipment_does_not_launder_invalid_ast_alias_or_controller_identity(self):
        for mutation in ("bad_ast", "wrong_alias_number", "missing_binding", "wrong_enum_pin", "wrong_words",
                         "extra_route_field", "ambiguous_route", "wrong_reference_family", "harmony_conflict", "extra_effect",
                         "unknown_execution", "null_params"):
            with self.subTest(mutation=mutation):
                current = entry("Double Jab")
                bundle, deps = private_equipment(current)
                params = bundle["spell"]["execution"]["native_behavior"]["parameters"]
                aliases, existing = controller_cues()
                if mutation == "bad_ast":
                    deps["formulas"][0]["minimum"] = {"op": "floor", "args": [{"const": "1"}, {"const": "2"}]}
                elif mutation == "wrong_alias_number":
                    aliases["CONST_ME_WHITE_TIGERCLASH"]["source_numeric_value"] = 10
                elif mutation == "missing_binding":
                    existing.clear()
                elif mutation == "wrong_enum_pin":
                    aliases["CONST_ME_WHITE_TIGERCLASH"]["proof"]["revision"] = "b" * 40
                elif mutation == "wrong_words":
                    bundle["spell"]["words"] = "exori bogus"
                elif mutation == "extra_route_field":
                    params["routes"][0]["callback"] = "other"
                elif mutation == "ambiguous_route":
                    params["routes"].append(copy.deepcopy(params["routes"][0]))
                elif mutation == "wrong_reference_family":
                    params["routes"][0]["ability"]["family"] = "Effect"
                elif mutation == "harmony_conflict":
                    params["harmony"]["gain"] = 0
                    params["harmony"]["spend_all"] = True
                elif mutation == "extra_effect":
                    deps["effects"][0]["unbound_mechanic"] = True
                elif mutation == "unknown_execution":
                    bundle["spell"]["execution"]["other_owner_commit"] = {"remove_item": 1}
                else:
                    bundle["spell"]["execution"]["native_behavior"]["parameters"] = None
                result = lower_combat_controller_roles(bundle, deps, current, aliases, existing)
                self.assertIsNone(result["bundle"])
                self.assertTrue(result["remaining"])

    def test_conservation_primary_uses_existing_heal_dispel_roles_and_retains_real_donor_ast(self):
        current = entry("Heal Friend")
        bundle, deps = private_conservation(current)
        ast = bundle["spell"]["execution"]["native_behavior"]["parameters"]["primary_formula"]
        ast["minimum"] = {"operator": "add", "arguments": [{"const": "111"}, {"const": "2"}, {"const": "3"}]}
        original = copy.deepcopy((bundle, deps, current))
        result = lower_combat_controller_roles(bundle, deps, current, *controller_cues())
        self.assertIsNotNone(result["bundle"])
        self.assertEqual(result["dependencies"]["formulas"], current["dependencies"]["formulas"])
        self.assertEqual(result["dependencies"]["effects"], current["dependencies"]["effects"])
        donor = next(row["value"] for row in result["preserved_fields"] if row["path"] == "/dependencies/formulas/0")
        self.assertEqual(donor["minimum"], {"op": "add", "args": [
            {"op": "add", "args": [{"const": "111"}, {"const": "2"}]}, {"const": "3"}]})
        self.assertEqual(result["remaining"][0]["missing_class"], "SOURCE_CONFLICT_KEPT")
        self.assertFalse(result["runtime_activation"])
        self.assertEqual((bundle, deps, current), original)

    def test_conservation_rejects_unknown_fields_loss_of_dispel_wrong_sign_and_missing_S5(self):
        for mutation in ("unknown_combat", "missing_dispel", "wrong_sign", "different_commit", "missing_S5", "extra_AST", "null_caster"):
            with self.subTest(mutation=mutation):
                current = entry("Heal Friend")
                bundle, deps = private_conservation(current)
                params = bundle["spell"]["execution"]["native_behavior"]["parameters"]
                if mutation == "unknown_combat":
                    params["combat_definitions"][0]["delay_ms"] = 100
                elif mutation == "missing_dispel":
                    params["combat_definitions"][0]["dispel_condition"] = None
                elif mutation == "wrong_sign":
                    params["primary_formula"]["minimum"] = {"const": "-111"}
                elif mutation == "different_commit":
                    params["execution_order"].reverse()
                elif mutation == "missing_S5":
                    params["primary_formula"]["minimum"] = {"helper": "crystal_base_damage_healing", "input": "level"}
                    current["manifest"]["entries"] = []
                elif mutation == "extra_AST":
                    params["primary_formula"]["minimum"]["other"] = True
                else:
                    params["caster_pre_primary_blue"] = None
                result = lower_combat_controller_roles(bundle, deps, current, *controller_cues())
                self.assertIsNone(result["bundle"])
                self.assertTrue(result["remaining"])

    def test_party_projects_existing_payload_with_specific_C3_evidence_and_retained_scope(self):
        for name in ("Enchant Party", "Enlighten Party", "Heal Party", "Protect Party", "Train Party"):
            with self.subTest(name=name):
                current = entry(name)
                bundle, deps = private_party(current)
                original = copy.deepcopy((bundle, deps, current))
                result = lower_party_source_model(bundle, deps, current)
                self.assertFalse(result["remaining"])
                self.assertEqual(result["bundle"], current["bundle"])
                self.assertEqual(result["dependencies"], current["dependencies"])
                self.assertEqual(result["scope"], "accepted_partial_model")
                self.assertTrue(result["partial_source_scope"])
                self.assertEqual(result["preserved_fields"][-1]["value"], bundle["spell"])
                self.assertEqual((bundle, deps, current), original)

    def test_party_requires_exact_field_precedence_and_current_policy_shape(self):
        for mutation in ("missing_evidence", "wrong_effect", "extra_source_field", "unsupported_commit", "null_membership"):
            with self.subTest(mutation=mutation):
                current = entry("Enlighten Party")
                bundle, deps = private_party(current)
                params = bundle["spell"]["execution"]["native_behavior"]["parameters"]
                if mutation == "missing_evidence":
                    current["manifest"]["entries"] = []
                elif mutation == "wrong_effect":
                    current["dependencies"]["effects"][0]["condition"]["regeneration"]["mana_gain"] = 99
                elif mutation == "extra_source_field":
                    params["conditions"][0]["parameters"]["private_amplification"] = 100
                elif mutation == "unsupported_commit":
                    params["commit_order"].append("remove_item")
                else:
                    params["membership"]["distance_lte"] = None
                result = lower_party_source_model(bundle, deps, current)
                self.assertIsNone(result["bundle"])
                self.assertTrue(result["remaining"])

    def test_party_does_not_rebind_wrong_reference_words_or_failure_types(self):
        for mutation in ("reference_spell_id", "words", "no_party_return", "insufficient_return", "insufficient_order"):
            with self.subTest(mutation=mutation):
                current = entry("Heal Party")
                bundle, deps = private_party(current)
                params = bundle["spell"]["execution"]["native_behavior"]["parameters"]
                if mutation == "reference_spell_id":
                    bundle["spell"][mutation] = 9999
                elif mutation == "words":
                    bundle["spell"][mutation] = "adori bogus"
                elif mutation == "no_party_return":
                    params["no_party_or_insufficient_members"]["return"] = None
                elif mutation == "insufficient_return":
                    params["insufficient_mana"]["return"] = None
                else:
                    params["insufficient_mana"]["order"].reverse()
                result = lower_party_source_model(bundle, deps, current)
                self.assertIsNone(result["bundle"])
                self.assertTrue(result["remaining"])

    def test_ui_flags_are_retained_and_do_not_gate_casting(self):
        current = entry("Light")
        bundle, deps = source(current)
        flags = [{"vocation": "sorcerer", "show_in_description": False}]
        bundle["spell"]["requirements"]["vocation_display_flags"] = flags
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        self.assertEqual(result["bundle"], current["bundle"])
        self.assertEqual(result["preserved_fields"][0]["value"], flags)
        self.assertIn("vocation_display_flags", bundle["spell"]["requirements"])

    def test_exact_numeric_item_alias_preserves_canonical_reference(self):
        current = entry("Fireball Rune")
        bundle, deps = source(current)
        for field in ("reagent", "result"):
            ref = bundle["spell"]["execution"]["conjure"][field]
            ref["key"] = ref["key"].replace("candidate:item/", "candidate:item/source/canary-main-current/")
            ref["revision"] = "source-r1"
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        self.assertEqual(result["bundle"], current["bundle"])
        self.assertEqual(len(result["preserved_fields"]), 2)

    def test_unknown_numeric_item_is_not_invented(self):
        current = entry("Fireball Rune")
        bundle, deps = source(current)
        bundle["spell"]["execution"]["conjure"]["result"]["key"] = "candidate:item/source/canary-main-current/999999"
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertEqual(result["remaining"][0]["owner"], "item-content")

    def test_created_field_item_alias_uses_existing_numeric_binding(self):
        current = entry("energybomb rune", "rune")
        bundle, deps = source(current)
        ref = next(effect["created_item"] for effect in deps["effects"] if "created_item" in effect)
        numeric = ref["key"].split("/")[-1]
        ref["key"] = "candidate:spell/source/canary-main-current/1234567890abcdef/item/" + numeric
        ref["revision"] = "source-r1"
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        self.assertEqual(result["dependencies"], current["dependencies"])

    def test_field_effect_order_changes_without_repurposing_existing_ids(self):
        current = entry("energybomb rune", "rune")
        unchanged = copy.deepcopy(current)
        bundle, deps = source(current)
        deps["effects"].reverse()
        deps["abilities"][0]["effects"].reverse()
        deps["abilities"][0]["zero_damage_health_path"] = True
        expected_roles = {effect["identity"]["key"]: effect["operation"]
                          for effect in current["dependencies"]["effects"]}
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        actual = result["dependencies"]
        self.assertEqual({effect["identity"]["key"]: effect["operation"]
                          for effect in actual["effects"]}, expected_roles)
        self.assertEqual([effect["operation"] for effect in actual["effects"]],
                         ["create_item", "presentation_only"])
        by_id = {effect["identity"]["key"]: effect for effect in actual["effects"]}
        self.assertEqual([by_id[ref["key"]]["operation"] for ref in actual["abilities"][0]["effects"]],
                         ["create_item", "presentation_only"])
        self.assertTrue(actual["abilities"][0]["zero_damage_health_path"])
        self.assertEqual(current, unchanged)

    def test_duplicate_effect_operations_require_unique_exact_role(self):
        current = entry("Light Healing")
        extra = copy.deepcopy(current["dependencies"]["effects"][1])
        extra["identity"]["key"] += "-other"
        extra["removed_condition"] = "poison"
        current["dependencies"]["effects"].append(extra)
        current["dependencies"]["abilities"][0]["effects"].append({"family": "Effect", **extra["identity"]})
        bundle, deps = source(current)
        deps["effects"][-1]["removed_condition"] = "fire"
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertTrue(any("AMBIGUOUS_OR_CHANGED_EFFECT_ROLE" in row["reason"]
                            for row in result["remaining"]))

    def test_exact_combined_DoT_keeps_schedule_sound_and_existing_effect_ids(self):
        current = entry("Curse")
        bundle, deps = combined_dot(current)
        original = copy.deepcopy((bundle, deps))
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        actual = result["dependencies"]
        self.assertEqual(actual["effects"], current["dependencies"]["effects"])
        self.assertEqual(actual["abilities"][0]["effects"], current["dependencies"]["abilities"][0]["effects"])
        self.assertTrue(actual["abilities"][0]["zero_damage_health_path"])
        self.assertEqual(result["bundle"]["spell"]["presentation"], bundle["spell"]["presentation"])
        self.assertTrue(any("decomposition" in row["reason"] for row in result["preserved_fields"]))
        self.assertEqual((bundle, deps), original)

    def test_combined_DoT_never_loses_extra_fields_or_changed_schedule(self):
        for mutation in ("extra", "condition_missing", "changed_schedule", "changed_presentation"):
            with self.subTest(mutation=mutation):
                current = entry("Curse")
                bundle, deps = combined_dot(current)
                combined = deps["effects"][0]
                if mutation == "extra":
                    combined["private_tick_hook"] = True
                elif mutation == "condition_missing":
                    del combined["condition"]
                elif mutation == "changed_schedule":
                    combined["condition"]["damage_over_time"]["fixed_ticks"][0]["interval_ms"] += 1
                else:
                    combined["presentation"]["projectile_asset_binding"] = "canary.appearance:missile/fire"
                result = lower_source_model(bundle, deps, current)
                self.assertIsNone(result["bundle"])
                self.assertTrue(any("COMBINED_CONDITION_PRESENTATION_" in row["reason"]
                                    for row in result["remaining"]))

    def test_combined_DoT_validates_original_identity_and_typed_reference(self):
        for mutation in ("identity_extra", "foreign_reference", "duplicate_identity"):
            with self.subTest(mutation=mutation):
                current = entry("Curse")
                bundle, deps = combined_dot(current)
                effect = deps["effects"][0]
                if mutation == "identity_extra":
                    effect["identity"]["family"] = "Ability"
                    deps["abilities"][0]["effects"] = [{"family": "Effect", **effect["identity"]}]
                elif mutation == "foreign_reference":
                    deps["abilities"][0]["effects"][0]["family"] = "Ability"
                else:
                    effect["identity"] = copy.deepcopy(deps["abilities"][0]["identity"])
                    deps["abilities"][0]["effects"] = [{"family": "Effect", **effect["identity"]}]
                result = lower_source_model(bundle, deps, current)
                self.assertIsNone(result["bundle"])
                self.assertTrue(result["remaining"])

    def test_S5_only_replaces_level_contribution_and_preserves_bounds(self):
        current = entry("Light Healing")
        bundle, deps = source(current)

        def legacy(value):
            if isinstance(value, list):
                return [legacy(child) for child in value]
            if not isinstance(value, dict):
                return value
            if value.get("fn") == "level_base_damage_healing":
                return {"op": "mul", "args": [{"var": "level"}, {"const": "0.2"}]}
            return {key: legacy(child) for key, child in value.items()}

        low = legacy(deps["formulas"][0]["minimum"])
        high = legacy(deps["formulas"][0]["maximum"])
        deps["formulas"][0]["minimum"] = {"op": "min", "args": [
            {"op": "abs", "args": [low]}, {"op": "abs", "args": [high]}]}
        deps["formulas"][0]["maximum"] = {"op": "max", "args": [
            {"op": "abs", "args": [low]}, {"op": "abs", "args": [high]}]}
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        self.assertEqual(result["dependencies"], current["dependencies"])
        self.assertTrue(any("S5_world_curve" in row["reason"] for row in result["preserved_fields"]))

    def test_known_official_mana_retained_with_source_value(self):
        current = entry("Light Healing")
        bundle, deps = source(current)
        bundle["spell"]["costs"]["mana"] = 123
        result = lower_source_model(bundle, deps, current)
        self.assertFalse(result["remaining"])
        self.assertEqual(result["bundle"]["spell"]["costs"]["mana"], current["bundle"]["spell"]["costs"]["mana"])
        self.assertTrue(any(row["path"] == "/spell/costs/mana" and row["value"] == 123
                            for row in result["preserved_fields"]))

    def test_unresolved_condition_survives_as_failure(self):
        current = entry("Light")
        bundle, deps = source(current)
        deps["effects"][0]["condition"]["private_timer_semantics"] = "script_owned"
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertTrue(any("private_timer_semantics" in row["reason"] for row in result["remaining"]))

    def test_private_state_contract_has_specific_owner(self):
        current = entry("Light")
        bundle, deps = source(current)
        bundle["spell"]["source_state_contract"] = {"private": "harmony"}
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertEqual(result["remaining"][0]["missing_class"], "CONTRACT_GAP")
        self.assertEqual(result["remaining"][0]["path"], "/spell/source_state_contract")

    def test_numeric_coefficient_conflict_is_not_erased(self):
        current = entry("Light Healing")
        bundle, deps = source(current)
        deps["formulas"][0]["minimum"] = {"const": "123"}
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertEqual(result["remaining"][0]["missing_class"], "SOURCE_CONFLICT_KEPT")

    def test_native_profile_and_wheel_cannot_be_replaced(self):
        current = entry("Avatar of Light")
        bundle, deps = source(current)
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertTrue(any(row["owner"] == "native-profile-owner" for row in result["remaining"]))
        self.assertTrue(any(row["owner"] == "SPELL-WHEEL-GATE-1" for row in result["remaining"]))

    def test_explicit_null_metadata_is_not_treated_as_absence(self):
        for field in ("vocation_display_flags", "source_cast_sound", "reference_rune_item_id"):
            with self.subTest(field=field):
                current = entry("Light")
                bundle, deps = source(current)
                parent = (bundle["spell"]["requirements"] if field == "vocation_display_flags" else
                          bundle["spell"]["presentation"] if field == "source_cast_sound" else bundle["spell"])
                parent[field] = None
                result = lower_source_model(bundle, deps, current)
                self.assertIsNone(result["bundle"])
                self.assertTrue(result["remaining"])

    def test_vocation_ui_must_match_unique_source_vocations(self):
        for rows in ([{"vocation": "none", "show_in_description": True}],
                     [{"vocation": "sorcerer", "show_in_description": True}] * 2):
            current = entry("Light")
            bundle, deps = source(current)
            bundle["spell"]["requirements"]["vocation_display_flags"] = rows
            result = lower_source_model(bundle, deps, current)
            self.assertIsNone(result["bundle"])
            self.assertTrue(result["remaining"])

    def test_world_curve_arguments_cannot_be_discarded(self):
        current = entry("Light Healing")
        bundle, deps = source(current)
        low = deps["formulas"][0]["minimum"]
        low["args"][0]["args"][0]["args"][0] = {"var": "magic_level"}
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertTrue(result["remaining"])

    def test_malformed_constant_is_held_with_concrete_diagnostic(self):
        current = entry("Light Healing")
        bundle, deps = source(current)
        deps["formulas"][0]["minimum"] = {"op": "abs", "args": [{"const": "not-a-number"}]}
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertTrue(any(row["reason"] == "source_formula_expression_shape_is_invalid"
                            for row in result["remaining"]))

    def test_no_numerical_reordering_in_formula_projection(self):
        current = entry("Light Healing")
        bundle, deps = source(current)
        expression = deps["formulas"][0]["minimum"]
        expression["args"] = list(reversed(expression["args"]))
        result = lower_source_model(bundle, deps, current)
        self.assertIsNone(result["bundle"])
        self.assertTrue(any(row["missing_class"] == "SOURCE_CONFLICT_KEPT" for row in result["remaining"]))


if __name__ == "__main__":
    unittest.main()
