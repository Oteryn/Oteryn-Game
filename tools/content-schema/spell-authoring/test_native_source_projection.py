"""Adversarial qualification of unchanged closed native-profile projections."""
import copy
import json
from pathlib import Path
import unittest

from native_source_projection import (project_native_candidate, project_native_controller_candidate,
                                      select_familiar_candidate, source_reference_catalog)


ROOT = Path(__file__).resolve().parents[3]
CATALOG = json.loads((ROOT / "content/abilities/definitions/player-spells.json").read_text())
SELECTION = json.loads((ROOT / "content/abilities/definitions/player-spell-selection.json").read_text())


def entry(name, carrier=None):
    return copy.deepcopy(next(row for row in CATALOG["bundles"]
                              if row["bundle"]["spell"]["name"] == name and
                              (carrier is None or row["bundle"]["spell"]["carrier"] == carrier)))


def source(target):
    bundle = copy.deepcopy(target["bundle"])
    bundle["spell"]["identity"] = {"key": "candidate:spell/source/canary-main-current/native-test",
                                   "revision": "source-player-test"}
    catalog = copy.deepcopy(target["catalog"])
    mapping = {}
    for reference in catalog["definitions"]:
        original = reference["key"]
        reference["key"] = original.replace("candidate:item/", "candidate:item/source/canary-main-current/")
        reference["revision"] = "source-player-test"
        mapping[original] = reference

    def rewrite(value):
        if isinstance(value, list):
            return [rewrite(child) for child in value]
        if not isinstance(value, dict):
            return value
        if set(value) == {"family", "key", "revision"} and value["family"] == "Item":
            return copy.deepcopy(mapping[value["key"]])
        return {key: rewrite(child) for key, child in value.items()}

    return rewrite(bundle), rewrite(target["dependencies"]), catalog


def project(bundle, deps, target, catalog):
    return project_native_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog)


def private_familiar(target):
    bundle, deps, catalog = source(target)
    p = bundle["spell"]["execution"]["native_behavior"]["parameters"]
    p.update(source_model="r61-familiar_summon", source_contract_version="current-pinned-r61",
        condition_sharing={"automatic_clone_all_owner_conditions": False, "future_haste_sharing": "companion_haste_source_policy",
                           "owner_speed_applied_at_creation": True, "spell_cooldown_subid": p["reference_spell_id"]},
        dynamic_inputs={"cooldown_rate": "config.RATE_SPELL_COOLDOWN", "duration": "config.FAMILIAR_TIME",
                        "look": "player.familiar_look_type", "unix_expiry": "player.kv.familiar-summon-time",
                        "vip_reduction": "config.VIP_FAMILIAR_TIME_COOLDOWN_REDUCTION",
                        "vocation": "player.vocation_base_id", "warning_handles": "player.FAMILIAR_TIMER_storage"},
        selection={"base_vocation_lookup": "FAMILIAR_ID", "creature_name": p["creature_name"],
                   "fallback_look_type_on_login": p["default_look_type"], "look_type_from_player": True,
                   "unknown_vocation_returns_false": True},
        current_helper_commit_order=["premium_guard", "summon_count_and_account_guard", "vocation_lookup",
            "compute_half_config_duration_seconds", "compute_vip_cooldown", "create_owned_monster", "apply_player_familiar_look",
            "register_familiar_death", "increase_speed_nonnegative", "caster_magic_blue", "creature_teleport", "save_unix_expiry",
            "schedule_expiry", "schedule_two_warnings", "apply_shared_spell_cooldown", "return_true"])
    p["expiry"].update(absent_player_or_creature_returns_true=True, warning_storage_reset=-1)
    p["login"].update(register_advance_event_before_selection=True,
                      remove_look_guard="(not_premium_and_has_look)_or_level_below_200")
    p["warning_dispatch"]["message_class"] = "loot"
    bundle["spell"]["name"] = "Summon " + p["vocation"].capitalize() + " Familiar"
    return bundle, deps, catalog


def private_barrier(target, expert=False):
    bundle, _, catalog = source(target)
    name = target["bundle"]["spell"]["name"]
    wall = name == "Magic Wall Rune"
    model, symbol = ("magic_wall", "ITEM_MAGICWALL") if wall else ("wild_growth", "ITEM_WILDGROWTH")
    effect = target["dependencies"]["effects"][0]
    catalog["definitions"] = [row for row in catalog["definitions"] if row["key"].endswith("/3180" if wall else "/3156")]
    parameters = {"callback_success": "implicit_nil", "combat_bindings": {"primary": 0}, "create_count": 1,
        "creation_failed": "callback_nil", "creation_position": "none_then_addItemEx" if expert else "target_position",
        "description_template": "Casted by: %s",
        "duration_seconds": {"distribution": "uniform_random_inclusive", "minimum": 16, "maximum": 24} if wall else
                            {"distribution": "constant", "minimum": 30, "maximum": 30},
        "expert_pvp_context_from_caster": expert, "formula_math_semantics": "Lua_binary64_with_ordered_left_fold_for_nary_arithmetic",
        "helper_definitions": [], "insert_error": "callback_false" if expert else "Game_createItem_failure_nil",
        "insert_flag": "FLAG_NOLIMIT" if expert else None,
        "item_duration": {"decay_to_default": 0, "mutation_scope": "shared_ItemType", "show_duration": True, "start_decaying": True},
        "item_variants": {symbol: int(effect["created_item"]["key"].rsplit("/", 1)[1]),
                          symbol + "_SAFE": int(effect["pvp_safe_item"]["key"].rsplit("/", 1)[1])},
        "safe_item_when": ["IsExpertPVP", "WORLD_TYPE_NO_PVP"] if expert else ["WORLDTYPE_OPTIONAL"],
        "source_cast_return": "combat_result", "source_costs_unchanged": True,
        "source_function_scopes": [{"body_sha256": "a" * 64, "line_start": 1,
                                    "symbol": "rune.onCastSpell", "typed_controller_binding": model}],
        "source_model": "r62/" + model, "source_monk_spell_type": None,
        "source_sound_bindings": {"castSound": "SOUND_EFFECT_TYPE_SPELL_OR_RUNE", "impactSound": "SOUND_EFFECT_TYPE_SPELL_MAGIC_WALL_RUNE"},
        "tile_guards": {"floor_change": "false", "missing": "false", "top_non_player_creature": "false"},
        "combat_definitions": [{"aggressive": True, "area": None, "block_armor": False,
            "callback_bindings": [{"function": "onCreateMagicWall" if wall else "onCreateWildGrowth", "kind": "CALLBACK_PARAM_TARGETTILE"}],
            "chain_effect": "CONST_ME_NONE", "condition_declarations": [], "dispel_condition": None,
            "health_type": "COMBAT_NONE", "impact_effect": "CONST_ME_NONE", "projectile_effect": "CONST_ANI_ENERGY",
            "source_combat_index": 0, "source_parameter_bindings": {"COMBAT_PARAM_DISTANCEEFFECT": "CONST_ANI_ENERGY"}, "use_charges": False}]}
    bundle["spell"]["execution"] = {"native_behavior": {"key": "tile_item_operation", "parameters": parameters}}
    return bundle, {"abilities": [], "effects": [], "formulas": []}, catalog


def private_disintegrate(target):
    bundle, deps, catalog = source(target)
    catalog["definitions"] = [row for row in catalog["definitions"] if row["key"].endswith("/3197")]
    p = {"combat_bindings": {}, "combat_definitions": [], "helper_definitions": [],
         "final_cancel": "RETURNVALUE_NOTPOSSIBLE", "final_effect": "CONST_ME_POFF", "iteration": "ipairs",
         "formula_math_semantics": "Lua_binary64_with_ordered_left_fold_for_nary_arithmetic",
         "items_missing": "skip_to_final_presentation", "tile_missing": "skip_to_final_presentation",
         "removal": {"action_id_required": 0, "excluded_item_ids": [4240, 4241, 4242, 4243, 4246, 4247, 4248],
                     "movable_required": True, "unique_id_strictly_greater_than": 65535},
         "source_cast_return": "literal_true_after_final_presentation", "source_costs_unchanged": True,
         "source_function_scopes": [{"body_sha256": "a" * 64, "line_start": 6,
                                     "symbol": "rune.onCastSpell", "typed_controller_binding": "desintegrate_rune"}],
         "source_model": "r62/desintegrate_rune", "source_monk_spell_type": None,
         "source_sound_bindings": {"castSound": "SOUND_EFFECT_TYPE_SPELL_OR_RUNE", "impactSound": "SOUND_EFFECT_TYPE_SPELL_DISINTEGRATE_RUNE"},
         "visited_item_limit": 500}
    bundle["spell"]["execution"] = {"native_behavior": {"key": "tile_item_operation", "parameters": p}}
    bundle["spell"]["targeting"]["aggressive"] = True
    return bundle, deps, catalog


class NativeProjectionTests(unittest.TestCase):
    def test_existing_s16_and_s22_policies_need_exact_evidence_and_typed_source_fields(self):
        target = entry("Avatar of Light")
        bundle, deps, catalog = source(target)
        bundle["spell"]["requirements"].update(learning_required=True, level=300)
        result, _, receipt = project(bundle, deps, target, catalog)
        self.assertEqual(result, target["bundle"])
        proofs = {row["field"]: row for row in receipt["field_provenance"]}
        self.assertEqual(proofs["/requirements/level"]["kind"], "accepted_source_policy_override")
        self.assertEqual(proofs["/requirements/learning_required"]["kind"], "accepted_source_policy_override")
        for mutation in ("missing_S16", "missing_S22", "wrong_type"):
            with self.subTest(mutation=mutation):
                current, candidate = copy.deepcopy(target), copy.deepcopy(bundle)
                if mutation == "missing_S16":
                    current["manifest"]["entries"] = [row for row in current["manifest"]["entries"]
                        if not row.get("resolution", "").startswith("S16:")]
                elif mutation == "missing_S22":
                    current["manifest"]["entries"] = [row for row in current["manifest"]["entries"]
                        if not row.get("resolution", "").startswith("S22:")]
                else:
                    candidate["spell"]["requirements"]["learning_required"] = 1
                with self.assertRaises(ValueError):
                    project(candidate, deps, current, catalog)

    def test_group_leaf_override_needs_its_own_wiki_field_proof(self):
        target = entry("Sharpshooter")
        bundle, deps, catalog = source(target)
        bundle["spell"]["groups"][1]["group"] = "focus"
        result, _, receipt = project(bundle, deps, target, catalog)
        self.assertEqual(result, target["bundle"])
        self.assertIn("/groups/1/group", {row["field"] for row in receipt["field_provenance"]})
        target["manifest"]["entries"] = [row for row in target["manifest"]["entries"]
            if row.get("destination") != "/spell/spell/groups/1/group"]
        with self.assertRaisesRegex(ValueError, "SOURCE_CONFLICT_KEPT:/groups/1/group"):
            project(bundle, deps, target, catalog)

    def test_only_known_s27_header_conflict_pairs_have_field_proof_and_residual_flags(self):
        for name, field, original in (("Ice Burst", "reference_spell_id", 262),
                                      ("Terra Burst", "reference_spell_id", 263),
                                      ("Sharpshooter", "words", "utito tempo san")):
            with self.subTest(name=name):
                target = entry(name)
                bundle, deps, catalog = source(target)
                native = bundle["spell"]["execution"]["native_behavior"]
                native["parameters"]["source_model"] = "r59_accepted_" + native["key"]
                bundle["spell"][field] = original
                bundle["spell"]["source_state_contract"] = {"authoring_contract_extension_pending": True,
                    "chain_initial_selector": None, "native_execution_qualified": False,
                    "normalization": "S27_ACCEPTED_CANONICAL_NATIVE_PARAMETERS", "raw_source_equivalence": False,
                    "runtime_activation": False, "separate_augment_parameters": {}, "version": 1,
                    "wheel_augments_owner": "ProjectV2AugmentBinding"}
                result, _, receipt = project_native_controller_candidate(bundle, deps, target,
                    target["bundle"]["spell"]["identity"], catalog)
                self.assertEqual(result, target["bundle"])
                conflict = next(row for row in receipt["partial_source_scope"] if
                    row["reason"] == "accepted_explicit_wiki_or_official_native_header_pair_retained")
                self.assertEqual(conflict["missing_class"], "SOURCE_CONFLICT_KEPT")
                self.assertEqual(conflict["field_provenance"][0]["source"], original)
                self.assertTrue(conflict["field_provenance"][0]["field_provenance"])
                changed = copy.deepcopy(bundle)
                changed["spell"][field] = 9999 if field == "reference_spell_id" else "adori bogus"
                with self.assertRaisesRegex(ValueError, "NATIVE_CONTROLLER_SOURCE_HEADER_MISMATCH"):
                    project_native_controller_candidate(changed, deps, target, target["bundle"]["spell"]["identity"], catalog)
                no_proof = copy.deepcopy(target)
                no_proof["manifest"]["entries"] = [row for row in no_proof["manifest"]["entries"]
                    if row.get("destination") != "/spell/spell/" + field]
                with self.assertRaisesRegex(ValueError, "SOURCE_CONFLICT_KEPT:/" + field):
                    project_native_controller_candidate(bundle, deps, no_proof, no_proof["bundle"]["spell"]["identity"], catalog)

    def test_s27_already_normalized_base_keeps_private_contract_and_wheel_gate(self):
        contract = {"authoring_contract_extension_pending": True, "chain_initial_selector": None,
            "native_execution_qualified": False, "normalization": "S27_ACCEPTED_CANONICAL_NATIVE_PARAMETERS",
            "raw_source_equivalence": False, "runtime_activation": False, "separate_augment_parameters": {},
            "version": 1, "wheel_augments_owner": "ProjectV2AugmentBinding"}
        for name in ("Magic Shield", "Avatar of Light"):
            with self.subTest(name=name):
                target = entry(name)
                bundle, deps, catalog = source(target)
                native = bundle["spell"]["execution"]["native_behavior"]
                native["parameters"]["source_model"] = "r59_accepted_" + native["key"]
                bundle["spell"]["source_state_contract"] = copy.deepcopy(contract)
                result, projected, receipt = project_native_controller_candidate(bundle, deps, target,
                    target["bundle"]["spell"]["identity"], catalog)
                self.assertEqual(result, target["bundle"])
                self.assertEqual(projected, target["dependencies"])
                self.assertEqual(receipt["raw_source_model"]["bundle"], bundle)
                self.assertFalse(receipt["whole_source_controller_equivalence"])
                if target["bundle"]["spell"]["requirements"].get("wheel_unlock") is True:
                    self.assertTrue(receipt["wheel_unlock_required"])
                    self.assertNotIn("RUNTIME_GAP", {row["missing_class"] for row in receipt["partial_source_scope"]})
                    self.assertIn("/source_state_contract", receipt["partial_source_scope"][0]["source_fields"])
                for mutation in ("private_selector", "raw_parity", "null_contract", "unreviewed_behavior"):
                    with self.subTest(mutation=mutation):
                        changed = copy.deepcopy(bundle)
                        if mutation == "private_selector":
                            changed["spell"]["source_state_contract"]["chain_initial_selector"] = {"kind": "all_targets"}
                        elif mutation == "raw_parity":
                            changed["spell"]["source_state_contract"]["raw_source_equivalence"] = True
                        elif mutation == "null_contract":
                            changed["spell"]["source_state_contract"] = None
                        else:
                            changed["spell"]["execution"]["native_behavior"]["parameters"]["unreviewed_world_write"] = True
                        with self.assertRaises(ValueError):
                            project_native_controller_candidate(changed, deps, target, target["bundle"]["spell"]["identity"], catalog)

    def test_s27_augment_requires_real_orthogonal_area_and_keeps_private_parameters(self):
        target = entry("Energy Wave")
        bundle, deps, catalog = source(target)
        native = bundle["spell"]["execution"]["native_behavior"]
        native["parameters"]["source_model"] = "r59_accepted_" + native["key"]
        contract = {"authoring_contract_extension_pending": True, "chain_initial_selector": None,
            "native_execution_qualified": False, "normalization": "S27_ACCEPTED_CANONICAL_NATIVE_PARAMETERS",
            "raw_source_equivalence": False, "runtime_activation": False, "version": 1,
            "wheel_augments_owner": "ProjectV2AugmentBinding", "separate_augment_parameters": {
                "damage_or_heal_bonus_percent": {"0": 0, "1": 0, "2": 10}, "enhanced_area_from_stage": 1,
                "enhanced_area": {"clear_sight": True, "diagonal": None, "directional": False,
                    "encoding": "0_inactive_1_hit_2_origin_3_origin_hit", "orthogonal": [[1, 1, 3]], "same_floor": True},
                "wheel": {"owner": "wheel_of_destiny", "perk": "energy wave", "snapshot": "cast_start",
                          "stage_kind": "augment", "stage_max": 2, "stage_min": 0, "stage_zero": "base_cast"}}}
        bundle["spell"]["source_state_contract"] = contract
        _, _, receipt = project_native_controller_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog)
        self.assertEqual(receipt["raw_source_model"]["bundle"]["spell"]["source_state_contract"], contract)
        contract["separate_augment_parameters"]["enhanced_area"]["orthogonal"] = None
        with self.assertRaisesRegex(ValueError, "NATIVE_S27_SOURCE_AUGMENT_AREA_MATRIX_UNSUPPORTED"):
            project_native_controller_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog)

    def test_missing_presentation_object_requires_proven_leaf_fields(self):
        target = entry("Haste")
        bundle, deps, catalog = source(target)
        del bundle["spell"]["presentation"]
        _, _, receipt = project(bundle, deps, target, catalog)
        self.assertIn("/presentation/cast_cue", {row["field"] for row in receipt["field_provenance"]})
        target["manifest"]["entries"] = [row for row in target["manifest"]["entries"]
            if row.get("destination") != "/spell/spell/presentation/cast_cue"]
        with self.assertRaisesRegex(ValueError, "NATIVE_MISSING_HEADER_UNPROVEN:/presentation/cast_cue"):
            project(bundle, deps, target, catalog)

    def test_barrier_source_base_maps_to_existing_ids_and_preserves_world_and_policy_scopes(self):
        for name in ("Magic Wall Rune", "Wild Growth Rune"):
            for expert in (False, True):
                with self.subTest(name=name, expert=expert):
                    target = entry(name, "rune")
                    bundle, deps, catalog = private_barrier(target, expert)
                    result, projected, receipt = project_native_controller_candidate(
                        bundle, deps, target, target["bundle"]["spell"]["identity"], catalog)
                    self.assertEqual(result, target["bundle"])
                    self.assertEqual(projected, target["dependencies"])
                    self.assertEqual(receipt["raw_source_model"]["bundle"], bundle)
                    self.assertEqual({row["missing_class"] for row in receipt["partial_source_scope"]},
                                     {"ADAPTER_MISSING", "SOURCE_CONFLICT_KEPT"})
                    self.assertFalse(receipt["whole_source_controller_equivalence"])
                    self.assertEqual(len(receipt["numeric_reference_provenance"]), 2)

    def test_barrier_unknown_behavior_changed_base_and_missing_policy_remain_held(self):
        for mutation in ("extra_execution", "extra_param", "wrong_item", "changed_duration", "health_combat", "null_combat", "wrong_id", "no_policy", "integer_boolean"):
            with self.subTest(mutation=mutation):
                target = entry("Magic Wall Rune", "rune")
                bundle, deps, catalog = private_barrier(target)
                p = bundle["spell"]["execution"]["native_behavior"]["parameters"]
                if mutation == "extra_execution":
                    bundle["spell"]["execution"]["unreviewed_world_write"] = True
                elif mutation == "extra_param":
                    p["unreviewed_world_write"] = True
                elif mutation == "wrong_item":
                    p["item_variants"]["ITEM_MAGICWALL"] = 2130
                elif mutation == "changed_duration":
                    p["duration_seconds"]["maximum"] = 25
                elif mutation == "health_combat":
                    p["combat_definitions"][0]["health_type"] = "COMBAT_FIREDAMAGE"
                elif mutation == "null_combat":
                    p["combat_definitions"] = None
                elif mutation == "wrong_id":
                    bundle["spell"]["reference_spell_id"] = 9999
                elif mutation == "no_policy":
                    target["manifest"]["entries"] = []
                else:
                    p["item_duration"]["show_duration"] = 1
                with self.assertRaises(ValueError):
                    project_native_controller_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog)

    def test_disintegrate_maps_protected_removal_and_retains_actual_source_conflicts(self):
        target = entry("desintegrate rune", "rune")
        bundle, deps, catalog = private_disintegrate(target)
        result, projected, receipt = project_native_controller_candidate(bundle, deps, target,
            target["bundle"]["spell"]["identity"], catalog)
        self.assertEqual(result, target["bundle"])
        self.assertEqual(projected, target["dependencies"])
        self.assertEqual(receipt["partial_source_scope"][0]["missing_class"], "SOURCE_CONFLICT_KEPT")
        self.assertIn("/targeting/aggressive", receipt["partial_source_scope"][0]["source_fields"])
        self.assertEqual(receipt["raw_source_model"]["bundle"], bundle)
        for mutation in ("limit", "script_protection", "unknown", "missing_aggression_proof"):
            with self.subTest(mutation=mutation):
                current = copy.deepcopy(target)
                source_bundle, source_deps, source_catalog = private_disintegrate(target)
                p = source_bundle["spell"]["execution"]["native_behavior"]["parameters"]
                if mutation == "limit":
                    p["visited_item_limit"] = 501
                elif mutation == "script_protection":
                    p["removal"]["unique_id_strictly_greater_than"] = 0
                elif mutation == "unknown":
                    p["remove_immovable"] = True
                else:
                    current["manifest"]["entries"] = [row for row in current["manifest"]["entries"]
                        if not row.get("resolution", "").startswith("S27 D.2:")]
                with self.assertRaises(ValueError):
                    project_native_controller_candidate(source_bundle, source_deps, current,
                        current["bundle"]["spell"]["identity"], source_catalog)

    def test_private_familiar_projects_base_and_preserves_full_source_and_owner_gaps(self):
        target = entry("Druid familiar")
        bundle, deps, catalog = private_familiar(target)
        original = copy.deepcopy((bundle, deps, catalog, target))
        result, projected, receipt = project_native_controller_candidate(
            bundle, deps, target, target["bundle"]["spell"]["identity"], catalog, SELECTION)
        self.assertEqual(result, target["bundle"])
        self.assertEqual(projected, target["dependencies"])
        self.assertEqual(receipt["scope"], "accepted_partial_native_profile")
        self.assertFalse(receipt["whole_source_controller_equivalence"])
        self.assertFalse(receipt["raw_source_parity"])
        self.assertEqual(receipt["raw_source_model"]["bundle"], bundle)
        self.assertIn("/execution/native_behavior/parameters/condition_sharing",
                      receipt["partial_source_scope"][0]["source_fields"])
        self.assertEqual((bundle, deps, catalog, target), original)

    def test_private_familiar_unknown_changed_or_null_extensions_remain_held(self):
        target = entry("Druid familiar")
        for mutation in ("unknown", "sharing", "null", "condition", "extra_dependency", "wrong_id"):
            with self.subTest(mutation=mutation):
                bundle, deps, catalog = private_familiar(target)
                p = bundle["spell"]["execution"]["native_behavior"]["parameters"]
                if mutation == "unknown":
                    p["unreviewed_owner_write"] = True
                elif mutation == "sharing":
                    p["condition_sharing"]["automatic_clone_all_owner_conditions"] = True
                elif mutation == "null":
                    p["dynamic_inputs"] = None
                elif mutation == "condition":
                    p["cooldown"]["duration_multiplier"] = 3
                elif mutation == "extra_dependency":
                    deps["effects"].append({"identity": {"key": "extra", "revision": "extra"}})
                else:
                    bundle["spell"]["reference_spell_id"] = 194
                with self.assertRaises(ValueError):
                    project_native_controller_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog, SELECTION)

    def test_acquire_summon_preserves_target_copy_and_registrar_default_partial_scope(self):
        target = entry("animate dead rune", "rune")
        bundle, deps, catalog = source(target)
        bundle["spell"]["targeting"]["range_tiles"] = 0
        p = bundle["spell"]["execution"]["native_behavior"]["parameters"]
        p.update(source_model="r61-acquire_summon", source_contract_version="acquire-target-copy-r61",
                 inherit_master_attack_target={"clear_when_master_has_no_target": True, "enabled": True,
                                               "order": "after_set_master", "overwrite_existing": True})
        result, _, receipt = project_native_controller_candidate(bundle, deps, target,
            target["bundle"]["spell"]["identity"], catalog)
        self.assertEqual(result, target["bundle"])
        self.assertEqual(len(receipt["partial_source_scope"][0]["source_fields"]), 2)
        self.assertEqual(bundle["spell"]["targeting"]["range_tiles"], 0)
        bundle["spell"]["targeting"]["range_tiles"] = 7
        with self.assertRaisesRegex(ValueError, "NATIVE_ACQUIRE_SOURCE_RANGE_UNSUPPORTED"):
            project_native_controller_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog)

    def test_explicit_s21_familiar_alias_preserves_donor_and_selected_target(self):
        for vocation in ("Druid", "Knight", "Paladin", "Sorcerer"):
            with self.subTest(vocation=vocation):
                target = entry(vocation + " familiar")
                bundle, deps, catalog = source(target)
                donor_name = "Summon " + vocation + " Familiar"
                bundle["spell"]["name"] = donor_name
                selected, binding = select_familiar_candidate(bundle, CATALOG["bundles"], SELECTION)
                self.assertEqual(selected["bundle"]["spell"]["identity"], target["bundle"]["spell"]["identity"])
                result, dependencies, receipt = project_native_candidate(
                    bundle, deps, selected, selected["bundle"]["spell"]["identity"], catalog, SELECTION)
                self.assertEqual(result, target["bundle"])
                self.assertEqual(dependencies, target["dependencies"])
                self.assertEqual(receipt["source_selection"], binding)
                self.assertEqual(binding["source_name"], donor_name)
                self.assertEqual(bundle["spell"]["name"], donor_name)
                self.assertEqual(binding["policy"], "S21")
                self.assertFalse(receipt["raw_source_parity"])

    def test_familiar_alias_rejects_ambiguous_selection_and_target(self):
        target = entry("Druid familiar")
        bundle, _, _ = source(target)
        bundle["spell"]["name"] = "Summon Druid Familiar"
        selection = copy.deepcopy(SELECTION)
        selection["selections"].append(copy.deepcopy(selection["selections"][0]))
        with self.assertRaisesRegex(ValueError, "NATIVE_FAMILIAR_SELECTION_AMBIGUOUS"):
            select_familiar_candidate(bundle, CATALOG["bundles"], selection)
        with self.assertRaisesRegex(ValueError, "NATIVE_FAMILIAR_TARGET_AMBIGUOUS"):
            select_familiar_candidate(bundle, CATALOG["bundles"] + [target], SELECTION)

    def test_familiar_alias_requires_exact_policy_name_id_words_and_caster(self):
        target = entry("Druid familiar")
        for mutation in ("name", "reference_spell_id", "words", "vocations", "controller_vocation", "policy", "donor"):
            with self.subTest(mutation=mutation):
                bundle, deps, catalog = source(target)
                bundle["spell"]["name"] = "Summon Druid Familiar"
                selection = copy.deepcopy(SELECTION)
                if mutation == "name":
                    bundle["spell"]["name"] = "Summon druid familiar"
                elif mutation == "reference_spell_id":
                    bundle["spell"][mutation] = 194
                elif mutation == "words":
                    bundle["spell"][mutation] = "utevo gran res eq"
                elif mutation == "vocations":
                    bundle["spell"]["requirements"][mutation] = ["knight", "elite_knight"]
                elif mutation == "controller_vocation":
                    bundle["spell"]["execution"]["native_behavior"]["parameters"]["vocation"] = "knight"
                elif mutation == "policy":
                    selection["selections"][0]["selected"] = entry("Knight familiar")["bundle"]["spell"]["identity"]
                else:
                    bundle["spell"]["identity"]["key"] = "candidate:spell/source/crystal-main-current/native-test"
                with self.assertRaisesRegex(ValueError, "NATIVE_FAMILIAR_"):
                    project_native_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog, selection)

    def test_s21_alias_does_not_substitute_controller_or_accept_extra_null_metadata(self):
        target = entry("Druid familiar")
        for mutation in ("controller", "extra_controller", "null_metadata", "null_requirements"):
            with self.subTest(mutation=mutation):
                bundle, deps, catalog = source(target)
                bundle["spell"]["name"] = "Summon Druid Familiar"
                if mutation == "controller":
                    bundle["spell"]["execution"]["native_behavior"]["parameters"]["register_party_protection"] = True
                    reason = "SOURCE_CONFLICT_KEPT:"
                elif mutation == "extra_controller":
                    bundle["spell"]["execution"]["native_behavior"]["parameters"]["dynamic_inputs"] = {"look": "player.familiar_look_type"}
                    reason = "NATIVE_CONTROLLER_ADAPTER_MISSING"
                elif mutation == "null_metadata":
                    bundle["spell"]["requirements"]["vocation_display_flags"] = None
                    reason = "NATIVE_DISPLAY_METADATA_INVALID"
                else:
                    bundle["spell"]["requirements"] = None
                    reason = "NATIVE_FAMILIAR_ALIAS_VOCATION_MISMATCH"
                with self.assertRaisesRegex(ValueError, reason):
                    project_native_candidate(bundle, deps, target, target["bundle"]["spell"]["identity"], catalog, SELECTION)

    def test_numeric_item_descriptors_complete_catalog_without_replacing_barrier_controller(self):
        target = entry("Magic Wall Rune", "rune")
        bundle, deps, catalog = source(target)
        original = copy.deepcopy(bundle)
        catalog["definitions"] = [row for row in catalog["definitions"] if row["key"].endswith("/3180")]
        bundle["spell"]["execution"] = {"native_behavior": {"key": "tile_item_operation", "parameters": {
            "source_model": "r62/magic_wall", "item_variants": {"ITEM_MAGICWALL": 2128, "ITEM_MAGICWALL_SAFE": 10181},
            "expert_pvp_context_from_caster": True}}}
        raw_controller = copy.deepcopy(bundle["spell"]["execution"])
        completed, receipts = source_reference_catalog(bundle, catalog, target)
        self.assertEqual({row["key"].rsplit("/", 1)[-1] for row in completed["definitions"]}, {"3180", "2128", "10181"})
        self.assertEqual([row["source_value"] for row in receipts], [2128, 10181])
        self.assertEqual(bundle["spell"]["execution"], raw_controller)
        self.assertEqual(len(catalog["definitions"]), 1)
        self.assertEqual(len(original["spell"]["execution"]), 1)
        with self.assertRaisesRegex(ValueError, "NATIVE_CONTROLLER_ADAPTER_MISSING"):
            project(bundle, deps, target, catalog)

    def test_numeric_item_descriptors_cannot_invent_item_ids_or_hide_missing_values(self):
        target = entry("Magic Wall Rune", "rune")
        for mutation in ("unknown_number", "boolean", "null", "extra_enum", "missing_enum"):
            with self.subTest(mutation=mutation):
                bundle, _, catalog = source(target)
                catalog["definitions"] = [row for row in catalog["definitions"] if row["key"].endswith("/3180")]
                variants = {"ITEM_MAGICWALL": 2128, "ITEM_MAGICWALL_SAFE": 10181}
                if mutation == "unknown_number":
                    variants["ITEM_MAGICWALL"] = 99999
                elif mutation == "boolean":
                    variants["ITEM_MAGICWALL"] = True
                elif mutation == "null":
                    variants["ITEM_MAGICWALL"] = None
                elif mutation == "extra_enum":
                    variants["UNRELATED_ITEM"] = 2128
                else:
                    del variants["ITEM_MAGICWALL_SAFE"]
                bundle["spell"]["execution"] = {"native_behavior": {"key": "tile_item_operation", "parameters": {
                    "source_model": "r62/magic_wall", "item_variants": variants}}}
                with self.assertRaisesRegex(ValueError, "NATIVE_(REFERENCE_UNBOUND_OR_DUPLICATE|NUMERIC_ITEM_DESCRIPTOR_INVALID)"):
                    source_reference_catalog(bundle, catalog, target)

    def test_numeric_exclusion_catalog_is_proven_but_removal_controller_remains_held(self):
        target = entry("desintegrate rune", "rune")
        bundle, deps, catalog = source(target)
        catalog["definitions"] = [row for row in catalog["definitions"] if row["key"].endswith("/3197")]
        excluded = [4240, 4241, 4242, 4243, 4246, 4247, 4248]
        bundle["spell"]["execution"] = {"native_behavior": {"key": "tile_item_operation", "parameters": {
            "source_model": "r62/desintegrate_rune", "removal": {"excluded_item_ids": excluded},
            "final_cancel": "RETURNVALUE_NOTPOSSIBLE"}}}
        completed, receipts = source_reference_catalog(bundle, catalog, target)
        self.assertEqual(len(completed["definitions"]), 8)
        self.assertEqual([row["source_value"] for row in receipts], excluded)
        with self.assertRaisesRegex(ValueError, "NATIVE_CONTROLLER_ADAPTER_MISSING"):
            project(bundle, deps, target, catalog)
        excluded.append(4240)
        with self.assertRaisesRegex(ValueError, "NATIVE_NUMERIC_ITEM_DESCRIPTOR_INVALID"):
            source_reference_catalog(bundle, catalog, target)

    def test_house_maps_display_metadata_and_proven_missing_defaults(self):
        target = entry("House Guest List")
        bundle, deps, catalog = source(target)
        spell = bundle["spell"]
        spell["requirements"]["vocation_display_flags"] = [
            {"vocation": vocation, "show_in_description": True}
            for vocation in spell["requirements"]["vocations"]]
        del spell["targeting"]["allow_on_self"]
        del spell["targeting"]["check_floor"]
        result, dependencies, receipt = project(bundle, deps, target, catalog)
        self.assertEqual(result, target["bundle"])
        self.assertEqual(dependencies, target["dependencies"])
        self.assertEqual({row["field"] for row in receipt["field_provenance"]},
                         {"/targeting/allow_on_self", "/targeting/check_floor"})
        self.assertFalse(receipt["raw_source_parity"])
        self.assertFalse(receipt["runtime_activation"])

    def test_actual_premium_difference_retains_official_field_provenance(self):
        target = entry("Haste")
        bundle, deps, catalog = source(target)
        self.assertFalse(target["bundle"]["spell"]["requirements"]["premium"])
        bundle["spell"]["requirements"]["premium"] = True
        _, _, receipt = project(bundle, deps, target, catalog)
        proof = receipt["field_provenance"][0]
        self.assertEqual(proof["field"], "/requirements/premium")
        self.assertEqual(proof["kind"], "accepted_official_wiki_override")
        self.assertTrue(proof["source"])
        self.assertFalse(proof["accepted"])
        self.assertTrue(all(target["manifest"]["entries"][index]["resolution"].startswith("S15:")
                            for index in proof["manifest_indices"]))

    def test_source_only_cost_conflict_is_not_silently_overridden(self):
        target = entry("House Guest List")
        bundle, deps, catalog = source(target)
        bundle["spell"]["costs"]["mana"] = 1
        with self.assertRaisesRegex(ValueError, "SOURCE_CONFLICT_KEPT:/costs/mana"):
            project(bundle, deps, target, catalog)

    def test_official_precedence_cannot_hide_invalid_source_field_types(self):
        target = entry("Haste")
        bundle, deps, catalog = source(target)
        bundle["spell"]["requirements"]["premium"] = 1
        with self.assertRaisesRegex(ValueError, "NATIVE_HEADER_TYPE_MISMATCH:/requirements/premium"):
            project(bundle, deps, target, catalog)

    def test_food_preserves_existing_item_ids_and_draw_order(self):
        target = entry("Food")
        bundle, deps, catalog = source(target)
        result, _, receipt = project(bundle, deps, target, catalog)
        self.assertEqual(result, target["bundle"])
        self.assertTrue(receipt["normalized_controller_equality"])
        pool = bundle["spell"]["execution"]["native_behavior"]["parameters"]["pool"]
        pool[0], pool[1] = pool[1], pool[0]
        with self.assertRaisesRegex(ValueError, "NATIVE_CONTROLLER_ADAPTER_MISSING"):
            project(bundle, deps, target, catalog)

    def test_extra_or_undeclared_items_are_held(self):
        target = entry("Food")
        for mutation in ("extra", "undeclared"):
            bundle, deps, catalog = source(target)
            if mutation == "extra":
                catalog["definitions"].append({"family": "Item", "key": "candidate:item/source/canary-main-current/99999",
                                               "revision": "source-player-test"})
            else:
                bundle["spell"]["execution"]["native_behavior"]["parameters"]["pool"][0]["revision"] = "unlisted"
            with self.assertRaisesRegex(ValueError, "NATIVE_REFERENCE_"):
                project(bundle, deps, target, catalog)

    def test_explicit_null_reference_revision_is_rejected(self):
        target = entry("Food")
        bundle, deps, catalog = source(target)
        catalog["definitions"][0]["revision"] = None
        with self.assertRaisesRegex(ValueError, "NATIVE_REFERENCE_SOURCE_UNSUPPORTED"):
            project(bundle, deps, target, catalog)

    def test_duplicate_numeric_item_under_another_revision_cannot_hide_missing_catalog_item(self):
        target = entry("Food")
        bundle, _, catalog = source(target)
        catalog["definitions"][1] = copy.deepcopy(catalog["definitions"][0])
        catalog["definitions"][1]["revision"] = "different-source-revision"
        with self.assertRaisesRegex(ValueError, "NATIVE_REFERENCE_UNBOUND_OR_DUPLICATE"):
            source_reference_catalog(bundle, catalog, target)

    def test_changed_numeric_controller_parameter_is_a_source_conflict(self):
        target = entry("Swift Foot")
        bundle, deps, catalog = source(target)
        bundle["spell"]["execution"]["native_behavior"]["parameters"]["damage_dealt_percent"]["regular"] += 1
        with self.assertRaisesRegex(ValueError, "SOURCE_CONFLICT_KEPT:/execution/native_behavior/parameters/damage_dealt_percent/regular"):
            project(bundle, deps, target, catalog)

    def test_changed_current_profile_and_extra_source_controller_are_held(self):
        target = entry("House Guest List")
        bundle, deps, catalog = source(target)
        altered = copy.deepcopy(target)
        altered["bundle"]["spell"]["costs"]["mana"] = 1
        with self.assertRaisesRegex(ValueError, "CURRENT_NATIVE_PROFILE_UNQUALIFIED"):
            project(bundle, deps, altered, catalog)
        bundle["spell"]["execution"]["native_behavior"]["parameters"]["unexpected_world_write"] = True
        with self.assertRaisesRegex(ValueError, "NATIVE_CONTROLLER_ADAPTER_MISSING"):
            project(bundle, deps, target, catalog)

    def test_unmapped_source_state_and_unknown_display_flags_are_held(self):
        target = entry("House Guest List")
        bundle, deps, catalog = source(target)
        bundle["spell"]["source_state_contract"] = {"unrepresented_augment": True}
        with self.assertRaisesRegex(ValueError, "NATIVE_HEADER_FIELD_UNSUPPORTED"):
            project(bundle, deps, target, catalog)
        del bundle["spell"]["source_state_contract"]
        bundle["spell"]["requirements"]["vocation_display_flags"] = [
            {"vocation": "unknown", "show_in_description": True}]
        with self.assertRaisesRegex(ValueError, "NATIVE_DISPLAY_METADATA_INVALID"):
            project(bundle, deps, target, catalog)

    def test_explicit_null_display_metadata_is_rejected(self):
        target = entry("House Guest List")
        bundle, deps, catalog = source(target)
        bundle["spell"]["requirements"]["vocation_display_flags"] = None
        with self.assertRaisesRegex(ValueError, "NATIVE_DISPLAY_METADATA_INVALID"):
            project(bundle, deps, target, catalog)

    def test_explicit_null_rune_reference_metadata_is_rejected(self):
        target = entry("House Guest List")
        bundle, deps, catalog = source(target)
        bundle["spell"]["reference_rune_item_id"] = None
        with self.assertRaisesRegex(ValueError, "NATIVE_RUNE_ITEM_PROOF_MISMATCH"):
            project(bundle, deps, target, catalog)


if __name__ == "__main__":
    unittest.main()
