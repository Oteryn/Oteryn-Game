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
    # QB-2
    "oteryn:npc.a_majestic_warwolf": (33355, 31991, 8),
    "oteryn:npc.budrik": (32525, 31906, 8),
    "oteryn:npc.ceiron": (32665, 31717, 7),
    "oteryn:npc.eruaran": (33618, 32372, 5),
    "oteryn:npc.flickering_soul": (33621, 31419, 10),
    "oteryn:npc.gnomally": (32795, 31746, 10),
    "oteryn:npc.gnommander": (32248, 32604, 8),
    "oteryn:npc.gregor": (32406, 32201, 6),
    "oteryn:npc.grof_the_guard": (32372, 32182, 7),
    "oteryn:npc.kulag_the_guard": (32287, 32264, 7),
    "oteryn:npc.miles_the_guard": (32429, 32173, 5),
    "oteryn:npc.the_bone_master": (32793, 32225, 14),
    "oteryn:npc.the_dream_master": (32844, 32228, 14),
    "oteryn:npc.tim_the_guard": (32425, 32223, 6),
    "oteryn:npc.ustan": (32580, 32757, 6),
    "oteryn:npc.vanys": (33685, 32180, 8),
    "oteryn:npc.walter_the_guard": (32338, 32277, 7),
    # QB-3
    "oteryn:npc.cornelia": (32335, 31794, 7),
    "oteryn:npc.hanna": (32408, 32220, 7),
    "oteryn:npc.lubo": (32489, 32119, 7),
    "oteryn:npc.trisha": (32346, 31749, 7),
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
        self.assertEqual(contacts, {"oteryn:npc.gnommander", "oteryn:npc.vescu"})


if __name__ == "__main__":
    unittest.main()
