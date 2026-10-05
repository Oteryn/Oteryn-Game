"""Offline checks of the TibiaTools auto-attack fixture. No network."""

import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import capture_tibiapal_auto_attack as tool  # noqa: E402

with open(tool.FIXTURE, encoding="utf-8") as handle:
    FIXTURE = json.load(handle)
ROWS = FIXTURE["rows"]


class FixtureGrid(unittest.TestCase):
    def test_schema_and_capture_time(self):
        self.assertEqual(FIXTURE["schema"], tool.SCHEMA)
        self.assertTrue(FIXTURE["capture_started_at"] <= FIXTURE["capture_finished_at"])

    def test_grid_is_the_packet_grid(self):
        grid = FIXTURE["grid"]
        self.assertEqual(grid["vocations"], ["knight", "paladin", "sorcerer", "druid"])
        self.assertEqual(grid["levels"], [8, 50, 100, 300, 600, 1000])
        self.assertEqual(grid["skills"], [10, 50, 100, 120])

    def test_row_count_is_the_full_cross_product(self):
        grid = FIXTURE["grid"]
        cells = len(grid["vocations"]) * len(grid["levels"]) * len(grid["skills"])
        self.assertEqual(len(ROWS), cells * (1 + len(grid["weapons"])))
        self.assertEqual(len({r["key"] for r in ROWS}), len(ROWS))

    def test_no_monk_row(self):
        for row in ROWS:
            self.assertNotEqual(row["request"]["stats"]["vocation"], "monk", row["key"])
        self.assertNotIn("monk", FIXTURE["grid"]["vocations"])

    def test_every_row_records_triple_time_description_and_mode(self):
        for row in ROWS:
            raw = row["raw"]
            self.assertEqual(sorted(raw), ["avg", "max", "min"], row["key"])
            self.assertTrue(all(isinstance(v, int) for v in raw.values()), row["key"])
            self.assertLessEqual(raw["min"], raw["avg"], row["key"])
            self.assertLessEqual(raw["avg"], raw["max"], row["key"])
            self.assertTrue(row["captured_at"].endswith("Z"), row["key"])
            self.assertTrue(row["api_description"], row["key"])
            self.assertEqual(row["oteryn_fight_mode"], "OFFENSIVE", row["key"])
            self.assertEqual(row["oteryn_attack_factor"], 1.0, row["key"])

    def test_requests_carry_only_stats_and_weapon(self):
        for row in ROWS:
            request = row["request"]
            self.assertEqual(sorted(request), ["stats", "weapon"], row["key"])
            self.assertEqual(sorted(request["stats"]), ["level", "skill", "vocation"], row["key"])
            self.assertEqual(sorted(request["weapon"]), ["id"], row["key"])


class RequestsReproduceOffline(unittest.TestCase):
    def test_bodies_are_rebuilt_from_the_grid_definition(self):
        built = [(key, body) for key, _, body in tool.build_requests(FIXTURE["grid"])]
        self.assertEqual(built, [(r["key"], r["request"]) for r in ROWS])


class WeaponRows(unittest.TestCase):
    def setUp(self):
        self.by_id = {w["tibiatools_id"]: w for w in FIXTURE["grid"]["weapons"]}

    def test_one_oteryn_key_per_weapon(self):
        keys = [w["oteryn_item_key"] for w in self.by_id.values()]
        self.assertEqual(len(keys), len(self.by_id))
        self.assertEqual(len(set(keys)), len(keys))
        for key in keys:
            self.assertRegex(key, r"^oteryn:item\.tibia\.i\d+$")

    def test_weapon_rows_record_tibiatools_attack_and_fist_rows_do_not(self):
        for row in ROWS:
            weapon_id = row["request"]["weapon"]["id"]
            if weapon_id == tool.FISTS_ID:
                self.assertNotIn("tibiatools_attack", row, row["key"])
                self.assertNotIn("oteryn_item_key", row, row["key"])
                continue
            weapon = self.by_id[weapon_id]
            self.assertIsInstance(row.get("tibiatools_attack"), int, row["key"])
            self.assertEqual(row["tibiatools_attack"], weapon["tibiatools_attack"], row["key"])
            self.assertEqual(row["oteryn_item_key"], weapon["oteryn_item_key"], row["key"])

    def test_residue_coverage_per_class(self):
        covered = {(w["class"], w["tibiatools_attack"] % 5) for w in self.by_id.values()}
        missing = {(m["class"], m["residue_mod_5"]) for m in FIXTURE["missing_residues"]}
        for cls in tool.WEAPON_CLASSES:
            for residue in range(5):
                self.assertTrue(
                    ((cls, residue) in covered) != ((cls, residue) in missing),
                    "%s %d must be covered or listed missing, not both or neither" % (cls, residue),
                )
        for weapon in self.by_id.values():
            self.assertEqual(weapon["attack_residue_mod_5"], weapon["tibiatools_attack"] % 5)


class SelectWeapons(unittest.TestCase):
    def test_prefers_matching_attack_and_reports_missing(self):
        catalogue = [
            {"id": 1, "name": "A", "skill": "axe", "hands": "two", "attack": 10},
            {"id": 2, "name": "B", "skill": "axe", "hands": "one", "attack": 15},
            {"id": 3, "name": "C", "skill": "axe", "hands": "one", "attack": 20},
            {"id": 4, "name": "Dup", "skill": "axe", "hands": "one", "attack": 11},
            {"id": 5, "name": "Dup", "skill": "axe", "hands": "one", "attack": 11},
            {"id": 6, "name": "V (Charged)", "skill": "axe", "hands": "one", "attack": 12},
        ]
        oteryn = {
            "a": [("k:a", "AXE", 10)],
            "b": [("k:b", "AXE", 99)],
            "c": [("k:c", "AXE", 20)],
            "dup": [("k:d", "AXE", 11)],
            "v (charged)": [("k:v", "AXE", 12)],
        }
        chosen, missing = tool.select_weapons(catalogue, oteryn)
        picked = {(w["class"], w["attack_residue_mod_5"]): w["tibiatools_id"] for w in chosen}
        # residue 0: B differs in attack, A matches but is two-handed, C matches and is one-handed
        self.assertEqual(picked[("axe", 0)], 3)
        self.assertNotIn(("axe", 1), picked)
        self.assertNotIn(("axe", 2), picked)
        reasons = {(m[0], m[1]) for m in missing}
        self.assertIn(("axe", 1), reasons)
        self.assertIn(("club", 0), reasons)


if __name__ == "__main__":
    unittest.main()
