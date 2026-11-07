"""Reviewed NPC ground preserves source custody and the stable palette."""

import copy
import json
import unittest

import npcfix_ground
import summer_ground
import world_objects


class NpcfixGroundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = npcfix_ground.records()
        cls.index = json.loads(
            (npcfix_ground.ROOT / "content/world/placements/index.json").read_bytes()
        )

    def test_exact_official_facts_without_an_item_alias(self):
        self.assertEqual(
            [row["provenance"]["source_item_id"] for row in self.rows],
            sorted(npcfix_ground.IDS),
        )
        for row in self.rows:
            identifier = row["provenance"]["source_item_id"]
            self.assertEqual(row["identity"]["key"], f"oteryn:terrain.tibia.i{identifier}")
            self.assertEqual(row["ground_speed"], {"state": "KNOWN", "value": 150})
            self.assertEqual(row["walkable"], {"state": "KNOWN", "value": True})
            self.assertNotIn("item_pointer", row["provenance"])
            self.assertEqual(world_objects.validate_record(row), [])

    def test_stable_palette_and_unrelated_rows_are_preserved(self):
        before = copy.deepcopy(self.index)
        after = npcfix_ground.retarget_palette(self.index, self.rows)
        self.assertEqual(self.index, before)
        self.assertEqual(len(after["palette"]), len(before["palette"]))
        for old, new in zip(before["palette"], after["palette"]):
            self.assertEqual(old["source_item_id"], new["source_item_id"])
            if old["source_item_id"] not in npcfix_ground.IDS:
                self.assertEqual(old, new)
            else:
                self.assertEqual(new["key"], f"oteryn:terrain.tibia.i{old['source_item_id']}")
                self.assertFalse(new["provisional"])

    def test_late_palette_drift_cannot_partially_change_the_input(self):
        altered = copy.deepcopy(self.index)
        target = next(row for row in altered["palette"] if row["source_item_id"] == 53254)
        target.update(key="oteryn:terrain.someone_else", provisional=False)
        saved = copy.deepcopy(altered)
        with self.assertRaisesRegex(ValueError, "palette before drift"):
            npcfix_ground.retarget_palette(altered, self.rows)
        self.assertEqual(altered, saved)

    def test_changed_source_facts_are_rejected(self):
        for field, value in [
            ("walkable", {"state": "KNOWN", "value": False}),
            ("ground_speed", {"state": "KNOWN", "value": 100}),
        ]:
            altered = copy.deepcopy(self.rows[0])
            altered[field] = value
            self.assertTrue(world_objects.validate_record(altered), field)

    def test_existing_item_binding_guard_remains_strict(self):
        with self.assertRaisesRegex(ValueError, "unexpectedly has an Item binding"):
            summer_ground.outputs(identifier=32691, expected_speed=150)

    def test_generated_files_are_current(self):
        for name, raw in npcfix_ground.outputs().items():
            self.assertEqual((npcfix_ground.ROOT / name).read_bytes(), raw, name)


if __name__ == "__main__":
    unittest.main()
