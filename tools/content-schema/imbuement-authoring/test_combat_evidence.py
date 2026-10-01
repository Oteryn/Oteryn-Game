import copy
import hashlib
from fractions import Fraction
import json
import unittest

import combat_evidence as combat


class CombatEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads(combat.PACKET.read_bytes())

    def mutation_rejected(self, edit):
        packet = copy.deepcopy(self.packet)
        edit(packet)
        with self.assertRaises(ValueError):
            combat.validate(packet)

    def test_complete_sequence_and_evidence_graph(self):
        combat.validate(self.packet)
        rules = {r["id"]: r for r in self.packet["rules"]}
        sequence = rules["vibrancy_sequence"]["value"]
        self.assertEqual(sequence["recovery_sources"], ["monster", "player"])
        self.assertIsNone(sequence["deflect_additional_pvp_paralysis"])

    def test_pvp_success_gate_cannot_be_promoted_from_shorter_excerpts(self):
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "vibrancy_sequence")["value"].update(deflect_additional_pvp_paralysis=True))

    def test_initial_immunity_and_historical_reflection_are_rejected(self):
        def sequence(packet):
            return next(r for r in packet["rules"] if r["id"] == "vibrancy_sequence")["value"]
        self.mutation_rejected(lambda p: sequence(p).update(initial_paralysis_intercepted=True))
        self.mutation_rejected(lambda p: sequence(p).update(reflect_to_attacker=True))
        self.mutation_rejected(lambda p: sequence(p).update(trigger="initial_paralysis_attack"))

    def test_unknown_rounding_cannot_silently_select_calculator_ceiling(self):
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "leech_rounding").update(value="ceil"))

    def test_changing_status_cannot_admit_an_unresolved_value(self):
        for rule in self.packet["rules"]:
            if rule["status"] == "PUBLIC_EVIDENCE_UNRESOLVED":
                with self.subTest(rule=rule["id"]):
                    self.mutation_rejected(lambda p, name=rule["id"]: next(
                        r for r in p["rules"] if r["id"] == name
                    ).update(status="GLOBAL_VERIFIED", value="invented"))

    def test_missing_or_invented_rule_cannot_shrink_audit_scope(self):
        self.mutation_rejected(lambda p: p["rules"].pop())
        self.mutation_rejected(lambda p: p["rules"][-1].update(id="untracked_rule"))

    def test_selected_sequence_and_calculator_results_cannot_drift(self):
        self.mutation_rejected(lambda p: p["rules"][0]["value"]["recovery_sources"].append("arbitrary"))
        self.mutation_rejected(lambda p: p["rules"][1]["value"]["examples"][0].update(unrounded_amount=999999))

    def test_equal_hit_examples_cannot_become_full_combat_proof(self):
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "leech_equal_damage_aoe_scaling")["value"].update(scope="ALL_TARGET_DAMAGE"))
        examples = next(r for r in self.packet["rules"]
            if r["id"] == "leech_equal_damage_aoe_scaling")["value"]["examples"]
        self.assertEqual([r["unrounded_amount"] for r in examples], [250, 375])

    def test_test_server_news_is_not_live_release_evidence(self):
        self.mutation_rejected(lambda p: p["excluded_test_server_changes"][0].update(admission="LIVE"))

    def test_missing_source_and_runtime_activation_fail_closed(self):
        self.mutation_rejected(lambda p: p["rules"][0].update(evidence=["invented_primary"]))
        self.mutation_rejected(lambda p: p.update(activation="READY"))

    def test_live_release_removal_does_not_certify_current_reflection(self):
        def release(packet):
            return next(r for r in packet["rules"] if r["id"] == "vibrancy_reflection_removed_at_release")
        self.mutation_rejected(lambda p: release(p)["value"].update(reflect_to_attacker=True))
        self.mutation_rejected(lambda p: release(p)["value"].update(reflect_to_attacker=0))
        self.mutation_rejected(lambda p: release(p)["value"].update(scope="CURRENT_GLOBAL"))
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "vibrancy_reflection_current").update(value=False))
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "official_vibrancy_release_4828").update(source_stage="TEASER"))

    def test_mana_ceil_each_target_is_distinct_from_ceil_after_sum(self):
        self.assertEqual(combat.mana_reference_example([101], 800), 9)
        self.assertEqual(combat.mana_reference_example([100, 900], 800), 45)
        self.assertNotEqual(combat.mana_reference_example([100, 900], 800), 44)
        for damages in ([], [0], [-1], [True]):
            with self.assertRaises(ValueError):
                combat.mana_reference_example(damages, 800)

    def test_mana_specific_values_cannot_become_life_or_generic_rules(self):
        changes = {"scope": "ALL_LEECH", "rounding": "CEIL_AFTER_SUM",
                   "critical_damage_included": False, "damage_prey_bonus_included": True,
                   "overkill_damage_counts": False, "zero_damage_target_count": "INCLUDED"}
        for key, value in changes.items():
            with self.subTest(key=key):
                self.mutation_rejected(lambda p, k=key, v=value: next(r for r in p["rules"]
                    if r["id"] == "mana_leech_current_reference_formula")["value"].update({k: v}))

    def test_post_target_community_formula_cannot_claim_july_global_certification(self):
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "mana_leech_current_reference_formula").update(target_time_status="TARGET_CERTIFIED"))
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "fandom_formulae_1205374").update(role="PRIMARY_OFFICIAL"))
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "fandom_formulae_1205374").update(revision=1197205))
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "fandom_formulae_1205374").update(revision_timestamp="2026-07-21T13:24:22Z"))

    def test_pair_observations_cannot_grant_arbitrary_equipment_composition(self):
        def pairs(packet):
            return next(r for r in packet["rules"] if r["id"] == "leech_two_powerful_equipment_pairs")["value"]
        self.mutation_rejected(lambda p: pairs(p).update(scope="ALL_EQUIPMENT"))
        self.mutation_rejected(lambda p: pairs(p).update(other_equipment_composition="ADDITIVE"))
        self.mutation_rejected(lambda p: pairs(p)["observed_pairs"][0].update(combined_share_bps=2000))
        self.mutation_rejected(lambda p: pairs(p)["observed_pairs"][1].update(tier="basic"))

    def test_source_identity_and_selected_claim_digest_are_checked(self):
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "fandom_formulae_1205374")["selected_claims"].update(scope="ALL_LEECH"))
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "tibiaqa_two_mana_2018").update(sha256="0" * 64))
        self.mutation_rejected(lambda p: p["engine_combat_hypotheses"][0].update(status="GLOBAL_VERIFIED"))

    def test_all_rules_preserve_target_time_qualification(self):
        for rule in self.packet["rules"]:
            with self.subTest(rule=rule["id"]):
                self.mutation_rejected(lambda p, name=rule["id"]: next(r for r in p["rules"]
                    if r["id"] == name).update(target_time_status="GLOBAL_TARGET_CERTIFIED"))

    def test_reported_life_healing_distinguishes_nearest_and_aggregate_rounding(self):
        for targets, damage, reported_each in combat.LIFE_CASES:
            derived = combat.life_reported_example(damage, targets)
            self.assertEqual(derived["ceil_each"], reported_each)
            self.assertEqual(derived["total_if_ceil_each"], targets * reported_each)
            if targets in (2, 3):
                self.assertNotEqual(derived["nearest_each"], reported_each)
            if targets > 1:
                self.assertNotEqual(derived["ceil_aggregated_once"], targets * reported_each)
        for damage, count in ((900, 2), (438, 2), (387, 3), (0, 1), (438, True)):
            with self.assertRaises(ValueError):
                combat.life_reported_example(damage, count)

    def test_reported_life_cases_cannot_change_healing_or_select_a_generic_algorithm(self):
        def life(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "life_leech_reported_equal_hit_ceiling")["value"]
        for key, value in {
            "scope": "ALL_CURRENT_LIFE_LEECH", "interpretation": "UNIVERSAL_CEILING",
            "universal_rounding": "ceil", "unequal_target_rounding": "ceil",
            "current_target_continuity": True, "cap_bps": 10000,
            "overkill_behavior": "INCLUDED",
        }.items():
            with self.subTest(key=key):
                self.mutation_rejected(lambda p, k=key, v=value: life(p).update({k: v}))
        self.mutation_rejected(lambda p: life(p)["reported_examples"][1].update(
            healed_each_target=53, total_healed=106))
        self.mutation_rejected(lambda p: life(p)["reported_examples"][2].update(total_healed=115))
        self.mutation_rejected(lambda p: life(p)["reported_examples"].append(
            {"hit_targets": 2, "damage_per_target": 900, "healed_each_target": 124,
             "total_healed": 248}))
        self.mutation_rejected(lambda p: life(p)["setup"].update(wand_equipped=0))

    def test_wheel_example_cannot_grant_other_combinations_life_or_caps(self):
        def wheel(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "mana_leech_wheel_equipment_example")["value"]
        self.assertEqual(wheel(self.packet)["combined_share_bps"], 850)
        for key, value in {
            "combined_share_bps": 1600, "wheel_share_bps": 800, "chance_bps": 100,
            "family": "Vampirism", "other_combinations": "ALL_ADDITIVE",
            "life_leech_composition": "ADDITIVE", "cap_bps": 10000,
            "current_target_continuity": True,
        }.items():
            with self.subTest(key=key):
                self.mutation_rejected(lambda p, k=key, v=value: wheel(p).update({k: v}))

    def test_named_charm_report_cannot_exclude_every_charm_or_claim_current_observation(self):
        def charm(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "leech_elemental_parry_wound_reported_exclusions")["value"]
        self.mutation_rejected(lambda p: charm(p).update(scope="ALL_CURRENT_CHARMS"))
        self.mutation_rejected(lambda p: charm(p).update(other_charms={"low_blow": False}))
        self.mutation_rejected(lambda p: charm(p).update(current_target_continuity=True))
        self.mutation_rejected(lambda p: charm(p).update(gameplay_calendar_date="2026-10-01"))
        self.mutation_rejected(lambda p: charm(p)["reported_to_trigger_imbuement_leech"].update(parry=0))

    def test_captured_report_dates_raw_bytes_and_revision_history_are_pinned(self):
        for source_id in combat.BOUNDED_SOURCE_RECORD_SHA256:
            with self.subTest(source=source_id):
                def source(packet):
                    return next(s for s in packet["sources"] if s["id"] == source_id)
                self.mutation_rejected(lambda p: source(p).update(sha256="0" * 64))
                self.mutation_rejected(lambda p: source(p).update(published_on="2026-10-01"))
                self.mutation_rejected(lambda p: source(p).update(role="PRIMARY_OFFICIAL"))
                self.mutation_rejected(lambda p: source(p).update(target_time_status="TARGET_CERTIFIED"))
        self.mutation_rejected(lambda p: next(s for s in p["sources"]
            if s["id"] == "tibiaqa_life_equal_hit_2020").update(report_body_edited_on="2022-06-21"))
        self.mutation_rejected(lambda p: p["sources"].remove(next(s for s in p["sources"]
            if s["id"] == "tibiaqa_life_answer_revisions_14497")))

    def test_source_claims_cannot_be_forged_by_rehashing_the_selected_claims(self):
        def forge(packet):
            source = next(s for s in packet["sources"] if s["id"] == "tibiaqa_life_equal_hit_2020")
            source["selected_claims"]["reported_cases"][2].update(healed_each_target=38)
            source["selected_claims_sha256"] = hashlib.sha256(json.dumps(
                source["selected_claims"], sort_keys=True, ensure_ascii=False,
                separators=(",", ":")).encode()).hexdigest()
        self.mutation_rejected(forge)

    def test_bounded_profiles_do_not_promote_or_disconnect_generic_unknowns(self):
        for name in combat.BOUNDED_LINKS:
            rule = next(r for r in self.packet["rules"] if r["id"] == name)
            self.assertIsNone(rule["value"])
            with self.subTest(rule=name):
                self.mutation_rejected(lambda p: next(r for r in p["rules"]
                    if r["id"] == name).update(bounded_profiles=[]))
                self.mutation_rejected(lambda p: next(r for r in p["rules"]
                    if r["id"] == name).update(value="UNIVERSAL_RULE"))

    def test_reported_matching_ammo_split_converts_physical_half_not_whole_attack(self):
        ranged = next(r for r in self.packet["rules"]
            if r["id"] == "ranged_elemental_ammo_reported_cases")["value"]
        flash, shiver = ranged["reported_cases"]
        self.assertEqual(flash["reported_damage_split_bps"],
                         {"physical": 5000, "energy": 5000, "ice": 0})
        physical = shiver["base_physical_attack"]
        native_ice = shiver["base_elemental_attack"]
        total = physical + native_ice
        converted = physical * Fraction(ranged["conversion_bps"], 10000)
        # Interpret the approximate reported14/14 case, not a damage pipeline.
        self.assertEqual((physical - converted) / total * 10000, 4500)
        self.assertEqual((native_ice + converted) / total * 10000, 5500)
        self.assertEqual(shiver["reported_damage_split_bps"], {"physical": 4500, "ice": 5500})
        self.assertNotEqual((physical - total * Fraction(1, 10)) / total * 10000, 4500)

    def test_reported_ranged_cases_reject_wrong_element_or_overwritten_native_component(self):
        def ranged(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "ranged_elemental_ammo_reported_cases")["value"]
        self.mutation_rejected(lambda p: ranged(p)["reported_cases"][0].update(
            reported_damage_split_bps={"physical": 5000, "energy": 0, "ice": 5000}))
        self.mutation_rejected(lambda p: ranged(p)["reported_cases"][1].update(
            reported_damage_split_bps={"physical": 4000, "ice": 6000}))
        self.mutation_rejected(lambda p: ranged(p)["reported_cases"][1].update(
            reported_damage_split_bps={"physical": 9000, "ice": 1000}))
        self.mutation_rejected(lambda p: ranged(p)["reported_cases"][0].update(ammo="Diamond Arrow"))
        self.mutation_rejected(lambda p: ranged(p).update(conversion_bps=5000))

    def test_historical_ranged_report_cannot_select_general_order_caps_or_trial_breakdown(self):
        def ranged(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "ranged_elemental_ammo_reported_cases")["value"]
        for key, value in {
            "scope": "ALL_RANGED_AND_MELEE", "current_target_continuity": True,
            "universal_conversion_order": "BEFORE_ARMOR", "critical_order": "AFTER_CONVERSION",
            "armor_order": "BEFORE_CONVERSION", "integer_rounding": "floor",
            "damage_cap": 1000, "universal_damage_conservation": True,
            "reported_arrows_total": 400, "arrows_per_ammo": 200,
            "interpretation": "EXACT_CURRENT_GLOBAL_SPLITS",
        }.items():
            with self.subTest(key=key):
                self.mutation_rejected(lambda p, k=key, v=value: ranged(p).update({k: v}))
        for rule in self.packet["rules"]:
            if rule["status"] == "PUBLIC_EVIDENCE_UNRESOLVED":
                self.assertIsNone(rule["value"])

    def test_ranged_author_dates_comment_trial_count_and_claims_cannot_be_self_certified(self):
        def source(packet):
            return next(s for s in packet["sources"]
                if s["id"] == "tibiaqa_basic_frost_elemental_ammo_2021")
        self.mutation_rejected(lambda p: source(p).update(trial_count_comment_at="2026-10-01T12:00:00Z"))
        self.mutation_rejected(lambda p: source(p)["selected_claims"].update(author="CipSoft"))
        def forge(packet):
            captured = source(packet)
            captured["selected_claims"]["reported_arrows_total"] = 400
            captured["selected_claims_sha256"] = hashlib.sha256(json.dumps(
                captured["selected_claims"], sort_keys=True, ensure_ascii=False,
                separators=(",", ":")).encode()).hexdigest()
        self.mutation_rejected(forge)

    def test_native_ammo_reference_conflict_cannot_be_removed_or_select_a_current_rule(self):
        def conflict(packet):
            return next(c for c in packet["source_conflicts"]
                if c.get("id") == "ranged_native_ammunition_reference_conflict")
        self.assertIsNone(conflict(self.packet)["current_native_ammo_rule"])
        self.assertIsNone(conflict(self.packet)["current_timer_rule"])
        self.mutation_rejected(lambda p: p["source_conflicts"].remove(conflict(p)))
        self.mutation_rejected(lambda p: p["source_conflicts"].append(copy.deepcopy(conflict(p))))
        for key, value in {
            "status": "CURRENT_GLOBAL_VERIFIED", "resolution": "HISTORICAL_REPORT_OVERRIDES_REFERENCE",
            "current_native_ammo_rule": False, "current_timer_rule": True,
            "conflicting_historical_case": "NO_CONFLICT",
            "sources": ["tibiaqa_basic_frost_elemental_ammo_2021"],
        }.items():
            with self.subTest(key=key):
                self.mutation_rejected(lambda p, k=key, v=value: conflict(p).update({k: v}))
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "ranged_elemental_ammo_reported_cases").update(
                evidence=["tibiaqa_basic_frost_elemental_ammo_2021"]))

    def test_current_fandom_native_ammo_claim_retains_complete_capture_and_confidence(self):
        def source(packet):
            return next(s for s in packet["sources"] if s["id"] == "fandom_imbuing_full")
        self.mutation_rejected(lambda p: source(p).update(revision=1197205))
        self.mutation_rejected(lambda p: source(p).update(revision_access="READ_ARCHIVE"))
        self.mutation_rejected(lambda p: source(p).update(role="PRIMARY_OFFICIAL"))
        def forge(packet):
            captured = source(packet)
            captured["selected_claims"]["verbatim"] = "Only different-element ammo is excluded."
            captured["selected_claims_sha256"] = hashlib.sha256(json.dumps(
                captured["selected_claims"], sort_keys=True, ensure_ascii=False,
                separators=(",", ":")).encode()).hexdigest()
        self.mutation_rejected(forge)


    def test_archived_life_prey_claim_does_not_select_a_generic_leech_pipeline(self):
        def value(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "life_leech_damage_prey_exclusion")["value"]
        selected = value(self.packet)
        self.assertEqual(selected["scope"], "LIFE_LEECH_REFERENCE_ONLY")
        self.assertIs(selected["damage_prey_bonus_included"], False)
        for key in ("all_damage_modifier_order", "overkill_basis", "rounding",
                    "current_target_continuity"):
            self.assertIsNone(selected[key])
            self.mutation_rejected(lambda p, k=key: value(p).update({k: "invented"}))
        for change in ({"scope": "ALL_LEECH"}, {"scope": "MANA_LEECH_ONLY"},
                       {"damage_prey_bonus_included": True},
                       {"damage_prey_bonus_included": 0}):
            self.mutation_rejected(lambda p, c=change: value(p).update(c))

    def test_archive_snapshot_cannot_become_a_revision_or_target_observation_date(self):
        for source_id in ("fandom_life_archive_2026", "fandom_vibrancy_archive_2025"):
            def source(packet, name=source_id):
                return next(s for s in packet["sources"] if s["id"] == name)
            captured = source(self.packet)
            self.assertIsNone(captured["published_on"])
            self.assertIsNone(captured["revision_timestamp"])
            self.assertEqual(captured["access_status"], "FULL_PUBLIC_ARCHIVED_COMMUNITY_HTML")
            for change in ({"published_on": captured["archive_snapshot_at"][:10]},
                           {"revision_timestamp": captured["archive_snapshot_at"]},
                           {"archive_snapshot_at": "2026-10-01T10:00:00Z"},
                           {"revision": 1197205}, {"sha256": "0" * 64},
                           {"role": "PRIMARY_OFFICIAL"},
                           {"digest_scope": "EXACT_TARGET_GAMEPLAY_LOG"}):
                with self.subTest(source=source_id, change=change):
                    self.mutation_rejected(lambda p, c=change: source(p).update(c))

    def test_archive_initial_success_qualification_cannot_be_forged_or_disconnected(self):
        def source(packet):
            return next(s for s in packet["sources"]
                if s["id"] == "fandom_vibrancy_archive_2025")
        self.assertIsNone(source(self.packet)["selected_claims"]["success_state_lifetime"])
        self.assertIsNone(source(self.packet)["selected_claims"]["success_state_reset"])
        def forge(packet):
            selected_source = source(packet)
            selected_source["selected_claims"].update(
                qualification="UNCONDITIONAL", success_state_lifetime="UNTIL_UNEQUIPPED")
            selected_source["selected_claims_sha256"] = hashlib.sha256(json.dumps(
                selected_source["selected_claims"], sort_keys=True, ensure_ascii=False,
                separators=(",", ":")).encode()).hexdigest()
        self.mutation_rejected(forge)
        for rule_id in ("vibrancy_sequence", "vibrancy_pvp_gate"):
            def remove_archive(packet, name=rule_id):
                next(r for r in packet["rules"] if r["id"] == name)["evidence"].remove(
                    "fandom_vibrancy_archive_2025")
            self.mutation_rejected(remove_archive)

    def test_new_life_reference_preserves_all_seven_unresolved_profiles(self):
        self.assertEqual(len(self.packet["rules"]), 18)
        unresolved = {r["id"] for r in self.packet["rules"] if r["value"] is None}
        self.assertEqual(unresolved, {
            "leech_rounding", "leech_unequal_damage_and_overkill_order",
            "vibrancy_reflection_current", "critical_healing_scope",
            "leech_equipment_composition", "protection_equipment_composition",
            "vibrancy_pvp_gate"})
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "life_leech_damage_prey_exclusion").update(
                target_time_status="EXACT_TARGET_GLOBAL_OBSERVATION"))


    def test_current_vibrancy_full_text_cannot_be_removed_or_supply_a_gate_lifetime(self):
        for name in ("vibrancy_sequence", "vibrancy_pvp_gate"):
            self.mutation_rejected(lambda p, rid=name: next(r for r in p["rules"]
                if r["id"] == rid)["evidence"].remove("fandom_vibrancy_current_1194726"))
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "vibrancy_pvp_gate").update(
                value={"initial_success": "FIRST_RECOVERY", "lifetime": "UNTIL_UNEQUIPPED"}))
        source = next(s for s in self.packet["sources"]
            if s["id"] == "fandom_vibrancy_current_1194726")
        self.assertEqual(source["revision"], 1194726)
        self.assertIsNone(source["selected_claims"]["success_state_lifetime"])
        self.assertIsNone(source["selected_claims"]["success_state_reset"])

    def test_native_reference_engine_discrepancy_cannot_be_erased_or_resolved_as_global(self):
        def conflict(packet):
            return next(c for c in packet["source_conflicts"]
                if c.get("id") == "native_percentage_reduction_reference_vs_engine")
        self.assertIsNone(conflict(self.packet)["current_global_pipeline"])
        self.mutation_rejected(lambda p: p["source_conflicts"].remove(conflict(p)))
        self.mutation_rejected(lambda p: conflict(p).update(current_global_pipeline="ENGINE"))

    def test_current_evaluation_date_does_not_promote_historical_rounding_reports(self):
        self.assertEqual(self.packet["target"], "global-tibia-current-2026-10-01")
        self.mutation_rejected(lambda p: p.update(
            target="global-tibia-observable-2026-07-28-post-server-save"))
        mana = next(r for r in self.packet["rules"]
            if r["id"] == "mana_leech_current_reference_formula")
        self.assertEqual(mana["status"], "CURRENT_DATED_COMMUNITY_EXPLICIT_MANA_ONLY")
        self.assertEqual(mana["value"]["scope"], "MANA_LEECH_ONLY")
        self.assertEqual(mana["target_time_status"],
            "CURRENT_DATED_COMMUNITY_REVISION_NOT_GLOBAL_RUNTIME_OBSERVATION")
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "mana_leech_current_reference_formula").update(
                status="POST_TARGET_COMMUNITY_EXPLICIT_MANA_ONLY"))
        for rule_id in ("life_leech_reported_equal_hit_ceiling",
                        "ranged_elemental_ammo_reported_cases"):
            rule = next(r for r in self.packet["rules"] if r["id"] == rule_id)
            self.assertTrue(rule["target_time_status"].startswith("HISTORICAL_"))
            self.mutation_rejected(lambda p, name=rule_id: next(r for r in p["rules"]
                if r["id"] == name).update(target_time_status=
                    "CURRENT_DATED_COMMUNITY_REVISION_NOT_GLOBAL_RUNTIME_OBSERVATION"))

    def test_current_life_prey_corroboration_keeps_the_mana_formula_life_boundary(self):
        rule = next(r for r in self.packet["rules"]
            if r["id"] == "life_leech_damage_prey_exclusion")
        self.assertEqual(set(rule["evidence"]), {
            "fandom_life_archive_2026", "fandom_formulae_1205374",
            "fandom_life_current_1101811"})
        self.assertEqual(rule["status"], "CURRENT_COMMUNITY_EXPLICIT_LIFE_PREY_ONLY")
        self.assertIsNone(rule["value"]["rounding"])
        self.assertIsNone(rule["value"]["overkill_basis"])
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "life_leech_damage_prey_exclusion").update(
                evidence=["fandom_formulae_1205374"]))
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "leech_rounding").update(value="ceil"))

    def test_native_reference_pair_is_iterative_not_additive_or_a_generic_pipeline(self):
        def value(packet):
            return next(r for r in packet["rules"]
                if r["id"] == "native_equipment_percentage_reduction_example")["value"]
        selected = value(self.packet)
        damage = selected["original_damage"]
        for step in selected["percentage_steps"]:
            damage = damage * (10000 - step["reduction_bps"]) // 10000
            self.assertEqual(damage, step["damage_after_step"])
        self.assertEqual(damage, 178)
        # This particular final integer coincides with additive reduction;
        # the reference establishes its stages, not a discriminating hit log.
        self.assertNotEqual(Fraction(190 * (10000 - 600), 10000),
                            Fraction(200 * (10000 - 500 - 600), 10000))
        self.assertEqual([damage - 7, damage - 4],
            selected["reference_damage_after_armor_range"])
        for change in ({"scope": "ALL_EQUIPMENT_AND_IMBUEMENTS"},
                       {"reference_percentage_rounding": "ROUND_REMOVED_DAMAGE"},
                       {"reference_armor_stage": "BEFORE_PERCENTAGE_REDUCTION"},
                       {"imbuement_and_native_composition": "SUM"},
                       {"wheel_order": "LAST"},
                       {"current_global_runtime_applicability": True}):
            self.mutation_rejected(lambda p, c=change: value(p).update(c))
        self.mutation_rejected(lambda p: next(r for r in p["rules"]
            if r["id"] == "protection_equipment_composition").update(value="multiply"))

if __name__ == "__main__":
    unittest.main()
