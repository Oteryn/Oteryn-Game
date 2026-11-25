"""Coordinate regressions for Dawnport's source ground and north staircase."""

import json
import unittest
from pathlib import Path

from world_region_codec import decode_region

ROOT = Path(__file__).resolve().parents[3]
WORLD = ROOT / "content/world"


class DawnportGround(unittest.TestCase):
    def test_runtime_static_surface_ids_match_compiler_catalogue_order(self):
        registry = json.loads((WORLD / "source-static-definitions.json").read_bytes())
        terrain_keys = {
            record["identity"]["key"]
            for shard in (WORLD / "terrain").glob("terrain-*.json")
            for record in json.loads(shard.read_bytes())["records"]
        }
        terrain_keys.update(record["identity"]["key"] for record in registry["records"])
        self.assertEqual(registry["technical_terrain_keys"], sorted(terrain_keys))

    def test_donor_ground_survives_palette_projection(self):
        index = json.loads((WORLD / "placements/index.json").read_bytes())
        for x, y, z, source_ids in [
            (31934, 31925, 7, [53066, 19394]),
            (32065, 31894, 6, [21346]),
            (32065, 31893, 6, [21346, 7705]),
            (32065, 31889, 6, [21346, 1947]),
            (32065, 31888, 5, [416, 19946]),
        ]:
            with self.subTest(cell=(x, y, z)):
                region = WORLD / f"placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
                _, _, _, sectors = decode_region(region.read_bytes())
                items = next(
                    items
                    for _, tiles in sectors
                    for tx, ty, _, _, _, items in tiles
                    if (tx, ty) == (x, y)
                )
                rows = [index["palette"][slot] for slot, depth, _ in items if depth == 0]
                self.assertEqual([row["source_item_id"] for row in rows], source_ids)
                self.assertTrue(all(not row["provisional"] for row in rows))
                if source_ids[0] == 53066:
                    self.assertEqual(rows[0]["key"], "oteryn:terrain.tibia.i53066")


if __name__ == "__main__":
    unittest.main()
