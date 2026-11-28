"""Validate authored M30b chamber identities against the actual World payload."""

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec as codec


class AsuraChambers(unittest.TestCase):
    def test_orchid_encounter_and_treasures_use_native_placements(self):
        from engenc_asura_entry import entry_binding

        (quest,) = entry_binding()["quests"]
        stages = {row["key"]: row for row in quest["stages"]}
        entry = stages["orchid-entry"]["trigger"]
        self.assertEqual(entry["kind"], "map_use")
        callback = entry["callback"]
        self.assertEqual(
            callback["selector"]["native"]["position"],
            {"x": 32959, "y": 32679, "floor": -2},
        )
        self.assertEqual(
            callback["effects"]["encounter"]["monsters"],
            [
                {
                    "position": {"x": 32955, "y": 32679, "floor": -2},
                    "creature": "oteryn:creature.the_flaming_orchid",
                }
            ],
        )
        self.assertEqual(
            callback["storage"][0]["track"],
            "oteryn:quest-progress/engenc/asura/s4/opened",
        )
        counter = stages["orchid-kill"]["trigger"]["counter"]
        self.assertEqual(counter["encounter_source"], callback["key"])
        self.assertEqual(counter["maximum"], 1)
        for name, item, entrance in (("sapphire", 3029, 2472), ("ruby", 3030, 1993)):
            claim = stages[f"orchid-{name}"]["trigger"]["callback"]
            self.assertEqual(claim["selector"]["native"]["appearance"], entrance)
            self.assertEqual(claim["storage"], [{"track": f"oteryn:quest-progress/engenc/asura/orchid/{name}", "minimum": 0, "maximum": 0, "set": 1}])
            self.assertEqual(
                claim["effects"]["item_reward"],
                {
                    "item": f"oteryn:item.tibia.i{item}",
                    "count": 3,
                },
            )
        self.assertTrue(stages["orchid-ruby"]["completes"])
        door = stages["orchid-library-door"]["trigger"]["callback"]
        self.assertEqual(
            door["selector"]["native"],
            {
                "position": {"x": 32962, "y": 32674, "floor": -2},
                "ordinal": 1,
                "appearance": 1676,
            },
        )
        self.assertEqual(door["storage"][0]["track"], counter["track"])
        self.assertEqual(door["minimum_level"], 250)
        self.assertEqual(door["teleport"], {"x": 32962, "y": 32673, "floor": -2})

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
                self.assertTrue(tile(chamber.get("staging", chamber["entry"]))[5])
                door = tile(chamber["door"])[5]
                self.assertEqual(
                    palette[door[1][0]]["source_item_id"], chamber["door_appearance"]
                )
                self.assertEqual(chamber["door"][2], lo[2])


if __name__ == "__main__":
    unittest.main()
