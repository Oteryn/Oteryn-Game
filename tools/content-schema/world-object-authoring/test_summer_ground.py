"""The admitted ground must retain its actual identity and official facts."""

import copy
import json
import unittest

import summer_ground
import world_objects


class SummerGroundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = summer_ground.outputs()
        cls.record = json.loads(cls.files[summer_ground.SHARD])["records"][0]

    def test_exact_identity_and_stable_palette_slot(self):
        original = json.loads(
            (summer_ground.ROOT / "content/world/placements/index.json").read_bytes()
        )["palette"]
        palette = json.loads(self.files["content/world/placements/index.json"])[
            "palette"
        ]
        self.assertEqual(
            [row["source_item_id"] for row in palette],
            [row["source_item_id"] for row in original],
        )
        ground = next(row for row in palette if row["source_item_id"] == 44576)
        self.assertEqual(
            ground,
            {"key": summer_ground.KEY, "provisional": False, "source_item_id": 44576},
        )
        self.assertNotIn("item_pointer", self.record["provenance"])
        self.assertEqual(self.record["ground_speed"], {"state": "KNOWN", "value": 150})

    def test_generated_files_are_current(self):
        for name, raw in self.files.items():
            self.assertEqual((summer_ground.ROOT / name).read_bytes(), raw, name)

    def test_source_facts_and_identity_cannot_be_substituted(self):
        self.assertEqual(world_objects.validate_record(self.record), [])
        for field, value in [
            ("ground_speed", {"state": "KNOWN", "value": 100}),
            (
                "identity",
                {
                    "family": "Terrain",
                    "key": "oteryn:terrain.tibia.i44577",
                    "revision": "definition-r1",
                },
            ),
        ]:
            altered = copy.deepcopy(self.record)
            altered[field] = value
            self.assertTrue(world_objects.validate_record(altered), field)


if __name__ == "__main__":
    unittest.main()
