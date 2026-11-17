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
        self.assertFalse(any(r["declaration"].get("block_solid_contact", False)
                             for name, text in files.items() if name.startswith("placements-")
                             for r in json.loads(text)["records"]
                             if r["declaration"]["npc"] != "oteryn:npc.zarifan"))
        candidate = next(c for c in json.loads(cp.CANDIDATES.read_text())["candidates"]
                         if c["identity"]["key"] == "oteryn:npc.zarifan")
        self.assertEqual(candidate["movement"]["walk_interval_ms"], 2000)
        self.assertEqual(candidate["movement"]["walk_radius"], 2)
        self.assertFalse(candidate["movement"]["floor_change"])
        self.assertTrue(all(p["definition_sha256"] in
                            {v["sha256"] for v in candidate["provenance"].values()}
                            for p in records[0]["provenance"]))
