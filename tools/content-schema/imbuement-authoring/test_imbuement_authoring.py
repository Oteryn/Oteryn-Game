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

    def test_strike_uses_wiki_values_over_crystal(self):
        tiers = self.definition("Strike")["tiers"]
        self.assertEqual([t["effect"]["chance_bps"] for t in tiers], [1000] * 3)
        self.assertEqual([t["effect"]["extra_damage_bps"] for t in tiers], [1500, 2500, 5000])

    def test_basic_punch_uses_wiki_recipe_over_canary(self):
        material = self.definition("Punch")["tiers"][0]["materials"][0]
        self.assertEqual(material["source_name"], "Tarantula Egg")
        self.assertEqual(material["count"], 25)
        self.assertEqual(material["item_binding"], "UNRESOLVED")

    def test_recipes_are_cumulative(self):
        tiers = self.definition("Void")["tiers"]
        self.assertEqual([len(t["materials"]) for t in tiers], [1, 2, 3])
        self.assertEqual(tiers[0]["materials"], tiers[2]["materials"][:1])

    def test_death_protection_has_its_own_values(self):
        tiers = self.definition("Lich Shroud")["tiers"]
        self.assertEqual([t["effect"]["absorb_bps"] for t in tiers], [200, 500, 1000])

    def test_accepted_fees_exclude_protection(self):
        tiers = self.definition("Vampirism")["tiers"]
        self.assertEqual([t["apply_fee_gold"] for t in tiers], [5000, 30000, 200000])
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
        self.rejects(lambda c: self.definition("Strike", c)["tiers"][0]["effect"].__setitem__("chance_bps", 500))

    def test_reject_invented_binding_and_runtime_admission(self):
        self.rejects(lambda c: c.__setitem__("activation", "READY"))
        self.rejects(lambda c: c["definitions"][0]["tiers"][0]["materials"][0].__setitem__("item_binding", "oteryn:item.10281"))
        self.rejects(lambda c: c["definitions"][0].__setitem__("parity", "PROVEN"))

    def test_reject_access_and_fee_drift(self):
        self.rejects(lambda c: c["definitions"][0]["tiers"][1]["access"].__setitem__("premium_required", False))
        self.rejects(lambda c: c["definitions"][0]["tiers"][2].__setitem__("apply_fee_gold", 250000))

    def test_reject_source_packet_digest_drift(self):
        with patch.object(authoring, "FACTS_SHA256", "0" * 64):
            with self.assertRaisesRegex(ValueError, "digest changed"):
                authoring.build()

    def test_reject_new_unmapped_source_type(self):
        sources = authoring.source_facts()
        sources["wiki_br"]["records"][0]["name"] = "Unknown Imbuement"
        with patch.object(authoring, "source_facts", return_value=sources):
            with self.assertRaisesRegex(ValueError, "type set"):
                authoring.build()


if __name__ == "__main__":
    unittest.main()
