"""Regressions for Global source priority and the limits of public evidence."""
import json
from pathlib import Path
import unittest
from urllib.parse import urlparse


class GlobalRulesEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads((Path(__file__).parent / "samples" /
                                 "global-rules-evidence.json").read_text())
        cls.rules = {r["id"]: r for r in cls.packet["rules"]}
        cls.sources = cls.packet["sources"]

    def test_source_references_are_unique_and_complete(self):
        self.assertEqual(len(self.rules), len(self.packet["rules"]))
        for rule in self.rules.values():
            self.assertTrue(rule["evidence"], rule["id"])
            self.assertTrue(set(rule["evidence"]) <= self.sources.keys(), rule["id"])

    def test_every_digest_defines_the_bytes_hashed_and_access_limits(self):
        for source in self.sources.values():
            self.assertRegex(source["sha256"], r"^[0-9a-f]{64}$")
            self.assertTrue(source["digest_scope"])
            if source["access_status"] == "INDEXED_SEARCH_SNIPPET_ONLY":
                self.assertIn("snippet", source["digest_scope"])
            self.assertEqual(source["accessed_date"], self.packet["reviewed_at"])

    def test_official_claims_have_an_official_domain_in_their_evidence(self):
        for rule in self.rules.values():
            if rule["parity_status"].startswith("OFFICIAL_PUBLIC"):
                primary = [self.sources[key] for key in rule["evidence"]
                           if self.sources[key]["role"] == "OFFICIAL_PUBLIC"]
                self.assertTrue(primary, rule["id"])
                self.assertTrue(all(urlparse(s["url"]).hostname == "www.tibia.com"
                                    for s in primary), rule["id"])

    def test_post_2025_flat_fees_override_old_engine_base_costs(self):
        self.assertEqual(self.rules["apply_fee_gold"]["value"],
                         {"basic": 7500, "intricate": 60000, "powerful": 250000})
        self.assertIn("official_news_8396_fees", self.rules["apply_fee_gold"]["evidence"])
        conflict = next(c for c in self.packet["conflicts"]
                        if c["id"] == "accepted_fee_numbers_pre2025")
        self.assertNotEqual(conflict["accepted_decision_value"], conflict["global_value"])

    def test_success_and_protection_are_backed_by_retrieved_primary(self):
        self.assertEqual(self.rules["apply_success_percent"]["value"], 100)
        self.assertEqual(self.rules["protection_fee_gold"]["value"], 0)
        for key in ("apply_success_percent", "protection_fee_gold"):
            self.assertIn("official_news_8396_success", self.rules[key]["evidence"])

    def test_basic_scroll_existence_does_not_authorize_basic_inscription(self):
        self.assertTrue(self.rules["basic_scrolls_exist"]["value"])
        self.assertEqual(self.rules["scroll_inscription_tiers"]["value"], [2, 3])
        self.assertIn("official_news_8436_release",
                      self.rules["scroll_inscription_tiers"]["evidence"])
        self.assertFalse(self.rules["scroll_application_requires_quest"]["value"])
        self.assertFalse(self.rules["scroll_application_requires_premium"]["value"])

    def test_etcher_clear_all_is_separate_from_single_slot_shrine_fee(self):
        self.assertEqual(self.rules["clear_fee_gold"]["value"], 15000)
        self.assertEqual(self.rules["etcher_npc_price_gold"]["value"], 30000)
        self.assertTrue(self.rules["etcher_clears_all_slots"]["value"])
        self.assertIsNone(self.rules["etcher_consumed_units"]["value"])
        self.assertEqual(self.rules["etcher_consumed_units"]["parity_status"], "PARITY_PENDING")

    def test_current_sources_do_not_claim_immutable_frozen_date_certification(self):
        self.assertEqual(self.packet["target"],
                         "global-tibia-observable-2026-07-28-post-server-save")
        pending = {r["id"] for r in self.packet["unresolved"]}
        self.assertTrue({"exact_target_snapshot", "fine_grained_timers", "combat_pipeline"} <= pending)
        self.assertEqual(self.rules["duration_ms"]["target_time_status"],
                         "CURRENT_MANUAL_NOT_IMMUTABLE_TARGET_SNAPSHOT")

    def test_derived_runtime_details_do_not_become_primary_certification(self):
        for key in ("leech_overkill_counts", "timer_swiftness_ticks_outside_combat",
                    "vibrancy_deflection_chance_percent_by_tier"):
            self.assertTrue(self.rules[key]["parity_status"].startswith("DERIVED"))
        self.assertEqual(self.rules["featherweight_capacity_percent_by_tier"]["value"], [3, 8, 15])

    def test_stash_difference_and_known_bad_guide_stay_visible(self):
        self.assertEqual(self.rules["materials_sources"]["value"], ["backpack", "stash"])
        self.assertIn("stash_materials_first_slice", {c["id"] for c in self.packet["conflicts"]})
        self.assertIn("https://tibiavault.com/imbuing-guide",
                      {s["url"] for s in self.packet["rejected_sources"]})

    def test_independent_etcher_page_corroborates_behavior_without_inventing_consumption(self):
        source = self.sources["tibiopedia_etcher"]
        self.assertEqual(source["verified_page_title"], "Przedmioty: Etcher - Tibia ~ Tibiopedia.pl")
        self.assertIn("clears_all_imbuements", source["explicit_facts"])
        self.assertIn("consumed_units", source["unconfirmed_facts"])
        self.assertIn("tibiopedia_etcher", self.rules["etcher_clears_all_slots"]["evidence"])
        self.assertIsNone(self.rules["etcher_consumed_units"]["value"])

    def test_conflicting_guide_effect_values_are_not_selected_for_runtime_rules(self):
        for key in ("swiftness_bonus_points_by_tier", "strike_chance_percent",
                    "strike_extra_damage_percent_by_tier"):
            self.assertNotIn("tibiopedia_imbuing_guide", self.rules[key]["evidence"])
        self.assertIn("vibrancy_pvp", {r["id"] for r in self.packet["unresolved"]})
        self.assertTrue(all(not attempt["used_for_values"]
                            for attempt in self.packet["additional_public_access_attempts"]))


if __name__ == "__main__":
    unittest.main()
