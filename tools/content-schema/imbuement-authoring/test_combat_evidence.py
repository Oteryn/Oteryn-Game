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


if __name__ == "__main__":
    unittest.main()
