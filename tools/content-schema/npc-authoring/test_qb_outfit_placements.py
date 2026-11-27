"""QB-1: outfit hand-in NPCs are written at their single agreeing promotion-candidate cell."""
import json
import unittest

import convert_placements as cp

QB_CELLS = {
    "oteryn:npc.angelina": (32633, 32402, 10),
    "oteryn:npc.legola": (32298, 31785, 7),
    "oteryn:npc.lugri": (32389, 32118, 8),
    "oteryn:npc.lynda": (32331, 32199, 7),
    "oteryn:npc.myra": (32580, 32751, 6),
    "oteryn:npc.the_queen_of_the_banshees": (32260, 31866, 14),
    "oteryn:npc.zoltan": (33266, 31847, 4),
    # QB-2
    "oteryn:npc.flickering_soul": (33621, 31419, 10),
    "oteryn:npc.gnommander": (32248, 32604, 8),
    "oteryn:npc.grof_the_guard": (32372, 32182, 7),
    "oteryn:npc.kulag_the_guard": (32287, 32264, 7),
    "oteryn:npc.miles_the_guard": (32429, 32173, 5),
    "oteryn:npc.tim_the_guard": (32425, 32223, 6),
    "oteryn:npc.walter_the_guard": (32338, 32277, 7),
    # QB-3
    "oteryn:npc.cornelia": (32335, 31794, 7),
    "oteryn:npc.hanna": (32408, 32220, 7),
    "oteryn:npc.lubo": (32489, 32119, 7),
    "oteryn:npc.trisha": (32346, 31749, 7),
}


class QbOutfitPlacementTest(unittest.TestCase):
    def test_beggar_and_barbarian_test_contacts_are_at_donor_cells(self):
        files = cp.convert()
        expected = {
            "oteryn:npc.hugo": {"x": 32950, "y": 32102, "floor": 5},
            "oteryn:npc.simon_the_beggar": {"x": 32186, "y": 32471, "floor": 7},
            "oteryn:npc.sven": {"x": 32202, "y": 31153, "floor": 7},
        }
        actual = [
            (p["npc"], p["cell"])
            for name, text in files.items()
            if name.startswith("placements-")
            for row in json.loads(text)["records"]
            if (p := row["declaration"])["npc"] in expected
        ]
        self.assertEqual(sorted(actual), sorted(expected.items()))

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
        self.assertEqual(contacts, {"oteryn:npc.gnommander"})


if __name__ == "__main__":
    unittest.main()
