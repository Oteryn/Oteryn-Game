"""Full-anchor support retains both Item custody and every stable palette slot."""

import copy
import json
import unittest

import npcfix_extended_ground as ground
import world_objects


class ExtendedGroundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = ground.records()
        cls.index = json.loads((ground.ROOT / "content/world/placements/index.json").read_bytes())

    def test_exact_bound_and_appearance_only_source_facts(self):
        self.assertEqual([r["provenance"]["source_item_id"] for r in self.rows], [32691, 50820])
        for row, speed, color in zip(self.rows, [150, 100], [51, 129]):
            self.assertEqual(row["ground_speed"], {"state": "KNOWN", "value": speed})
            self.assertEqual(row["walkable"], {"state": "KNOWN", "value": True})
            self.assertEqual(row["automap"], {"state": "KNOWN", "value": {"shown": True, "color": color}})
            self.assertEqual(world_objects.validate_record(row), [])
        self.assertEqual(self.rows[0]["provenance"]["item_pointer"]["key"], "oteryn:item.tibia.i32691")
        self.assertNotIn("item_pointer", self.rows[1]["provenance"])

    def test_all_palette_positions_and_unrelated_rows_remain_identical(self):
        saved = copy.deepcopy(self.index)
        after = ground.retarget_palette(self.index)
        self.assertEqual(self.index, saved)
        self.assertEqual(len(after["palette"]), len(saved["palette"]))
        for old, new in zip(saved["palette"], after["palette"]):
            self.assertEqual(old["source_item_id"], new["source_item_id"])
            if old["source_item_id"] not in ground.IDS:
                self.assertEqual(old, new)
            else:
                self.assertEqual(new, {"key": f"oteryn:terrain.tibia.i{old['source_item_id']}",
                                       "source_item_id": old["source_item_id"], "provisional": False})
        self.assertEqual(ground.retarget_palette(after), after)

    def test_late_drift_cannot_partially_mutate_the_input(self):
        changed = copy.deepcopy(self.index)
        next(r for r in changed["palette"] if r["source_item_id"] == 50820)["key"] = "oteryn:terrain.other_owner"
        saved = copy.deepcopy(changed)
        with self.assertRaisesRegex(ValueError, "palette before drift"):
            ground.retarget_palette(changed)
        self.assertEqual(changed, saved)

    def test_duplicates_and_missing_palette_identity_are_rejected(self):
        for duplicate in [False, True]:
            changed = copy.deepcopy(self.index)
            row = next(r for r in changed["palette"] if r["source_item_id"] == 32691)
            if duplicate:
                changed["palette"].append(copy.deepcopy(row))
            else:
                changed["palette"].remove(row)
            with self.assertRaisesRegex(ValueError, "palette before drift"):
                ground.retarget_palette(changed)

    def test_changed_source_speed_or_appearance_is_rejected(self):
        for index in [0, 1]:
            changed = copy.deepcopy(self.rows[index])
            changed["ground_speed"]["value"] += 1
            self.assertTrue(world_objects.validate_record(changed))
            changed = copy.deepcopy(self.rows[index])
            changed["client_projection"]["value"]["appearance_id"] += 1
            self.assertTrue(world_objects.validate_record(changed))


if __name__ == "__main__":
    unittest.main()
