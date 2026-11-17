import json
import unittest
from pathlib import Path

import qready2_chest_bindings as qready2
import quest_access_bindings as access
import world_region_codec as codec


class Qready2ChestBindingsTest(unittest.TestCase):
    def test_capture_matches_owned_canonical_rewards(self):
        self.assertEqual(
            (qready2.ROOT / qready2.CAPTURE).read_bytes(), qready2.capture()
        )
        rows = json.loads(qready2.capture())["bindings"]
        self.assertEqual(24, len(rows))
        self.assertFalse(any("knight_outfits" in row["claim"] for row in rows))
        self.assertEqual(3, sum("purpletome" in row["claim"] for row in rows))

    def test_every_runtime_tile_has_exact_source_uid_and_appearance(self):
        index = json.loads((qready2.ROOT / access.INDEX).read_bytes())
        cache = {}
        for row in json.loads(qready2.capture())["bindings"]:
            x, y, z = row["position"]
            path = Path(
                f"content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
            )
            if path not in cache:
                cache[path] = {
                    (tile[0], tile[1]): tile
                    for _, tiles in codec.decode_region(
                        (qready2.ROOT / path).read_bytes()
                    )[3]
                    for tile in tiles
                }
            tile = cache[path][(x, y)]
            matches = [
                (index["palette"][i]["source_item_id"], depth)
                for i, depth, attrs in tile[5]
                if attrs and attrs.get("unique") == row["value"]
            ]
            self.assertEqual([(row["item"], 0)], matches, row["claim"])
