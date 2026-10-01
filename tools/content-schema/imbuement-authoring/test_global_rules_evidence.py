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

    def test_echo_scroll_primary_teaser_and_live_release_remain_distinct(self):
        teaser = self.sources["official_news_8834_basic_scrolls"]
        live = self.sources["official_news_8845_release"]
        self.assertEqual(teaser["published_date"], "2026-06-11")
        self.assertEqual(live["published_date"], "2026-07-13")
        self.assertEqual(teaser["access_status"], "FULL_BROWSER_TEXT_EXTRACTED")
        self.assertEqual(teaser["previous_capture"]["access_status"], "INDEXED_SEARCH_SNIPPET_ONLY")
        self.assertIn("always drop one", teaser["selected_quote"])
        self.assertNotIn("always drop one", live["selected_quote"])
        rule = self.rules["echo_warden_basic_scroll_drop_announcement"]
        self.assertEqual(rule["evidence"], ["official_news_8834_basic_scrolls"])
        self.assertEqual(rule["parity_status"], "OFFICIAL_PUBLIC_TEASER_STATEMENT_ONLY")
        self.assertIn("NOT_LIVE_DROP_MEASUREMENT", rule["target_time_status"])
        self.assertIn("official_news_8845_release", self.rules["basic_scrolls_exist"]["evidence"])

    def test_market_constraint_does_not_become_scroll_or_transfer_prohibition(self):
        rule = self.rules["market_listing_with_active_imbuement_allowed"]
        self.assertFalse(rule["value"])
        self.assertEqual(rule["parity_status"], "DERIVED_COMMUNITY_HISTORICAL_CORROBORATED")
        self.assertIn("CONTINUITY_UNVERIFIED", rule["target_time_status"])
        dates = {self.sources[s]["published_date"] for s in rule["evidence"]}
        self.assertEqual(dates, {"2020-01-22", "2020-04-04"})
        self.assertIn("completed scroll", rule["notes"])
        self.assertIn("direct player trade", rule["notes"])
        self.assertIsNone(self.rules["transfer_preserves_imbuement_state_and_remaining_duration"]["value"])

    def test_forge_tier_transfer_is_not_player_ownership_transfer(self):
        rule = self.rules["exaltation_forge_requires_unimbued_inputs"]
        self.assertEqual(rule["value"]["operations"], ["fusion", "forge_tier_transfer"])
        self.assertTrue(rule["value"]["requires_unimbued_items"])
        self.assertTrue(rule["value"]["imbuing_after_fusion_allowed"])
        source = self.sources["official_forge_nonimbued_inputs_2021"]
        self.assertEqual(source["revision"], "forum-post:39252841")
        self.assertIn("postid=39252841", source["url"])
        self.assertEqual(source["author_role"], "Community Manager")
        self.assertIn("teaser summary", rule["notes"])
        self.assertIn("CONTINUITY_UNVERIFIED", rule["target_time_status"])

    def test_pre_target_etcher_archive_does_not_fill_transaction_unknowns(self):
        source = self.sources["wiki_br_etcher_archived_2026_01_30"]
        self.assertEqual(source["revision"], "428283")
        self.assertLess(source["archive_snapshot_at"][:10], "2026-07-28")
        self.assertEqual(source["access_status"], "FULL_PUBLIC_ARCHIVE_HTML_EXTRACTED")
        self.assertEqual(source["explicit_facts"]["npc_price_gold"], 30000)
        self.assertTrue(source["explicit_facts"]["usable_by_free_account"])
        for key in ("etcher_consumed_units", "etcher_invalid_target_consumed_units",
                    "etcher_npc_purchase_requires_premium"):
            self.assertIsNone(self.rules[key]["value"])
        self.assertIn("wiki_br_etcher_archived_2026_01_30",
                      self.rules["etcher_npc_purchase_worthy_predicate"]["evidence"])

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
        self.assertNotEqual(conflict["candidate_proposal_value"], conflict["global_value"])

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
        self.assertTrue(all(not attempt["used_for_values"]
                            for attempt in self.packet["additional_public_access_attempts"]))

    def test_basic_scroll_timeline_precedes_frozen_target_without_primary_date_overclaim(self):
        introduction = self.rules["basic_scroll_introduction"]
        self.assertEqual(introduction["value"]["client_version"], "15.30")
        self.assertLessEqual(introduction["value"]["release_date"], "2026-07-28")
        self.assertEqual(introduction["value"]["teaser_date"], "2026-06-11")
        self.assertTrue(introduction["parity_status"].startswith("DERIVED"))
        self.assertNotIn("basic_scroll_origin_date", {r["id"] for r in self.packet["unresolved"]})

    def test_concrete_engine_hypotheses_do_not_silently_fill_global_unknowns(self):
        for key in ("etcher_consumed_units", "etcher_invalid_target_consumed_units",
                    "scroll_application_accepts_equipped_target", "fine_grained_timer_hypothesis"):
            rule = self.rules[key]
            self.assertIsNone(rule["value"])
            self.assertEqual(rule["parity_status"], "PARITY_PENDING")
            self.assertTrue(rule["hypotheses"])
            for hypothesis in rule["hypotheses"]:
                self.assertEqual(hypothesis["status"], "OTS_HYPOTHESIS_ONLY")
                self.assertFalse(hypothesis["selected_as_global"])
                self.assertTrue(all(self.sources[s]["role"] == "OTS_HYPOTHESIS_ONLY"
                                    for s in hypothesis["evidence"]))

    def test_same_bonus_limit_does_not_invent_broad_category_exclusions(self):
        rule = self.rules["same_imbuement_type_maximum_per_item"]
        self.assertEqual(rule["value"], 1)
        self.assertEqual(rule["evidence"], ["wiki_br_imbuements"])
        self.assertTrue(rule["parity_status"].startswith("DERIVED"))
        self.assertIn("does not prove exclusion", rule["notes"])

    def test_shared_conversion_exclusion_uses_independent_pre_target_community_revision(self):
        rule = self.rules["shared_category_compatibility"]
        self.assertEqual(rule["parity_status"], "DERIVED_COMMUNITY_REVISION_EXPLICIT")
        self.assertEqual(rule["evidence"], ["fandom_imbuing_full"])
        self.assertEqual(set(rule["value"]["mutually_exclusive_types"]),
                         {"Scorch", "Venom", "Frost", "Electrify", "Reap"})
        self.assertNotIn("shared_category_compatibility",
                         {r["id"] for r in self.packet["unresolved"]})
        self.assertEqual(rule["architecture_reference"]["status"], "CANDIDATE_NOT_ACCEPTED")
        self.assertFalse(rule["hypotheses"][0]["selected_as_global"])
        self.assertEqual(self.rules["same_imbuement_type_maximum_per_item"]["value"], 1)
        self.assertEqual(self.sources["fandom_imbuing_full"]["revision"], "1194750")
        self.assertLess(self.sources["fandom_imbuing_full"]["published_date"], "2026-07-28")

    def test_higher_required_level_is_scoped_community_evidence(self):
        rule = self.rules["equipment_above_character_level_imbuement_behavior"]
        self.assertEqual(rule["value"], {"effect_active": False,
                                       "standard_equipped_combat_timer_continues": True})
        self.assertEqual(rule["evidence"], ["tibiopedia_imbuing_guide"])
        self.assertTrue(rule["parity_status"].startswith("DERIVED"))
        for key in ("swiftness_bonus_points_by_tier", "strike_chance_percent",
                    "strike_extra_damage_percent_by_tier"):
            self.assertNotIn("tibiopedia_imbuing_guide", self.rules[key]["evidence"])

    def test_complete_transaction_composition_and_transfer_scope_stays_unqualified(self):
        pending = {r["id"] for r in self.packet["unresolved"]}
        for key in ("failed_transaction_consumption_and_rollback",
                    "native_effect_composition",
                    "transfer_preserves_imbuement_state_and_remaining_duration"):
            self.assertIn(key, pending)
            self.assertIsNone(self.rules[key]["value"])
            self.assertEqual(self.rules[key]["parity_status"], "PARITY_PENDING")

    def test_bounded_payment_preference_does_not_invent_transaction_order(self):
        rule = self.rules["transaction_payment_sources"]
        self.assertTrue(rule["parity_status"].startswith("DERIVED"))
        self.assertEqual(rule["value"]["scope"], "NORMAL_SHRINE_IMBUING")
        self.assertEqual(rule["value"]["material_source_priority"], ["inventory", "stash"])
        self.assertEqual(rule["value"]["gold_source_priority"], ["inventory", "bank"])
        for field in ("exact_inventory_container_search_order",
                      "exact_debit_order_between_gold_and_materials", "other_transaction_routes"):
            self.assertIsNone(rule["value"][field])
        self.assertIn("transaction_payment_sources", {r["id"] for r in self.packet["unresolved"]})

    def test_guide_intricate_strike_modifiers_are_not_rejected_as_effective_totals(self):
        source = next(r for r in self.packet["rejected_sources"]
                      if r["url"] == "https://tibiopedia.pl/articles/3,Imbuing-nasycenia")
        self.assertIn("correctly describe additive imbuement modifiers", source["corroborating_subset"])
        self.assertIn("Main Basic Strike", source["reason"])
        self.assertIn("Swiftness says20 rather than30", source["reason"])

    def test_intrinsic_critical_baseline_is_separate_from_additive_strike(self):
        baseline = self.rules["critical_intrinsic_baseline"]["value"]
        modifier = self.rules["strike_additive_modifiers"]["value"]
        self.assertEqual(baseline, {"chance_bps": 500, "extra_damage_bps": 1000})
        self.assertEqual(modifier, {"chance_bps": 500,
                                   "extra_damage_bps_by_tier": [500, 1500, 4000],
                                   "value_semantics": "ADDITIVE_IMBUEMENT_MODIFIER"})
        self.assertEqual(self.rules["strike_chance_percent"]["value"] * 100,
                         baseline["chance_bps"] + modifier["chance_bps"])
        self.assertEqual([v * 100 for v in self.rules["strike_extra_damage_percent_by_tier"]["value"]],
                         [baseline["extra_damage_bps"] + v for v in modifier["extra_damage_bps_by_tier"]])
        for key in ("strike_chance_percent", "strike_extra_damage_percent_by_tier"):
            self.assertEqual(self.rules[key]["value_semantics"],
                             "ISOLATED_EFFECTIVE_TOTAL_INCLUDING_INTRINSIC_BASELINE")
        self.assertEqual(self.sources["fandom_critical_full"]["published_date"], "2025-07-24")
        self.assertEqual(self.sources["official_news_8436_release"]["published_date"], "2025-07-21")

    def test_new_full_primary_captures_preserve_honest_method_and_prior_snippet_provenance(self):
        for key in ("official_news_8396_fees", "official_news_8396_success",
                    "official_news_8396_scrolls", "official_news_8436_release"):
            source = self.sources[key]
            self.assertEqual(source["access_status"], "FULL_BROWSER_TEXT_EXTRACTED")
            self.assertEqual(source["method"], "Remote Desktop + Chrome/CDP")
            self.assertEqual(source["previous_capture"]["access_status"], "INDEXED_SEARCH_SNIPPET_ONLY")
            self.assertRegex(source["previous_capture"]["sha256"], r"^[0-9a-f]{64}$")
        self.assertFalse(self.rules["vibrancy_reflection_at_live_introduction"]["value"])
        self.assertEqual(self.rules["vibrancy_reflection_at_live_introduction"]["evidence"],
                         ["official_news_4828_full"])

    def test_vibrancy_utility_timer_and_pz_state_counterclaim_do_not_certify_deadlines(self):
        rule = self.rules["timer_vibrancy_equipped_online_outside_combat"]
        self.assertEqual(rule["value"], {"requires_equipped": True,
                                       "requires_online": True, "combat_required": False})
        self.assertTrue(rule["parity_status"].startswith("DERIVED"))
        self.assertEqual(self.sources["tibiaqa_utility_timers_2021"]["published_date"], "2021-02-04")
        self.assertTrue(self.rules["hidden_logout_block_does_not_prove_out_of_combat"]["value"])
        conflict = next(c for c in self.packet["conflicts"]
                        if c["id"] == "combat_timer_pz_residual_state_source_conflict")
        self.assertIsNone(conflict["selected_runtime_pz_timer_policy"])
        self.assertIn("fine_grained_timers", {r["id"] for r in self.packet["unresolved"]})

    def test_post_target_mana_overkill_claim_does_not_establish_life_or_target_pipeline(self):
        rule = self.rules["leech_overkill_counts"]
        self.assertEqual(rule["scope"], "MANA_LEECH_COMMUNITY_CLAIM_ONLY")
        self.assertTrue(rule["parity_status"].startswith("DERIVED"))
        self.assertGreater(self.sources["fandom_formulae_full"]["published_date"], "2026-07-28")
        self.assertIn("combat_pipeline", {r["id"] for r in self.packet["unresolved"]})

    def test_etcher_purchase_gate_is_separate_from_free_application(self):
        self.assertTrue(self.rules["etcher_usable_by_free_account"]["value"])
        self.assertTrue(self.rules["etcher_npc_purchase_requires_worthy"]["value"])
        predicate = self.rules["etcher_npc_purchase_worthy_predicate"]["value"]
        self.assertEqual(predicate["predicate_ref"], "worthy_character")
        self.assertEqual(predicate["requirements"], ["completed_world_construction",
                                                     "completed_five_tome_handovers"])
        self.assertIsNone(self.rules["etcher_npc_purchase_requires_premium"]["value"])
        self.assertEqual(self.rules["etcher_npc_purchase_requires_premium"]["parity_status"],
                         "PARITY_PENDING")

    def test_architecture_conflicts_retain_candidate_proposal_authority(self):
        for conflict in self.packet["conflicts"]:
            self.assertNotIn("accepted_decision_value", conflict)
            if "candidate_proposal_value" in conflict:
                self.assertEqual(conflict["architecture_status"], "CANDIDATE_NOT_ACCEPTED")

    def test_pvp_snippet_qualification_keeps_success_state_gate_unresolved(self):
        rule = self.rules["vibrancy_pvp_gate"]
        self.assertIsNone(rule["value"])
        self.assertEqual(rule["parity_status"], "PARITY_PENDING")
        self.assertIn("vibrancy_pvp_gate", {r["id"] for r in self.packet["unresolved"]})
        self.assertNotIn("vibrancy_pvp", {r["id"] for r in self.packet["closed_gaps"]})
        conflict = next(r for r in self.packet["conflicts"]
                        if r["id"] == "vibrancy_pvp_success_state_gate")
        self.assertEqual(conflict["family_page_qualification"], "if initially successful")
        self.assertEqual(set(conflict["evidence"]), set(rule["evidence"]))
        for key in rule["evidence"]:
            self.assertEqual(self.sources[key]["access_status"], "INDEXED_SEARCH_SNIPPET_ONLY")
            self.assertEqual(self.sources[key]["role"], "DERIVED")
        profile = next(r for r in self.packet["delegated_profiles"]
                       if r["id"] == "vibrancy_sequence")
        self.assertEqual(profile["status"], "PARTIALLY_CORROBORATED_NOT_CLOSED")

    def test_probability_does_not_encode_vibrancy_as_initial_admission_chance(self):
        self.assertEqual(self.rules["vibrancy_deflection_chance_percent_by_tier"]
                         ["runtime_semantics_profile"], "imbuement-combat.json")
        self.assertIn("vibrancy_admission_only_probability",
                      {conflict["id"] for conflict in self.packet["conflicts"]})

    def test_unresolved_research_preserves_actual_failed_and_successful_attempts(self):
        attempts = {row["id"]: row for row in self.packet["research_attempts"]}
        self.assertIn("HTTP429", attempts["follow_up_queries"]["outcome"])
        self.assertIn("HTTP403", attempts["primary_dated_news"]["outcome"])
        self.assertIn("BothHTTP200", attempts["native_etcher_and_guide"]["outcome"])
        self.assertTrue(attempts["concrete_implementation_hypotheses"]["urls"])

    def test_historical_zero_minute_display_is_not_exact_expiration_evidence(self):
        rule = self.rules["historical_minute_display_and_slot_occupancy_example"]
        value = rule["value"]
        self.assertEqual(value["scope"], "REPORTED_HISTORICAL_DISPLAY_AND_OCCUPANCY_EXAMPLE")
        self.assertEqual(value["reported_before"],
                         {"displayed_time": "13:53", "remaining_duration_display": "0:00h"})
        self.assertEqual(value["reported_after"],
                         {"displayed_time": "13:56", "remaining_duration_display": "0:00h"})
        self.assertTrue(value["reported_reimbue_refused"])
        self.assertFalse(value["reported_attacking_during_interval"])
        self.assertIsNone(value["exact_remaining_seconds"])
        self.assertIsNone(value["display_zero_means_expired"])
        self.assertEqual(rule["parity_status"], "DERIVED_DATED_PUBLIC_PLAYER_REPORT")
        source = self.sources[rule["evidence"][0]]
        self.assertEqual(source["published_date"], "2020-03-19")
        self.assertEqual(source["accepted_answer_date"], "2020-03-19")
        self.assertEqual(source["last_edit_date"], "2020-03-20")
        self.assertIn("NOT_2026_TARGET_OBSERVATION", rule["target_time_status"])
        self.assertIn(rule["evidence"][0], self.rules["fine_grained_timer_hypothesis"]["evidence"])
        self.assertIsNone(self.rules["fine_grained_timer_hypothesis"]["value"])

    def test_completed_scroll_consumption_is_separate_and_keeps_ots_hypotheses_pending(self):
        for key, units in (("scroll_application_consumed_units", 1),
                           ("scroll_application_invalid_target_consumed_units", 0)):
            rule = self.rules[key]
            self.assertIsNone(rule["value"])
            self.assertEqual(rule["parity_status"], "PARITY_PENDING")
            self.assertEqual(rule["hypotheses"][0]["value"], units)
            self.assertEqual(rule["hypotheses"][0]["status"], "OTS_HYPOTHESIS_ONLY")
            self.assertFalse(rule["hypotheses"][0]["selected_as_global"])
            self.assertEqual(rule["hypotheses"][0]["evidence"], ["crystal_imbuements_player_impl"])
        pending = {r["id"] for r in self.packet["unresolved"]}
        self.assertIn("scroll_consumption", pending)
        self.assertEqual(len(pending), 12)
        self.assertIn("failed_transaction_consumption_and_rollback", pending)

    def test_discovery_and_unrelated_trade_receipts_do_not_become_runtime_facts(self):
        attempts = {r["id"]: r for r in self.packet["research_attempts"]}
        for key in ("tracker_button_not_duration_observation",
                    "depot_coin_trade_not_imbuement_preservation",
                    "tibiabr_article_discovery_only", "tibiabr_article_body_api_denied"):
            row = attempts[key]
            self.assertFalse(row["used_for_values"])
            self.assertTrue(row["urls"])
            self.assertTrue(set(row["evidence"]) <= self.sources.keys())
        denied = attempts["tibiabr_article_body_api_denied"]
        self.assertIn("HTTP403", denied["outcome"])
        self.assertEqual(len(denied["urls"]), 4)
        unqualified = {"tibiaqa_tracker_ui_2023", "tibiaqa_depot_coin_trade_2022",
                       "tibiabr_discovery_index_2026"}
        self.assertTrue(all(not (set(r["evidence"]) & unqualified)
                            for r in self.rules.values()))
        self.assertIsNone(self.sources["tibiabr_discovery_index_2026"]["published_date"])
        self.assertIsNone(self.rules["transfer_preserves_imbuement_state_and_remaining_duration"]["value"])

    def test_closed_and_unresolved_findings_have_traceable_urls_without_scope_escalation(self):
        for row in self.packet["closed_gaps"] + self.packet["unresolved"]:
            self.assertTrue(row["source_urls"], row["id"])
            self.assertEqual(row["source_urls"], list(dict.fromkeys(
                self.sources[key]["url"] for key in row["evidence"])))
            self.assertTrue(row["source_url_scope"])
        for row in self.packet["unresolved"]:
            self.assertIn("do not certify", row["source_url_scope"])

    def test_user_requested_crystal_branch_is_distinct_and_keeps_ots_confidence(self):
        source = self.sources["crystal_imbuements_player_impl"]
        self.assertEqual(source["branch"], "imbuements")
        self.assertEqual(source["revision"], "15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1")
        self.assertEqual(source["role"], "OTS_HYPOTHESIS_ONLY")
        self.assertTrue(any("crystal_imbuements_player_impl" in hypothesis["evidence"]
                            for hypothesis in self.rules["etcher_consumed_units"]["hypotheses"]))


if __name__ == "__main__":
    unittest.main()
