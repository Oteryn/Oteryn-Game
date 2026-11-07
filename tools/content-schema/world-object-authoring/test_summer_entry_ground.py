"""Summer entry is walkable only through its qualified official ground identity."""

import copy
import importlib
import json
from pathlib import Path
import unittest

import world_objects

ROOT = Path(__file__).resolve().parents[3]


class SummerEntryGroundTests(unittest.TestCase):
    def test_summer_teleport_target_has_admitted_ground(self):
        index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
        row = next(r for r in index["palette"] if r["source_item_id"] == 52077)
        self.assertEqual(
            row,
            {"key": "oteryn:terrain.tibia.i52077", "provisional": False, "source_item_id": 52077},
        )

    def test_generated_source_facts_and_identity_are_exact(self):
        ground = importlib.import_module("summer_entry_ground")
        files = ground.outputs()
        records = json.loads(files[ground.SHARD])["records"]
        self.assertEqual(len(records), 115)
        record = next(r for r in records if r["provenance"]["source_item_id"] == 52077)
        self.assertEqual(record["identity"]["key"], "oteryn:terrain.tibia.i52077")
        self.assertNotIn("item_pointer", record["provenance"])
        self.assertEqual(record["ground_speed"], {"state": "KNOWN", "value": 100})
        self.assertEqual(record["walkable"], {"state": "KNOWN", "value": True})
        for candidate in records:
            with self.subTest(appearance=candidate["provenance"]["source_item_id"]):
                self.assertEqual(world_objects.validate_record(candidate), [])
        for field, value in [
            ("ground_speed", {"state": "KNOWN", "value": 150}),
            ("walkable", {"state": "KNOWN", "value": False}),
        ]:
            altered = copy.deepcopy(record)
            altered[field] = value
            self.assertTrue(world_objects.validate_record(altered), field)
        for name, raw in files.items():
            self.assertEqual((ROOT / name).read_bytes(), raw, name)


if __name__ == "__main__":
    unittest.main()
