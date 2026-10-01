import copy
import json
import unittest

import newbranch_evidence as branch


class CrystalBranchEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads(branch.PACKET.read_text())

    def test_complete_pinned_hypothesis(self):
        self.assertTrue(branch.validate(self.packet))
        self.assertEqual(self.packet["sources"]["data/XML/imbuements.xml"]["blob_sha"],
                         "dc712cbd666a41614dbb1d9fbb355138e59fc5a3")
        self.assertEqual(self.packet["sources"]["src/creatures/players/imbuements/imbuements.cpp"]["blob_sha"],
                         "0cdc2db68a667ffa06eac98868cf4d153da63f7a")

    def test_recipes_fees_and_scroll_gap(self):
        rows = self.packet["selected_catalogue_comparison"]["records"]
        self.assertTrue(all(r["materials_match"] and r["apply_fee_matches"]
                            and r["clear_fee_matches"] and r["duration_matches"] for r in rows))
        self.assertEqual([int(b["price"]) for b in self.packet["xml"]["bases"]],
                         [7500, 60000, 250000])
        self.assertEqual({r["tier"] for r in rows if not r["scroll_id_matches"]}, {1})

    def test_strike_uses_combined_baseline(self):
        rows = [r for r in self.packet["selected_catalogue_comparison"]["records"]
                if r["name"] == "Strike"]
        self.assertEqual([r["configured_normalized_effect"]["extra_damage_bps"] for r in rows],
                         [1500, 2500, 5000])
        self.assertEqual([r["configured_normalized_effect"]["chance_bps"] for r in rows],
                         [1000, 1000, 1000])

    def test_vibrancy_not_false_functionality(self):
        rows = self.packet["selected_catalogue_comparison"]["records"]
        failed = [r for r in rows if not r["loader_applies_effect"]]
        self.assertEqual([(r["name"], r["tier"]) for r in failed],
                         [("Vibrancy", 1), ("Vibrancy", 2), ("Vibrancy", 3)])
        self.assertTrue(all(r["strength_numbers_match"] for r in failed))
        self.assertFalse(any(r["effect_configuration_matches"] for r in failed))
        self.assertEqual([r["selected_effect"]["remove_chance_bps"] for r in failed],
                         [1500, 2500, 5000])
        for row in failed:
            self.assertEqual(row["selected_effect"]["kind"], "paralysis_recovery")
            self.assertEqual(row["selected_effect"]["trigger"],
                             "additional_paralysis_attack_while_paralysed")
            self.assertEqual(row["selected_effect"]["sequence_profile"],
                             "imbuement-combat.json#vibrancy_sequence")
            self.assertEqual(row["configured_normalized_effect"]["kind"],
                             "paralysis_deflection")
        self.assertEqual(self.packet["selected_catalogue_comparison"]["summary"]
                         ["strength_numbers_matching"], 72)
        self.assertEqual(self.packet["selected_catalogue_comparison"]["summary"]
                         ["effects_config_matching"], 69)

    def test_token_bundles_match_recipes_and_tier_prices(self):
        lookup = {(r["name"], r["tier"]): r["materials"] for r in self.packet["xml"]["records"]}
        levels = {"basic": 1, "intricate": 2, "powerful": 3}
        for bundle in self.packet["gold_token_bundles"]:
            level = levels[bundle["tier"]]
            self.assertEqual(bundle["gold_tokens"], level * 2)
            self.assertEqual(bundle["token_item_id"], 22721)
            self.assertEqual(bundle["materials"], lookup[bundle["name"], level])

    def test_missing_tier_and_false_authority_rejected(self):
        for edit in (lambda p: p["xml"]["records"].pop(),
                     lambda p: p.update(role="OFFICIAL_PUBLIC"),
                     lambda p: p["engine_facts"][0].update(confidence="GLOBAL_VERIFIED")):
            packet = copy.deepcopy(self.packet)
            edit(packet)
            with self.assertRaises(ValueError):
                branch.validate(packet)

    def test_parser_requires_explicit_effect_and_keeps_basic_zero(self):
        raw = b'<imbuements>\n<imbuement name="Strike" base="1" category="3" premium="0" storage="0" scrollid="0"><attribute key="effect" type="skill" value="critical" bonus="500" chance="500"/><attribute key="item" value="11444" count="20"/></imbuement>\n</imbuements>'
        parsed = branch.parse_xml(raw)
        self.assertEqual(parsed["records"][0]["scroll_item_id"], 0)
        self.assertEqual(parsed["records"][0]["source_line"], 2)
        with self.assertRaises(ValueError):
            branch.parse_xml(raw.replace(b'key="effect"', b'key="description"'))


if __name__ == "__main__":
    unittest.main()
