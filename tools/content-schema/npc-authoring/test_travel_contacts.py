import json
import unittest

import convert_placements


class TravelContactsTest(unittest.TestCase):
    def test_crystal_ferry_origins_are_published(self):
        expected = {
            "captain_jack": (32190, 31958, 6),
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
                rows = [p for p in placements if p["npc"] == f"oteryn:npc.{npc}"]
                self.assertEqual(
                    [p["cell"] for p in rows],
                    [{"x": x, "y": y, "floor": floor}],
                )
                donor_y = {"captain_jack": 31957, "ferryman_kamil_meluna": 32468}.get(npc)
                if donor_y is not None:
                    source_cells = [
                        p["source_cell"] for p in rows[0]["provenance"]
                        if p.get("basis") == "CHOSEN_OTERYN_APPROXIMATION"
                    ]
                    self.assertTrue(source_cells)
                    self.assertEqual(source_cells, [{"x": x, "y": donor_y, "floor": floor}]
                                     * len(source_cells))


class PostmanContactsTest(unittest.TestCase):
    def test_noodles_has_a_published_actor(self):
        self.assert_published("noodles")

    def test_olrik_has_a_published_actor(self):
        self.assert_published("olrik")

    def assert_published(self, name):
        key = f"oteryn:npc.{name}"
        candidates = json.loads(convert_placements.CANDIDATES.read_text())["candidates"]
        candidate = next(row for row in candidates if row["identity"]["key"] == key)
        published = [
            row["declaration"]["cell"]
            for filename, data in convert_placements.convert().items()
            if filename.startswith("placements-")
            for row in json.loads(data)["records"]
            if row["declaration"]["npc"] == key
        ]
        source_cells = [row["position"] for row in candidate["placements"]]
        self.assertEqual(
            published,
            [
                {"x": cell["x"], "y": cell["y"], "floor": cell["z"]}
                for cell in source_cells
            ],
        )
