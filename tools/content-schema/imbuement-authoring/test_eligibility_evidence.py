"""Semantic regression checks for source conflicts and fail-closed Item admission."""
import copy
import json
import unittest

import eligibility_evidence as eligibility


class EligibilityEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads(eligibility.OUTPUT.read_bytes())
        cls.items = {r["client_id"]: r for r in cls.packet["items"]}

    def test_full_census_replays_against_repository_client_and_canonical_items(self):
        self.assertEqual(eligibility.build(self.packet), self.packet)
        self.assertEqual(len(self.items), 663)
        self.assertEqual(self.packet["summary"]["global_verified_items"], 0)

    def test_primary_slot_mutation_is_not_accepted_as_new_fact(self):
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["primary_items"][0]["slots"] = 2
        with self.assertRaisesRegex(ValueError, "stored primary facts differ"):
            eligibility.build(packet)

    def test_native_slot_drift_cannot_select_a_type_allowlist(self):
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["wiki_tables"]["tibiopedia_i49861"][0]["slots"] = 3
        with self.assertRaisesRegex(ValueError, "direct item table slots differ from primary"):
            eligibility.build(packet)

    def test_native_item_restrictions_are_not_overwritten_by_stale_tool(self):
        for item_id, denied, expected in ((49861, "strike", "precision"), (49866, "void", "chop"),
                                         (49869, "void", "chop")):
            row = self.items[item_id]
            self.assertEqual(row["status"], "DERIVED_SELECTED_OVER_STALE_HELPER")
            self.assertEqual(row["selected_eligibility_source"], f"tibiopedia_i{item_id}")
            self.assertNotIn(denied, row["allowed_types"])
            native = row["community_comparison"][f"tibiopedia_i{item_id}"]
            self.assertNotIn(denied, native["allowed_types"])
            self.assertEqual(native["allowed_types"][expected], 3)
            self.assertEqual(native["slots"], row["slots"])
            self.assertEqual(row["allowed_types"], native["allowed_types"])

    def test_lower_tiers_are_preserved_from_explicit_native_tables(self):
        for item_id, key, tier in ((53074, "featherweight", 2), (52352, "swiftness", 2),
                                  (50274, "epiphany", 1), (51260, "epiphany", 2)):
            native = self.items[item_id]["community_comparison"][f"tibiopedia_i{item_id}"]
            self.assertEqual(native["allowed_types"][key], tier)

    def test_canonical_existence_and_revisioned_names_guard_reference_namespace(self):
        for item_id in (49160, 53192):
            self.assertIsNone(self.items[item_id]["item_ref"])
            self.assertEqual(self.items[item_id]["status"], "CANONICAL_ITEM_ABSENT")
        for row in self.items.values():
            if row["binding_status"] == "CANONICAL_CLIENT_NAME_DISPUTE":
                self.assertIsNone(row["item_ref"])
            if row["item_ref"] is not None:
                self.assertEqual(row["item_ref"], {"family": "Item", "revision": "definition-r1",
                    "key": f"oteryn:item.tibia.i{row['client_id']}"})
        self.assertEqual(self.items[6527]["binding_status"], "EXPLICIT_ARTICLE_ALIAS_AND_REVISIONED_WIKI")
        self.assertEqual(self.items[53207]["binding_status"], "CLIENT_AND_REVISIONED_WIKI_CANONICAL_EXISTS")
        self.assertEqual(self.items[23223]["status"], "RETIRED_SOURCE_ITEM_EXCLUDED")
        self.assertIsNone(self.items[23223]["item_ref"])
        self.assertEqual(self.packet["source_registry"]["tibiopedia_i23223"]["withdrawn_version"], "11.50")

    def test_native_tier_cap_wins_without_erasing_helper_conflict(self):
        row = self.items[50164]  # Umbral Katar: helper advertises tier 3, explicit table caps tier 1.
        self.assertEqual(row["status"], "DERIVED_SELECTED_OVER_STALE_HELPER")
        self.assertTrue(row["allowed_types"])
        self.assertEqual(set(row["allowed_types"].values()), {1})
        self.assertTrue(any(d["kind"] == "COMMUNITY_TYPE_TIER_DISPUTE" for d in row["discrepancies"]))

    def test_unknown_and_engine_out_of_domain_values_are_not_coerced(self):
        self.assertEqual(eligibility.engine_types({"increase speed": 10}), {})
        self.assertEqual(eligibility.engine_types({"paralysis removal": 2}), {"vibrancy": 2})
        boots = self.items[3079]["engine_comparison"]["canary"]
        self.assertEqual(boots["unqualified_raw_tier_limits"], {"increase speed": 10})
        for item_id in (28464, 28465, 28478, 28479):
            self.assertEqual(self.items[item_id]["allowed_types"], {})
            self.assertEqual(self.items[item_id]["status"], "MISSING_ELIGIBILITY_EVIDENCE")
        self.assertTrue(any(r["status"] == "NONEXISTENT_OR_NO_JSON_FACTS"
                            for r in self.packet["rejected_sources"]))


if __name__ == "__main__":
    unittest.main()
