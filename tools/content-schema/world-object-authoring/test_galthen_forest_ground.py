import json
import unittest

import galthen_forest_ground
import world_objects


class GalthenForestGroundTests(unittest.TestCase):
    def test_portal_ground_retains_its_item_identity_and_walkable_source_speed(self):
        expected = galthen_forest_ground.output()
        shard = galthen_forest_ground.ROOT / galthen_forest_ground.SHARD
        self.assertTrue(shard.is_file(), "the Forest portal needs a Terrain route")
        self.assertEqual(shard.read_bytes(), expected)
        ground = json.loads(expected)["records"][0]
        self.assertEqual(world_objects.validate_record(ground), [])
        self.assertEqual(
            ground["provenance"]["item_pointer"]["key"], "oteryn:item.tibia.i31730"
        )
        self.assertEqual(ground["ground_speed"], {"state": "KNOWN", "value": 500})
        self.assertEqual(ground["walkable"], {"state": "KNOWN", "value": True})
