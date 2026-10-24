"""Reference parity and reproducibility; Rust integration tests own compiler admission."""
import copy
from fractions import Fraction
import unittest
from unittest import mock

import jsonschema

import craft_authoring as craft


class CraftAuthoringTests(unittest.TestCase):
    def test_regeneration_is_deterministic_and_committed_content_is_current(self):
        first = craft.outputs()
        self.assertEqual(craft.canonical_bytes(first), craft.canonical_bytes(craft.outputs()))
        self.assertEqual(craft.content_command(check=True), 0)
        self.assertEqual(len(first[craft.CATALOGUE]["records"]), 125)

    def test_source_snapshot_substitution_is_rejected(self):
        with mock.patch.object(craft, "SNAPSHOT_SHA256", "0" * 64):
            with self.assertRaisesRegex(ValueError, "source snapshot digest"):
                craft.build()

    def test_mechanical_corpse_keeps_all_nineteen_exact_probabilities(self):
        rows = {r["identity"]["key"].split(".")[-1]: r["recipe"] for r in craft.build()}
        alternatives = rows["corpse-mechanical"]["outputs"]["alternatives"]
        observed = {int(a["entries"][0]["item_id"]["key"].split("i")[-1]): a["weight"] for a in alternatives}
        self.assertEqual(observed, {281: 10, 282: 10, 9303: 10, 3026: 12, 3029: 12, 3032: 12, 7158: 12, 7159: 12,
                                    3035: 35, 3051: 35, 3052: 35, 3580: 35, 236: 35, 237: 35,
                                    3119: 540, 3123: 540, 3264: 540, 3409: 540, 3578: 540})
        self.assertEqual(sum(observed.values()), 3000)
        self.assertEqual(rows["corpse-mechanical"]["target_effect"], {"effect": "RETIRE"})
        ordinary = rows["corpse-ordinary"]["outputs"]["alternatives"]
        self.assertEqual(sum(a["weight"] for a in ordinary), 10000)
        self.assertEqual(next(a["weight"] for a in ordinary if not a["entries"]), 8885)

    def test_dirty_water_nested_tables_are_exact_and_not_fishing(self):
        row = next(r["recipe"] for r in craft.build() if r["identity"]["key"].endswith(".dirty-water"))
        alternatives = row["outputs"]["alternatives"]
        self.assertEqual(len(alternatives), 10)
        self.assertEqual(sum(a["weight"] for a in alternatives), 600)
        self.assertEqual(sorted(a["weight"] for a in alternatives), [3, 6, 14, 14, 23, 108, 108, 108, 108, 108])
        self.assertEqual(len(row["operands"]), 1)
        self.assertNotIn("skill", row)
        self.assertNotIn("tile_cooldown_ms", row)

    def test_all_cooking_branches_and_fluid_pairs_are_present(self):
        rows = craft.build()
        by_key = {r["identity"]["key"].split(".")[-1]: r["recipe"] for r in rows}
        vessels = craft.source_snapshot()["facts"]["baking"]["tables"]["liquidContainers"]
        self.assertEqual(sum(k.startswith("flour-") for k in by_key), len(vessels) * 2)
        for vessel in vessels:
            for fluid in (1, 9):
                target = by_key[f"flour-{vessel}-{fluid}"]["operands"][1]
                self.assertEqual((target["role"], target["sub_type"], target["output_sub_type"], target["lifecycle"]),
                                 ("TARGET", fluid, 0, "PRESERVE_INSTANCE"))
        self.assertLessEqual({"chocolate-dough", "garlic-dough", "garlic-tray", "sugar-oat"}, set(by_key))
        self.assertEqual(sum(k.startswith("bake-") for k in by_key), 16)
        self.assertEqual(sum(k.startswith("mill-") for k in by_key), 8)
        self.assertEqual(sum(k.startswith("cookies-") for k in by_key), 4)
        self.assertTrue(all(r["outputs"]["entries"][0]["count"] == 12 for k, r in by_key.items() if k.startswith("cookies-")))

    def test_fishing_curve_bait_timers_and_complete_adopted_water_sets(self):
        rows = [r["recipe"] for r in craft.build() if "skill" in r["recipe"]]
        self.assertEqual(len(rows), 46)
        by_rod = {}
        for row in rows:
            rod = row["input_item"]["key"]
            by_rod.setdefault(rod, set()).add(row["target"]["definition"]["key"])
            self.assertEqual(row["chance_curve"], craft.CURVE)
            self.assertEqual(row["operands"][0]["effect"], "KEEP")
            ingredient = row["operands"][1]
            self.assertEqual((ingredient["role"], ingredient["effect"]), ("INGREDIENT", "CONSUME"))
            self.assertEqual((ingredient["consume_on"], ingredient["count"]), ("SUCCESS", 1))
            self.assertEqual(ingredient["item_id"], craft.item(3492 if rod == craft.item(3483)["key"] else 953))
            water = row["target"]["definition"]["key"]
            self.assertEqual(row["tile_cooldown_ms"], 900000 if water == craft.item(7236)["key"] else 3600000 if water == craft.item(13988)["key"] else 1200000)
            if water == craft.item(13988)["key"]:
                self.assertTrue(any("PARITY_PENDING" in d for d in row["source"]["differences"]))
        self.assertEqual(by_rod[craft.item(3483)["key"]], by_rod[craft.item(9306)["key"]])
        self.assertNotIn(craft.item(622)["key"], by_rod[craft.item(3483)["key"]])
        self.assertNotIn(craft.item(13989)["key"], by_rod[craft.item(3483)["key"]])

    def test_schema_enforces_amended_weighted_ceiling(self):
        rows = craft.build()
        template = next(r for r in rows if r["identity"]["key"].endswith(".corpse-mechanical"))
        validator = jsonschema.Draft202012Validator(craft.load(craft.Path(__file__).with_name("schema.json")))
        for size, accepted in [(0, False), (1, False), (2, True), (19, True), (24, True), (25, False)]:
            with self.subTest(size=size):
                row = copy.deepcopy(template)
                row["recipe"]["outputs"]["alternatives"] = [row["recipe"]["outputs"]["alternatives"][0]] * size
                self.assertEqual(validator.is_valid([row]), accepted)

    def test_weighted_lowering_combines_duplicate_identities_without_rounding(self):
        result = craft.weighted([(1, Fraction(1, 3)), (1, Fraction(1, 6)), (2, Fraction(1, 2))])
        self.assertEqual(result["alternatives"], [{"weight": 1, "entries": [craft.output(1)]}, {"weight": 1, "entries": [craft.output(2)]}])


if __name__ == "__main__":
    unittest.main()
