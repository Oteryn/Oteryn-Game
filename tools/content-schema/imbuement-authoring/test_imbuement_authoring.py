#!/usr/bin/env python3
"""Offline checks for source selection, category exclusion and fail-closed drafting."""
import copy
import json
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator

import imbuement_authoring as authoring


class ImbuementAuthoringTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.candidate = authoring.build()

    def definition(self, name, candidate=None):
        return next(d for d in (candidate or self.candidate)["definitions"] if d["name"] == name)

    def rejects(self, mutate):
        candidate = copy.deepcopy(self.candidate)
        mutate(candidate)
        with self.assertRaises(ValueError):
            authoring.validate(candidate)

    def test_committed_files_have_no_drift(self):
        for path, data in ((authoring.SCHEMA, authoring.schema()),
                           (authoring.CATALOGUE, self.candidate),
                           (authoring.REPORT, authoring.comparison())):
            authoring.write_or_check(path, data, True)
        authoring.validate(self.candidate)
        Draft202012Validator.check_schema(authoring.schema())

    def test_full_type_and_category_coverage(self):
        definitions = self.candidate["definitions"]
        self.assertEqual(len(definitions), 24)
        self.assertEqual(sum(len(d["tiers"]) for d in definitions), 72)
        self.assertEqual(len({d["category"] for d in definitions}), 20)
        for name in ("Scorch", "Venom", "Frost", "Electrify", "Reap"):
            self.assertEqual(self.definition(name)["category"], "elemental_damage")

    def test_strike_separates_modifier_from_intrinsic_character_baseline(self):
        tiers = self.definition("Strike")["tiers"]
        self.assertEqual([t["effect"]["chance_bps"] for t in tiers], [500] * 3)
        self.assertEqual([t["effect"]["extra_damage_bps"] for t in tiers], [500, 1500, 4000])
        rules = {r["id"]: r for r in authoring.supporting()["global-rules-evidence.json"]["rules"]}
        baseline = rules["critical_intrinsic_baseline"]["value"]
        self.assertEqual([baseline["chance_bps"] + t["effect"]["chance_bps"] for t in tiers], [1000] * 3)
        self.assertEqual([baseline["extra_damage_bps"] + t["effect"]["extra_damage_bps"] for t in tiers], [1500, 2500, 5000])
        for tier in tiers:
            self.assertEqual(tier["effect"]["value_semantics"], "ADDITIVE_IMBUEMENT_MODIFIER")
            self.assertEqual(tier["provenance"]["effect"], "global-rules-evidence.json#strike_additive_modifiers")

    def test_reject_critical_totals_as_additive_modifiers(self):
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][2]["effect"].update(
            chance_bps=1000, extra_damage_bps=5000))
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["effect"].pop("value_semantics"))
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["provenance"].update(effect="wiki_br"))

    def test_vibrancy_is_recovery_on_an_additional_attack(self):
        tiers = self.definition("Vibrancy")["tiers"]
        self.assertEqual([t["effect"]["remove_chance_bps"] for t in tiers], [1500, 2500, 5000])
        for tier in tiers:
            self.assertEqual(tier["effect"]["kind"], "paralysis_recovery")
            self.assertEqual(tier["effect"]["trigger"], "additional_paralysis_attack_while_paralysed")
        self.rejects(lambda c: self.definition("Vibrancy", c)["tiers"][0].update(
            effect={"kind": "paralysis_deflection", "chance_bps": 1500}))

    def test_completion_does_not_admit_post_target_item_or_proposed_refs(self):
        report = authoring.comparison()["completion"]
        self.assertEqual(report["current_equipment_typed"], 629)
        self.assertEqual(report["target_equipment_typed"], 629)
        self.assertEqual(report["target_existing_item_refs"], 627)
        self.assertEqual(report["validated_missing_item_proposals"], 2)
        self.assertEqual(report["gold_token_exchange_bundles"], 9)

    def test_basic_punch_uses_wiki_recipe_over_canary(self):
        material = self.definition("Punch")["tiers"][0]["materials"][0]
        self.assertEqual(material["source_name"], "Tarantula Egg")
        self.assertEqual(material["count"], 25)
        self.assertEqual(material["item_binding"]["key"], "oteryn:item.tibia.i10281")

    def test_recipes_are_cumulative(self):
        tiers = self.definition("Void")["tiers"]
        self.assertEqual([len(t["materials"]) for t in tiers], [1, 2, 3])
        self.assertEqual(tiers[0]["materials"], tiers[2]["materials"][:1])

    def test_death_protection_has_its_own_values(self):
        tiers = self.definition("Lich Shroud")["tiers"]
        self.assertEqual([t["effect"]["absorb_bps"] for t in tiers], [200, 500, 1000])

    def test_current_global_fees_exclude_obsolete_xml_prices(self):
        tiers = self.definition("Vampirism")["tiers"]
        self.assertEqual([t["apply_fee_gold"] for t in tiers], [7500, 60000, 250000])
        self.assertEqual([t["clear_fee_gold"] for t in tiers], [15000] * 3)

    def test_comparison_matches_by_name_and_tier(self):
        report = authoring.comparison()
        material_diffs = [d for d in report["differences"] if d["field"] == "materials"]
        self.assertEqual([(d["name"], d["tier"]) for d in material_diffs], [("Punch", 1)])
        effect_diffs = [d for d in report["differences"] if d["field"] == "effect"]
        self.assertEqual({d["name"] for d in effect_diffs}, {"Strike", "Vibrancy"})

    def test_reject_duplicate_definition(self):
        self.rejects(lambda c: c["definitions"].__setitem__(1, copy.deepcopy(c["definitions"][0])))

    def test_reject_tier_above_three(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][2].__setitem__("tier", 4))

    def test_reject_reordered_tiers(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"].reverse())

    def test_reject_extra_category_for_element(self):
        self.rejects(lambda c: self.definition("Scorch", c).__setitem__("category", "strike"))

    def test_reject_invalid_effect_unit_or_shape(self):
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["effect"].__setitem__("chance_bps", 10001))
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["effect"].__setitem__("chance_bps", 10.5))
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["effect"].__setitem__("share_bps", 1500))

    def test_reject_incomplete_recipe(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][2]["materials"].pop(0))

    def test_reject_duplicate_or_zero_material(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][0]["materials"][0].__setitem__("count", 0))
        self.rejects(lambda c: c["definitions"][0]["tiers"][1]["materials"].__setitem__(1, copy.deepcopy(c["definitions"][0]["tiers"][1]["materials"][0])))

    def test_reject_silent_recipe_or_effect_change(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][0]["materials"][0].__setitem__("count", 21))
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["effect"].__setitem__("chance_bps", 1000))

    def test_reject_invented_binding_and_runtime_admission(self):
        self.rejects(lambda c: c.__setitem__("activation", "READY"))
        self.rejects(lambda c: c["definitions"][0]["tiers"][0]["materials"][0].__setitem__("item_binding", "oteryn:item.10281"))
        self.rejects(lambda c: c["definitions"][0].__setitem__("parity", "PROVEN"))

    def test_reject_access_and_fee_drift(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][1]["access"].__setitem__("premium_required", False))
        self.rejects(lambda c: c["definitions"][0]["tiers"][2].__setitem__("apply_fee_gold", 200000))

    def test_all_materials_and_scrolls_are_canonical_references(self):
        for definition in self.candidate["definitions"]:
            for tier in definition["tiers"]:
                refs = [m["item_binding"] for m in tier["materials"]] + [tier["scroll_item"]]
                self.assertTrue(all(r["family"] == "Item" and r["key"].startswith("oteryn:item.tibia.i") for r in refs))
        self.assertEqual(len({t["scroll_item"]["key"] for d in self.candidate["definitions"] for t in d["tiers"]}), 72)

    def test_engine_scroll_comparison_uses_actual_root_bindings(self):
        report = authoring.comparison()
        rows = [r for r in report["differences"] if r["field"] == "scroll_item_ids"]
        expected = {(d["name"], 1) for d in self.candidate["definitions"]}
        expected |= {("Vibrancy", 2), ("Vibrancy", 3)}
        self.assertEqual({(r["name"], r["tier"]) for r in rows}, expected)
        canonical = {(d["name"], t["tier"]): int(t["scroll_item"]["key"].split(".i")[-1])
                     for d in self.candidate["definitions"] for t in d["tiers"]}
        for row in rows:
            self.assertEqual(row["canary"], [])
            self.assertEqual(row["crystal"], [canonical[row["name"], row["tier"]]])

    def test_authoring_coverage_does_not_hide_canonical_tier_one_integration_gap(self):
        report = authoring.comparison()
        self.assertEqual(report["completion"]["basic_only_eligibility_profiles"], 12)
        self.assertTrue(any("tier1 lowering" in gap for gap in report["blocked"]))
        self.assertTrue(any("READY_UNPOPULATED" in gap for gap in report["blocked"]))
        self.assertEqual(self.candidate["activation"], "DRAFT_NOT_RUNTIME_READY")

    def test_exact_access_and_eligibility_references(self):
        for definition in self.candidate["definitions"]:
            self.assertEqual(definition["item_eligibility_binding"]["candidate_key"], definition["candidate_key"])
            for tier in definition["tiers"]:
                self.assertEqual(tier["access"]["profile"], definition["name"])
                self.assertEqual(tier["access"]["tier"], tier["name"].lower())

    def test_reject_swapped_valid_item_and_scroll_references(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][0]["materials"][0].__setitem__("item_binding", self.definition("Punch")["tiers"][0]["materials"][0]["item_binding"]))
        self.rejects(lambda c: c["definitions"][0]["tiers"][0].__setitem__("scroll_item", self.definition("Punch")["tiers"][0]["scroll_item"]))

    def test_reject_evidence_pin_drift(self):
        pins = dict(authoring.EVIDENCE_PINS)
        pins["imbuement-bindings.json"] = "0" * 64
        with patch.object(authoring, "EVIDENCE_PINS", pins):
            with self.assertRaisesRegex(ValueError, "supporting evidence digest changed"):
                authoring.build()

    def test_reject_source_packet_digest_drift(self):
        with patch.object(authoring, "FACTS_SHA256", "0" * 64):
            with self.assertRaisesRegex(ValueError, "digest changed"):
                authoring.build()

    def test_observation_plan_covers_every_gap_without_inventing_outcomes(self):
        packets = authoring.supporting()
        plan = packets["global-observation-plan.json"]
        self.assertEqual(self.candidate["observation_plan_profile"], "global-observation-plan.json")
        self.assertEqual({r["id"] for r in plan["requirements"]},
                         {r["id"] for r in packets["global-rules-evidence.json"]["unresolved"]})
        authoring.validate_observation_plan(plan, packets)
        edits = [lambda p: p["requirements"].pop(),
                 lambda p: p["requirements"][0]["scenarios"][0].update(expected_global_result=1),
                 lambda p: p["requirements"][0].update(status="GLOBAL_VERIFIED"),
                 lambda p: p["requirements"][0]["source_refs"][0].update(id="invented_primary"),
                 lambda p: p["requirements"][0]["scenarios"][0]["capture_fields"].clear(),
                 lambda p: p["counts"].update(observations_collected_by_this_plan=28),
                 lambda p: p["requirements"][1]["scenarios"][0].update(
                     id=p["requirements"][0]["scenarios"][0]["id"])]
        for edit in edits:
            with self.subTest(edit=edit):
                changed = copy.deepcopy(plan)
                edit(changed)
                with self.assertRaises(ValueError):
                    authoring.validate_observation_plan(changed, packets)

    def test_capture_requirements_cannot_be_weakened_with_the_scenario(self):
        packets = authoring.supporting()
        plan = copy.deepcopy(packets["global-observation-plan.json"])
        plan["capture_field_groups"]["public_context"] = ["public_artifact_url"]
        plan["requirements"][0]["scenarios"][0]["capture_fields"] = ["public_artifact_url"]
        with self.assertRaisesRegex(ValueError, "independently reviewed minima"):
            authoring.validate_observation_plan(plan, packets)
        plan = copy.deepcopy(packets["global-observation-plan.json"])
        payment = next(r for r in plan["requirements"] if r["id"] == "transaction_payment_sources")
        payment["scenarios"][0]["capture_group_refs"] = ["public_context", "item_state"]
        with self.assertRaisesRegex(ValueError, "minimum evidence context"):
            authoring.validate_observation_plan(plan, packets)

    def test_reject_new_unmapped_source_type(self):
        sources = authoring.source_facts()
        sources["wiki_br"]["records"][0]["name"] = "Unknown Imbuement"
        with patch.object(authoring, "source_facts", return_value=sources):
            with self.assertRaisesRegex(ValueError, "type set"):
                authoring.build()


if __name__ == "__main__":
    unittest.main()
