"""QB Barbarian Outfits: Bron and Ajax are written at their single agreeing promotion-candidate cell."""
import json
import unittest

import convert_placements as cp

QB_CELLS = {
    "oteryn:npc.ajax": (32417, 31583, 7),
    "oteryn:npc.bron": (32366, 31628, 7),
}


class QbBarbarianPlacementTest(unittest.TestCase):
    def test_hand_ins_are_written_once_at_their_source_cell(self):
        files = cp.convert()
        records = [r["declaration"] for name, text in files.items()
                   if name.startswith("placements-")
                   for r in json.loads(text)["records"]
                   if r["declaration"]["npc"] in QB_CELLS]
        cells = {r["npc"]: (r["cell"]["x"], r["cell"]["y"], r["cell"]["floor"]) for r in records}
        self.assertEqual(len(records), len(QB_CELLS))
        self.assertEqual(cells, QB_CELLS)
        contacts = {r["npc"] for r in records if r.get("block_solid_contact", False)}
        self.assertEqual(contacts, {"oteryn:npc.bron"})


if __name__ == "__main__":
    unittest.main()
