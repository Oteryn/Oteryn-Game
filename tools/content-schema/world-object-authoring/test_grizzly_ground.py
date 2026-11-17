"""The admitted Grizzly ground must retain its official collision facts."""

import copy
import json
import unittest

import grizzly_ground
import world_objects


class GrizzlyGroundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = grizzly_ground.outputs()
        cls.record = json.loads(cls.files[grizzly_ground.SHARD])["records"][0]

    def test_ground_identity_and_facts(self):
        self.assertEqual(world_objects.validate_record(self.record), [])
        self.assertEqual(self.record["identity"]["key"], grizzly_ground.KEY)
        self.assertEqual(self.record["kind"], {"state": "KNOWN", "value": "ground"})
        self.assertEqual(self.record["walkable"], {"state": "KNOWN", "value": True})
        self.assertEqual(self.record["ground_speed"], {"state": "KNOWN", "value": 100})
        self.assertNotIn("item_pointer", self.record["provenance"])

    def test_generated_files_are_current(self):
        for name, raw in self.files.items():
            self.assertEqual((grizzly_ground.ROOT / name).read_bytes(), raw, name)

    def test_ground_collision_must_match_official_source(self):
        record = copy.deepcopy(self.record)
        record["walkable"]["value"] = False
        self.assertEqual(
            world_objects.validate_record(record),
            ["appearance-only Terrain differs from its qualified official source"],
        )


if __name__ == "__main__":
    unittest.main()
