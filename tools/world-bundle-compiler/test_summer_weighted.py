"""Closed source group qualification preserves every unrelated spawn shard."""

import copy
import json
import unittest
from unittest.mock import patch

import convert_spawns
import summer_weighted


class SummerWeightedTest(unittest.TestCase):
    def test_source_three_children_are_one_weighted_slot_and_one_south_slot(self):
        row = summer_weighted.record(json.loads(summer_weighted.INPUT.read_bytes()))[
            "declaration"
        ]
        self.assertEqual(len(row["points"]), 2)
        main, south = row["points"]
        self.assertEqual(main["respawn_ms"], 47_104_000)
        self.assertEqual(south["respawn_ms"], 60_000)
        self.assertEqual([r["weight"] for r in main["weighted"]], [25, 25])
        self.assertNotEqual(main["cell"], south["cell"])

    def test_overlay_preserves_every_existing_scalar_source_byte(self):
        xml = convert_spawns.XML.read_bytes()
        with patch.object(summer_weighted, "augment", side_effect=lambda files: files):
            base = convert_spawns.convert(xml)
        augmented = summer_weighted.augment(copy.deepcopy(base))
        for name, text in base.items():
            if name != "index.json":
                self.assertEqual(augmented[name], text, name)
        old, new = (json.loads(files["index.json"]) for files in (base, augmented))
        self.assertEqual(new["record_count"], old["record_count"] + 1)
        self.assertEqual(new["point_count"], old["point_count"] + 2)
        self.assertEqual(summer_weighted.augment(copy.deepcopy(augmented)), augmented)

    def test_unreviewed_source_edit_is_rejected(self):
        value = json.loads(summer_weighted.INPUT.read_bytes())
        for mutate in [
            lambda row: row["points"][0].update(weight="0"),
            lambda row: row["centre"].update(x=33865),
            lambda row: row.update(revision="unreviewed"),
        ]:
            bad = copy.deepcopy(value)
            mutate(bad)
            with self.assertRaises(ValueError):
                summer_weighted.record(bad)


if __name__ == "__main__":
    unittest.main()
