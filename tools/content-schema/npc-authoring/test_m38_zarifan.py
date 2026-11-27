import json
import unittest
import convert_placements as cp


class ZarifanContactTest(unittest.TestCase):
    def test_source_contact_keeps_its_anchor_and_walking_profile(self):
        files = cp.convert()
        records = [
            r["declaration"]
            for name, text in files.items()
            if name.startswith("placements-")
            for r in json.loads(text)["records"]
            if r["declaration"]["npc"] == "oteryn:npc.zarifan"
        ]
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["cell"], {"x": 33040, "y": 32484, "floor": 11})
        self.assertTrue(records[0]["block_solid_contact"])
        # The placement scope and the approved mainland placements both publish
        # solid contacts. The Zarifan placement is one of the scoped contacts.
        scope = json.loads(cp.SCOPE.read_text())
        approved = set(scope["block_solid_contacts"])
        mainland = json.loads((cp.ROOT / cp.CONTACTS).read_text())
        for group in mainland.values():
            for row in group:
                key = f"oteryn:npc.{row['npc']}"
                if row.get("solid_contact"):
                    approved.add(key)
                elif row.get("nearby"):
                    approved.discard(key)
        published = {
            r["declaration"]["npc"]
            for name, text in files.items()
            if name.startswith("placements-")
            for r in json.loads(text)["records"]
        }
        solid = {
            r["declaration"]["npc"]
            for name, text in files.items()
            if name.startswith("placements-")
            for r in json.loads(text)["records"]
            if r["declaration"].get("block_solid_contact", False)
        }
        self.assertLessEqual({
            "oteryn:npc.bron", "oteryn:npc.gnommander", "oteryn:npc.imbul",
            "oteryn:npc.iyad", "oteryn:npc.lorek", "oteryn:npc.old_adall",
            "oteryn:npc.zarifan", "oteryn:npc.ziyad",
        }, solid)
        self.assertEqual(solid, approved & published)
        candidate = next(
            c
            for c in json.loads(cp.CANDIDATES.read_text())["candidates"]
            if c["identity"]["key"] == "oteryn:npc.zarifan"
        )
        self.assertEqual(candidate["movement"]["walk_interval_ms"], 2000)
        self.assertEqual(candidate["movement"]["walk_radius"], 2)
        self.assertFalse(candidate["movement"]["floor_change"])
        self.assertTrue(
            all(
                p["definition_sha256"]
                in {v["sha256"] for v in candidate["provenance"].values()}
                for p in records[0]["provenance"]
            )
        )
