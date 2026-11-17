"""QB Shaman Outfits: Hairycles and Chondur are written at their single agreeing promotion-candidate cell."""
import json
import unittest

import convert_placements as cp

QB_CELLS = {
    "oteryn:npc.chondur": (32361, 32550, 7),
    "oteryn:npc.hairycles": (32826, 32574, 6),
}


class QbShamanPlacementTest(unittest.TestCase):
    def test_contacts_are_written_once_at_their_source_cell(self):
        files = cp.convert()
        records = [r["declaration"] for name, text in files.items()
                   if name.startswith("placements-")
                   for r in json.loads(text)["records"]
                   if r["declaration"]["npc"] in QB_CELLS]
        cells = {r["npc"]: (r["cell"]["x"], r["cell"]["y"], r["cell"]["floor"]) for r in records}
        self.assertEqual(len(records), len(QB_CELLS))
        self.assertEqual(cells, QB_CELLS)
        self.assertFalse(any(r.get("block_solid_contact", False) for r in records))


if __name__ == "__main__":
    unittest.main()
