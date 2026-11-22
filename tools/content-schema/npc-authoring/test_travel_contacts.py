import json
import unittest

import convert_placements


class TravelContactsTest(unittest.TestCase):
    def test_crystal_ferry_origins_are_published(self):
        expected = {
            "captain_jack": (32190, 31957, 6),
            "eremo": (33327, 31882, 7),
            "ferryman_kamil": (32152, 32457, 7),
            "ferryman_kamil_meluna": (32088, 32469, 7),
            "harlow_vengoth": (32856, 31549, 7),
            "hawkhurst": (33356, 31982, 7),
            "hawkhurst_ingol": (33711, 32602, 6),
            "kendra": (33494, 31057, 7),
            "tonar": (33181, 32883, 7),
        }
        placements = [
            row["declaration"]
            for name, data in convert_placements.convert().items()
            if name.startswith("placements-")
            for row in json.loads(data)["records"]
        ]
        for npc, (x, y, floor) in expected.items():
            with self.subTest(npc=npc):
                self.assertEqual(
                    [p["cell"] for p in placements if p["npc"] == f"oteryn:npc.{npc}"],
                    [{"x": x, "y": y, "floor": floor}],
                )
