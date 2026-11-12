import json
import unittest

import grizzly_debris_profiles


class GrizzlyDebrisProfileTests(unittest.TestCase):
    def test_closed_walkable_debris_set(self):
        document = json.loads(
            (grizzly_debris_profiles.ROOT / grizzly_debris_profiles.PATH).read_bytes()
        )
        records = document["records"]
        ids = {
            int(row["production_definition"]["production_key"].rsplit("i", 1)[1])
            for row in records
        }
        self.assertEqual(ids, grizzly_debris_profiles.IDS)
        self.assertTrue({6484, 6487}.isdisjoint(ids))
        self.assertTrue(
            all(row["attributes"]["blocks_movement"] is False for row in records)
        )


if __name__ == "__main__":
    unittest.main()
