import json
import unittest

import elemental_ground
import world_objects


class ElementalGroundTests(unittest.TestCase):
    def test_earth_altar_retains_item_identity_and_official_ground_facts(self):
        data = elemental_ground.output()
        record = json.loads(data)["records"][0]
        self.assertEqual(world_objects.validate_record(record), [])
        self.assertEqual(
            record["provenance"]["item_pointer"]["key"], "oteryn:item.tibia.i7748"
        )
        self.assertEqual(record["ground_speed"], {"state": "KNOWN", "value": 150})
        self.assertEqual(record["walkable"], {"state": "KNOWN", "value": True})
        self.assertEqual(
            (elemental_ground.ROOT / elemental_ground.SHARD).read_bytes(), data
        )
