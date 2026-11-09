"""Validate authored M30b chamber identities against the actual World payload."""

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec as codec


class AsuraChambers(unittest.TestCase):
    def test_authored_rosters_have_two_distinct_native_tiles_and_exact_doors(self):
        document = json.loads(
            (Path(__file__).parent / "samples/engenc/asura-chambers.json").read_text()
        )
        self.assertEqual(document["basis"], "authored per owner rule")
        self.assertEqual([c["stage"] for c in document["chambers"]], ["s2", "s3", "s4"])
        palette = json.loads(
            (ROOT / "content/world/placements/index.json").read_text()
        )["palette"]
        cache = {}

        def tile(position):
            x, y, z = position
            if z not in cache:
                data = (
                    ROOT / f"content/world/placements/region-z{z:02}-x128-y127.b3"
                ).read_bytes()
                cache[z] = {
                    (t[0], t[1]): t
                    for _, sector in codec.decode_region(data)[3]
                    for t in sector
                }
            return cache[z][x, y]

        for chamber in document["chambers"]:
            with self.subTest(stage=chamber["stage"]):
                self.assertEqual(len({tuple(p) for p in chamber["defenders"]}), 2)
                lo, hi = chamber["room_min"], chamber["room_max"]
                for position in chamber["defenders"] + [chamber["landing"]]:
                    self.assertTrue(
                        all(a <= p <= b for a, p, b in zip(lo, position, hi))
                    )
                    self.assertTrue(tile(position)[5])
                self.assertTrue(tile(chamber["entry"])[5])
                door = tile(chamber["door"])[5]
                self.assertEqual(
                    palette[door[1][0]]["source_item_id"], chamber["door_appearance"]
                )
                self.assertEqual(chamber["door"][2], lo[2])


if __name__ == "__main__":
    unittest.main()
