"""Exact source qualification of the static quest Ram, without Item admission."""

import copy
import json
import unittest

import engnpc2_ram_world as ram
import world_objects


class NativeRamTests(unittest.TestCase):
    def test_source_record_and_stable_palette(self):
        record = ram.record()
        self.assertEqual(json.loads(ram.outputs()[ram.SHARD])["family"], "Terrain")
        self.assertEqual(world_objects.validate_record(record), [])
        self.assertEqual(record["identity"]["family"], "Terrain")
        self.assertEqual(record["walkable"], {"state": "KNOWN", "value": False})
        self.assertEqual(record["kind"], {"state": "KNOWN", "value": "wall"})
        self.assertNotIn("item_pointer", record["provenance"])
        palette = json.loads(ram.outputs()["content/world/placements/index.json"])[
            "palette"
        ]
        self.assertEqual(
            [r for r in palette if r["source_item_id"] == ram.IDENTIFIER],
            [{"source_item_id": ram.IDENTIFIER, "key": ram.KEY, "provisional": False}],
        )

    def test_forged_geometry_and_identity_are_rejected(self):
        record = ram.record()
        for path, value in [
            (("walkable", "value"), True),
            (("kind", "value"), "ground"),
            (("client_projection", "value", "appearance_id"), 53438),
            (("identity", "key"), "oteryn:terrain.tibia.i53438"),
        ]:
            forged = copy.deepcopy(record)
            at = forged
            for key in path[:-1]:
                at = at[key]
            at[path[-1]] = value
            self.assertTrue(world_objects.validate_record(forged), path)

    def test_generated_files_are_current(self):
        for name, raw in ram.outputs().items():
            self.assertEqual((ram.ROOT / name).read_bytes(), raw, name)


if __name__ == "__main__":
    unittest.main()
