import copy
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


if __name__ == "__main__":
    unittest.main()
