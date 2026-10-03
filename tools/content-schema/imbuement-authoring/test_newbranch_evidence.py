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

    def test_strike_modifier_is_separate_from_combined_baseline(self):
        rows = [r for r in self.packet["selected_catalogue_comparison"]["records"]
                if r["name"] == "Strike"]
        self.assertEqual([r["configured_normalized_effect"]["extra_damage_bps"] for r in rows],
                         [500, 1500, 4000])
        self.assertEqual([r["configured_normalized_effect"]["chance_bps"] for r in rows],
                         [500, 500, 500])
        self.assertTrue(all(r["configured_normalized_effect"]["value_semantics"]
                            == "ADDITIVE_IMBUEMENT_MODIFIER" for r in rows))
        self.assertEqual([r["configured_effective_total_with_intrinsic_baseline"]
                          ["extra_damage_bps"] for r in rows], [1500, 2500, 5000])
        self.assertEqual([r["configured_effective_total_with_intrinsic_baseline"]
                          ["chance_bps"] for r in rows],
                         [1000, 1000, 1000])
        self.assertTrue(all(r["configured_effective_total_with_intrinsic_baseline"]
                            ["value_semantics"] == "ISOLATED_TOTAL_NOT_AN_IMBUEMENT_MODIFIER"
                            for r in rows))

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

    def mutation_rejected(self, edit):
        packet = copy.deepcopy(self.packet)
        edit(packet)
        with self.assertRaises(ValueError):
            branch.validate(packet)

    def test_comparison_requires_exact_unique_keys_and_independent_strengths(self):
        self.mutation_rejected(lambda p: p["selected_catalogue_comparison"].update(
            records=[p["selected_catalogue_comparison"]["records"][0]] * 72))
        self.mutation_rejected(lambda p: next(r for r in p["selected_catalogue_comparison"]["records"]
            if r["name"] == "Strike")["configured_normalized_effect"].update(extra_damage_bps=9999))
        self.mutation_rejected(lambda p: p["selected_catalogue_comparison"]["summary"].update(effects_loaded=72))

    def test_duplicate_bundles_and_changed_transactions_are_rejected(self):
        self.mutation_rejected(lambda p: p.update(gold_token_bundles=[p["gold_token_bundles"][0]] * 9))
        self.mutation_rejected(lambda p: p["gold_token_bundles"][0].update(gold_tokens=999))
        self.mutation_rejected(lambda p: p["gold_token_bundles"][0]["materials"][0].update(count=1))

    def test_capture_changes_cannot_preserve_old_source_attribution(self):
        self.mutation_rejected(lambda p: p["xml"]["records"][0]["effect"].update(bonus="9999"))
        self.mutation_rejected(lambda p: p["sources"]["data/XML/imbuements.xml"].update(blob_sha="0" * 40))
        self.mutation_rejected(lambda p: next(r for r in p["engine_facts"]
            if r["id"] == "strike_xml_delta_and_player_baseline")["value"].update(player_baseline_chance_bps=999))

    def test_effective_strike_totals_cannot_be_reintroduced_as_item_modifiers(self):
        def replace_modifier(packet):
            row = next(r for r in packet["selected_catalogue_comparison"]["records"]
                       if r["name"] == "Strike" and r["tier"] == 3)
            # Changing both copied comparison sides must not fool recomputation.
            for side in ("configured_normalized_effect", "selected_effect"):
                row[side].update(chance_bps=1000, extra_damage_bps=5000)
        self.mutation_rejected(replace_modifier)
        self.mutation_rejected(lambda p: p["critical_modifier_public_evidence"]
            ["strike_additive_modifiers"].update(extra_damage_bps_by_tier=[1500, 2500, 5000]))
        facts = self.packet["critical_modifier_public_evidence"]
        self.assertEqual(facts["intrinsic_character_baseline"]["value_semantics"],
                         "CHARACTER_BASELINE_APPLIED_ONCE")
        self.assertEqual(facts["sources"]["official_news_8421"]["role"], "OFFICIAL_PUBLIC_TEASER")

    def test_new_engine_paths_remain_separate_ots_hypotheses(self):
        facts = {r["id"]: r for r in self.packet["engine_facts"]}
        self.assertFalse(facts["quest_storage_check_configuration_default"]["value"])
        self.assertEqual(facts["leech_excluded_damage"]["value"],
                         {"extension": True, "origin_condition": True})
        self.assertEqual(facts["leech_chance_comparison"]["value"]["failure_comparison"], ">= chance")
        self.assertEqual(facts["healing_critical_separate_path"]["value"]["wheel_perk"], "Blessing of the Grove")
        self.assertIn("damaging", facts["critical_roll_scope"]["value"])
        self.assertEqual(facts["scroll_target_equipped_allowed"]["anchors"][-1]["line"], 2854)
        self.assertTrue(all(r["confidence"] == "OTS_SOURCE_CODE_ONLY" for r in facts.values()))

    def test_previous_scroll_parser_supports_root_child_and_explicit_absence(self):
        root = b'<imbuements>\n<imbuement name="Strike" base="1" scrollid="53769"/>\n</imbuements>'
        child = b'<imbuements>\n<imbuement name="Strike" base="2"><attribute key="scroll" value="51742"/></imbuement>\n</imbuements>'
        absent = b'<imbuements>\n<imbuement name="Vibrancy" base="2"/>\n</imbuements>'
        self.assertEqual(branch.parse_previous_scroll_bindings(root)[0], {
            "name": "Strike", "tier": 1, "scroll_item_id": 53769,
            "source_line": 2, "representation": "ROOT_SCROLLID"})
        self.assertEqual(branch.parse_previous_scroll_bindings(child)[0]["scroll_item_id"], 51742)
        self.assertEqual(branch.parse_previous_scroll_bindings(child)[0]["representation"], "CHILD_SCROLL_ATTRIBUTE")
        self.assertEqual(branch.parse_previous_scroll_bindings(absent)[0]["scroll_item_id"], 0)
        self.assertEqual(branch.parse_previous_scroll_bindings(absent)[0]["representation"], "ABSENT")
        with self.assertRaises(ValueError):
            branch.parse_previous_scroll_bindings(child.replace(b'base="2"', b'base="2" scrollid="51742"'))
        with self.assertRaises(ValueError):
            branch.parse_previous_scroll_bindings(child.replace(b'</imbuement>',
                b'<attribute key="scroll" value="51742"/></imbuement>'))

    def test_corrected_references_preserve_original_capture_and_restore_actual_summer_ids(self):
        import imbuement_authoring as authoring
        original = authoring.source_facts()
        self.assertTrue(all(not r["scroll_item_ids"] for r in original["crystal"]["records"]))
        corrected = branch.corrected_previous_references(self.packet)
        self.assertEqual(sum(bool(r["scroll_item_ids"]) for r in corrected["crystal"]), 72)
        self.assertEqual(sum(bool(r["scroll_item_ids"]) for r in corrected["canary"]), 46)
        self.assertEqual(next(r for r in corrected["crystal"]
            if r["name"] == "Strike" and r["tier"] == 1)["scroll_item_ids"], [53769])
        self.assertEqual(next(r for r in corrected["canary"]
            if r["name"] == "Vibrancy" and r["tier"] == 2)["scroll_item_ids"], [])
        self.assertEqual(original, authoring.source_facts())
        defects = self.packet["original_capture_defects"]
        self.assertEqual(defects[0]["affected_record_count"], 72)

    def test_previous_comparison_exposes_all_basic_scroll_differences(self):
        comparison = self.packet["previous_reference_comparison"]
        summer = comparison["crystal"]["record_differences"]
        self.assertEqual(len(summer), 24)
        self.assertEqual({r["field"] for r in summer}, {"scroll_item_id"})
        self.assertEqual({r["tier"] for r in summer}, {1})
        self.assertTrue(all(r["new_branch"] == 0 and 53751 <= r["previous_reference"] <= 53774
                            for r in summer))
        canary = comparison["canary"]["record_differences"]
        missing = [r for r in canary if r["field"] == "scroll_item_id"]
        self.assertEqual({(r["name"], r["tier"]) for r in missing}, {("Vibrancy", 2), ("Vibrancy", 3)})
        self.mutation_rejected(lambda p: p["previous_reference_comparison"]["crystal"].update(record_differences=[]))
        self.mutation_rejected(lambda p: p["previous_reference_comparison"]["canary"]["record_differences"].pop())

    def test_corrected_scroll_capture_cannot_be_forged_or_disconnected_from_raw_sources(self):
        def capture(packet):
            return packet["corrected_previous_scroll_bindings"]["crystal"]
        self.mutation_rejected(lambda p: capture(p)["records"][0].update(scroll_item_id=0))
        self.mutation_rejected(lambda p: capture(p)["records"][0].update(source_line=999999))
        self.mutation_rejected(lambda p: capture(p).update(sha256="0" * 64))
        self.mutation_rejected(lambda p: capture(p).update(revision=branch.REVISION))
        self.mutation_rejected(lambda p: capture(p).update(records=[capture(p)["records"][0]] * 72))
        self.mutation_rejected(lambda p: p.update(original_capture_defects=[]))
        for name in ("canary", "crystal", "invented"):
            with self.subTest(reference=name), self.assertRaises(ValueError):
                branch.verify_previous_xml(self.packet, name, b"<imbuements/>")


if __name__ == "__main__":
    unittest.main()
