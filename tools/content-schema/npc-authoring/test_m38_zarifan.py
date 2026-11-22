import json
import unittest
import convert_placements as cp


class ZarifanContactTest(unittest.TestCase):
    def test_source_contact_keeps_its_anchor_and_walking_profile(self):
        files = cp.convert()
        records = [r["declaration"] for name, text in files.items()
                   if name.startswith("placements-")
                   for r in json.loads(text)["records"]
                   if r["declaration"]["npc"] == "oteryn:npc.zarifan"]
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["cell"], {"x": 33040, "y": 32484, "floor": 11})
        self.assertTrue(records[0]["block_solid_contact"])
        # The generated catalogue must contain exactly the reviewed solid contacts.
        self.assertEqual({r["declaration"]["npc"]
                          for name, text in files.items() if name.startswith("placements-")
                          for r in json.loads(text)["records"]
                          if r["declaration"].get("block_solid_contact", False)},
                         {f"oteryn:npc.{key}" for key in (
                             "bron", "doctor_marrow", "ellipses", "gnomargery", "gnombold",
                             "gnomilly", "gnommander", "kepar", "palimuth", "rafzan",
                             "raymond_striker", "sinatuki", "the_first_dragon",
                             "wyrdins_apprentice", "zalamon", "zarifan")})
        candidate = next(c for c in json.loads(cp.CANDIDATES.read_text())["candidates"]
                         if c["identity"]["key"] == "oteryn:npc.zarifan")
        self.assertEqual(candidate["movement"]["walk_interval_ms"], 2000)
        self.assertEqual(candidate["movement"]["walk_radius"], 2)
        self.assertFalse(candidate["movement"]["floor_change"])
        self.assertTrue(all(p["definition_sha256"] in
                            {v["sha256"] for v in candidate["provenance"].values()}
                            for p in records[0]["provenance"]))
