"""QB-1: outfit hand-in NPCs are written at their single agreeing promotion-candidate cell."""
import json
import unittest

import convert_placements as cp

QB_CELLS = {
    "oteryn:npc.angelina": (32633, 32402, 10),
    "oteryn:npc.atrad": (32077, 32533, 9),
    "oteryn:npc.erayo": (32518, 32911, 8),
    "oteryn:npc.hugo": (32950, 32102, 5),
    "oteryn:npc.legola": (32298, 31785, 7),
    "oteryn:npc.lugri": (32389, 32118, 8),
    "oteryn:npc.lynda": (32331, 32199, 7),
    "oteryn:npc.myra": (32580, 32751, 6),
    "oteryn:npc.sandra": (33257, 31839, 7),
    "oteryn:npc.simon_the_beggar": (32186, 32471, 7),
    "oteryn:npc.the_queen_of_the_banshees": (32260, 31866, 14),
    "oteryn:npc.vescu": (32565, 32651, 7),
    "oteryn:npc.zoltan": (33266, 31847, 4),
}


class QbOutfitPlacementTest(unittest.TestCase):
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
        self.assertEqual(contacts, {"oteryn:npc.vescu"})


if __name__ == "__main__":
    unittest.main()
