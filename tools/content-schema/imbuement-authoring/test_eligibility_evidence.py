"""Semantic regression checks for source conflicts and fail-closed Item admission."""
import copy
from datetime import date
import json
import unittest
from unittest.mock import patch

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

    def test_primary_slots_do_not_corroborate_a_single_native_type_profile(self):
        native_only = [r for r in self.items.values() if r["selected_eligibility_source"] and
                       r["selected_eligibility_source"].startswith("tibiopedia_i") and
                       len(r["eligibility_sources"]) == 1]
        self.assertEqual(len(native_only), 76)
        self.assertTrue(all(r["evidence_status"] == "COMMUNITY_SINGLE_SOURCE" for r in native_only))
        counts = self.packet["summary"]["evidence_status_counts"]
        self.assertEqual(counts["COMMUNITY_CORROBORATED"], 430)
        self.assertEqual(counts["COMMUNITY_SINGLE_SOURCE"], 97)
        self.assertEqual(counts["COMMUNITY_SINGLE_SOURCE_WITH_CORROBORATED_FIELD"], 1)
        self.assertEqual(counts["DERIVED_SELECTED_OVER_STALE_HELPER"], 101)
        for row in self.items.values():
            if row["evidence_status"] == "COMMUNITY_CORROBORATED":
                profiles = [r["allowed_types"] for r in row["community_comparison"].values()]
                self.assertGreater(len(profiles), 1)
                self.assertTrue(all(p == profiles[0] for p in profiles))

    def test_normalized_native_cap_must_match_raw_magic_level_observation(self):
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["wiki_tables"]["tibiopedia_i51260"][0]["allowed_types"]["epiphany"] = 1
        with self.assertRaisesRegex(ValueError, "normalized native table differs from captured raw"):
            eligibility.build(packet)

    def test_raw_native_caps_and_source_levels_must_match_selected_extraction(self):
        packet = copy.deepcopy(self.packet)
        meta = packet["source_registry"]["tibiopedia_i51260"]
        row = next(r for r in meta["raw_allowed_max_tiers"] if r["source_type"] == "Magic Level")
        row["max_tier"] = 1
        with self.assertRaisesRegex(ValueError, "native raw type rows differ from selected facts"):
            eligibility.build(packet)
        packet = copy.deepcopy(self.packet)
        meta = packet["source_registry"]["tibiopedia_i51260"]
        meta["selected_facts"]["allowed_max_tiers"][0]["source_level"] = "lvl 1:"
        with self.assertRaisesRegex(ValueError, "native selected facts digest mismatch"):
            eligibility.build(packet)

    def test_selected_fact_digest_has_exact_scope_and_is_replayed(self):
        meta = self.packet["source_registry"]["tibiopedia_i51260"]
        self.assertEqual(eligibility.binding.sha256(eligibility.binding.canonical_bytes(meta["selected_facts"])),
                         meta["selected_facts_sha256"])
        for mutation in ("digest", "scope", "excluded_field"):
            with self.subTest(mutation=mutation):
                packet = copy.deepcopy(self.packet)
                meta = packet["source_registry"]["tibiopedia_i51260"]
                if mutation == "digest":
                    meta["selected_facts_sha256"] = "0" * 64
                elif mutation == "scope":
                    meta["selected_facts_digest_scope"] = "only normalized allowed_types"
                else:
                    meta["selected_facts"]["html_path"] = "/tmp/not-a-portable-source"
                    meta["selected_facts_sha256"] = eligibility.binding.sha256(
                        eligibility.binding.canonical_bytes(meta["selected_facts"]))
                with self.assertRaisesRegex(ValueError, "native selected facts digest"):
                    eligibility.build(packet)

    def test_amendments_require_nonempty_unique_existing_sources_and_unique_item_type(self):
        for source_ids in ([], ["missing_source"], ["fandom_stoic_iks_casque_note"] * 2):
            with self.subTest(source_ids=source_ids):
                packet = copy.deepcopy(self.packet)
                packet["normalized_sources"]["type_amendments"][0]["source_ids"] = source_ids
                with self.assertRaisesRegex(ValueError, "type amendment lacks source evidence"):
                    eligibility.build(packet)
        packet = copy.deepcopy(self.packet)
        amendments = packet["normalized_sources"]["type_amendments"]
        amendments.append(copy.deepcopy(amendments[0]))
        with self.assertRaisesRegex(ValueError, "duplicate item/type amendment"):
            eligibility.build(packet)
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["type_amendments"][0]["client_id"] = 9999999
        with self.assertRaisesRegex(ValueError, "type amendment lacks primary item identity"):
            eligibility.build(packet)

    def test_repeated_row_is_not_independent_profile_corroboration(self):
        packet = copy.deepcopy(self.packet)
        table = packet["normalized_sources"]["wiki_tables"]["wiki_helmets"]
        table.append(copy.deepcopy(table[0]))
        with self.assertRaisesRegex(ValueError, "duplicate named item in community source"):
            eligibility.build(packet)

    def test_amendment_value_must_follow_concrete_allowed_and_excluded_tier_claims(self):
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["type_amendments"][0]["max_tier"] = 1
        with self.assertRaisesRegex(ValueError, "maximum differs from concrete selected source claims"):
            eligibility.build(packet)
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["type_amendments"][0]["source_ids"] = ["primary_client"]
        with self.assertRaisesRegex(ValueError, "source lacks a unique matching named type claim"):
            eligibility.build(packet)
        for field, value in (("item_name", "stoic iks cuirass"), ("candidate_key", "bash")):
            with self.subTest(field=field):
                packet = copy.deepcopy(self.packet)
                packet["normalized_sources"]["type_amendments"][0][field] = value
                with self.assertRaisesRegex(ValueError, "source lacks a unique matching named type claim"):
                    eligibility.build(packet)

    def test_typed_amendment_claims_cannot_drift_from_recorded_patch_or_item_note(self):
        for source_id, field, value in (("tibiopedia_release_13_32_14544", "allowed_tiers", [1]),
                                        ("fandom_stoic_iks_casque_note", "excluded_tiers", [2, 3])):
            with self.subTest(source=source_id):
                packet = copy.deepcopy(self.packet)
                packet["source_registry"][source_id]["item_type_tier_claims"][0][field] = value
                with self.assertRaisesRegex(ValueError, "differ from recorded item"):
                    eligibility.build(packet)
        packet = copy.deepcopy(self.packet)
        claims = packet["source_registry"]["tibiopedia_release_13_32_14544"]["item_type_tier_claims"]
        claims.append(copy.deepcopy(claims[0]))
        with self.assertRaisesRegex(ValueError, "source lacks a unique matching named type claim"):
            eligibility.build(packet)

    def test_narrow_field_corroboration_does_not_promote_complete_disputed_profiles(self):
        kinds = {20065: "ALL_TYPES_TIER_CEILING", 34156: "TYPE_MAX_TIER",
                 28715: "TYPE_TIER_AVAILABLE", 39158: "TYPE_TIER_AVAILABLE"}
        for item_id, kind in kinds.items():
            row = self.items[item_id]
            self.assertEqual(row["evidence_status"], "DERIVED_SELECTED_OVER_STALE_HELPER")
            self.assertEqual(row["independent_field_corroborations"][0]["field_kind"], kind)
            self.assertTrue(any(d["kind"] == "COMMUNITY_TYPE_TIER_DISPUTE" for d in row["discrepancies"]))
            self.assertEqual(row["allowed_types"], row["community_comparison"][row["selected_eligibility_source"]]["allowed_types"])
        self.assertEqual(self.packet["summary"]["independently_corroborated_field_items"], 4)
        self.assertEqual(self.packet["summary"]["independently_corroborated_field_claims"], 4)
        self.assertEqual(self.packet["summary"]["discrepancy_counts"]["COMMUNITY_TYPE_TIER_DISPUTE"], 101)
        self.assertEqual(self.items[20068]["independent_field_corroborations"], [])
        self.assertEqual(self.items[34097]["independent_field_corroborations"], [])

    def test_field_corroboration_requires_exact_named_claim_quote_and_tier_token(self):
        for mutation in ("identity", "quote", "tier", "duplicate", "source", "context"):
            with self.subTest(mutation=mutation):
                packet = copy.deepcopy(self.packet)
                references = packet["normalized_sources"]["field_corroborations"]
                claim = packet["source_registry"]["fandom_umbral_blade_basic_ceiling"]["corroboration_claims"][0]
                if mutation == "identity":
                    claim["item_name"] = "umbral master blade"
                elif mutation == "quote":
                    claim["quote"] += " Invented statement."
                elif mutation == "tier":
                    claim["value"] = 2
                elif mutation == "duplicate":
                    references.append(copy.deepcopy(references[0]))
                elif mutation == "source":
                    references[0]["source_id"] = "primary_client"
                else:
                    packet["source_registry"]["tibiaqa_frostflower_swiftness_field"]["corroboration_claims"][0]["quote_context"] = "Ordinary boots"
                with self.assertRaisesRegex(ValueError, "field corroboration|corroboration lacks"):
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
            self.assertEqual(self.items[item_id]["binding_status"], "CANONICAL_ITEM_ABSENT")
        for row in self.items.values():
            if row["binding_status"] == "CANONICAL_CLIENT_NAME_DISPUTE":
                self.assertIsNone(row["item_ref"])
            if row["item_ref"] is not None:
                self.assertEqual(row["item_ref"], {"family": "Item", "revision": "definition-r1",
                    "key": f"oteryn:item.tibia.i{row['client_id']}"})
        self.assertEqual(self.items[6527]["binding_status"], "EXPLICIT_ARTICLE_ALIAS_AND_REVISIONED_WIKI")
        self.assertEqual(self.items[53207]["binding_status"], "CLIENT_AND_REVISIONED_WIKI_CANONICAL_EXISTS")
        self.assertEqual(self.items[23223]["status"], "RETIRED_SOURCE_ITEM_EXCLUDED")
        self.assertIn(self.items[23223]["item_ref"], (None, {"family": "Item", "revision": "definition-r1",
            "key": "oteryn:item.tibia.i23223"}))
        self.assertEqual(self.packet["source_registry"]["tibiopedia_i23223"]["withdrawn_version"], "11.50")

    def test_august_release_is_included_in_current_target_without_canonical_invention(self):
        row = self.items[53192]
        self.assertEqual(self.packet["target"], "global-tibia-current-2026-10-01")
        self.assertEqual(eligibility.TARGET_DATE, date(2026, 10, 1))
        self.assertEqual(row["allowed_types"], {"featherweight": 3})
        self.assertEqual(row["status"], "CANONICAL_ITEM_ABSENT")
        self.assertIsNone(row["item_ref"])
        self.assertEqual(row["target_time_status"], "PRE_TARGET_RELEASE_CONTINUITY_UNVERIFIED")
        self.assertEqual(row["target_time_evidence"]["date"], "2026-08-04")
        self.assertEqual(self.packet["summary"]["typed_items"], 629)
        self.assertEqual(self.packet["summary"]["target_candidate_typed_items"], 629)
        self.assertEqual(self.packet["summary"]["target_candidate_bound_items"], 627)
        self.assertNotIn("POST_TARGET_RELEASE_EXCLUDED", self.packet["summary"]["target_time_status_counts"])
        packet = copy.deepcopy(self.packet)
        packet["normalized_sources"]["release_dates"]["15.32.fc9100"]["date"] = "2026-07-01"
        with self.assertRaisesRegex(ValueError, "release date differs from pinned source"):
            eligibility.build(packet)

    def test_source_release_day_boundary_is_inclusive(self):
        for target_date, excluded in ((date(2026, 8, 3), True), (date(2026, 8, 4), False)):
            with self.subTest(target_date=target_date), patch.object(eligibility, "TARGET_DATE", target_date):
                result = eligibility.build(self.packet)
                row = next(r for r in result["items"] if r["client_id"] == 53192)
                self.assertEqual(row["target_time_status"] == "POST_TARGET_RELEASE_EXCLUDED", excluded)
                self.assertEqual(row["allowed_types"], {"featherweight": 3})
                self.assertIsNone(row["item_ref"])

    def test_future_release_remains_excluded_until_its_day(self):
        # A synthetic fixture exercises a release after the new current target;
        # the immutable production source dates are never changed or regenerated.
        packet = copy.deepcopy(self.packet)
        release = packet["normalized_sources"]["release_dates"]["15.32.fc9100"]
        release["date"] = "2026-10-02"
        packet["source_registry"][release["source_id"]]["release_date"] = release["date"]
        for target_date, excluded in ((date(2026, 10, 1), True), (date(2026, 10, 2), False)):
            with self.subTest(target_date=target_date), patch.object(eligibility, "TARGET_DATE", target_date):
                result = eligibility.build(packet)
                row = next(r for r in result["items"] if r["client_id"] == 53192)
                self.assertEqual(row["target_time_status"] == "POST_TARGET_RELEASE_EXCLUDED", excluded)
                self.assertEqual(result["summary"]["target_candidate_typed_items"], 628 if excluded else 629)
        self.assertEqual(self.packet["normalized_sources"]["release_dates"]["15.32.fc9100"]["date"], "2026-08-04")

    def test_dated_item_patch_corrects_one_cap_without_promoting_whole_profile(self):
        row = self.items[44636]
        self.assertEqual(row["community_comparison"]["wiki_helmets"]["allowed_types"]["epiphany"], 1)
        self.assertEqual(row["allowed_types"]["epiphany"], 2)
        self.assertEqual(row["evidence_status"], "COMMUNITY_SINGLE_SOURCE_WITH_CORROBORATED_FIELD")
        self.assertEqual(len(row["selected_type_amendments"]), 1)
        self.assertEqual(self.packet["source_registry"]["tibiopedia_release_13_32_14544"]["release_date"], "2024-01-23")
        audit = self.packet["source_registry"]["type_profile_followup_audit"]
        self.assertEqual(audit["full_profiles_newly_corroborated"], 0)
        self.assertEqual(audit["remaining_independent_profile_proof"], 22)

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
