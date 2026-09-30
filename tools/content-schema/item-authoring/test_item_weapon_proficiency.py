"""Tests for item_weapon_proficiency (ITEM-PROF-1). Run with `python test_item_weapon_proficiency.py`."""

import json
import unittest

import item_weapon_proficiency as iwp


class ThresholdClassTests(unittest.TestCase):
    def test_knight_crossbow_standard(self):
        self.assertEqual(iwp.threshold_class("Sword 1H Crimson Sword"), "knight")
        self.assertEqual(iwp.threshold_class("Grand Sanguine 2H Axe"), "knight")
        self.assertEqual(iwp.threshold_class("Replica Carving Club"), "knight")
        self.assertEqual(iwp.threshold_class("Amber 2H Crossbow"), "crossbow")
        self.assertEqual(iwp.threshold_class("Distance 2H Rift Crossbow"), "crossbow")
        self.assertEqual(iwp.threshold_class("Sanguine 2H Bow"), "standard")
        self.assertEqual(iwp.threshold_class("Wand 1H Wand of Decay"), "standard")
        self.assertEqual(iwp.threshold_class("Throw - Small Stone"), "standard")

    def test_threshold_tables_have_nine_increasing_levels(self):
        self.assertEqual(set(iwp.THRESHOLDS), {"standard", "knight", "crossbow"})
        for table in iwp.THRESHOLDS.values():
            self.assertEqual(len(table), 9)
            self.assertEqual(table, sorted(table))


class ArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = iwp.build()

    def test_counts_and_keys(self):
        document = self.document
        self.assertEqual(len(document["profiles"]), iwp.EXPECTED_PROFILES)
        self.assertEqual(len(document["bindings"]), iwp.EXPECTED_BINDINGS)
        keys = [b["item_key"] for b in document["bindings"]]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertTrue(all(k.startswith("oteryn:item.tibia.i") for k in keys))
        ids = {p["proficiency_id"] for p in document["profiles"]}
        self.assertTrue(all(b["proficiency_id"] in ids for b in document["bindings"]))
        self.assertEqual(document["counts"]["referenced_profiles"], 430)

    def test_mastery_is_top_level_plus_two(self):
        for profile in self.document["profiles"]:
            self.assertEqual(profile["top_level"], len(profile["levels"]))
            self.assertEqual(profile["mastery_level"], profile["top_level"] + 2)
            self.assertLessEqual(profile["mastery_level"], 9)

    def test_committed_file_is_current_and_valid_json(self):
        text = iwp.render(self.document)
        self.assertEqual(json.loads(text)["schema"], iwp.SCHEMA)
        self.assertEqual(iwp.DEFAULT_OUTPUT.read_text(encoding="utf-8"), text)


if __name__ == "__main__":
    unittest.main()
