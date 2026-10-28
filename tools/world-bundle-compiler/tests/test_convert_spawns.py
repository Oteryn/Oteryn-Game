"""Tests for convert_spawns.py: the canonical spawn family is the pinned CrystalServer file minus
the held groups (SPAWN-ADMIT-1)."""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import convert_spawns as cs  # noqa: E402


class ConvertSpawnsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = cs.convert(cs.XML.read_bytes())
        cls.index = json.loads(cls.files["index.json"])
        cls.records = [
            r
            for name, text in sorted(cls.files.items())
            if name.startswith("spawns-")
            for r in json.loads(text)["records"]
        ]
        cls.held = json.loads(cs.HELD.read_text())["records"]

    def test_counts(self):
        self.assertEqual(len(self.records), 54667)
        self.assertEqual(sum(len(r["declaration"]["points"]) for r in self.records), 87795)
        self.assertEqual(self.index["record_count"], 54667)
        self.assertEqual(self.index["point_count"], 87795)

    def test_provenance_is_crystalserver(self):
        self.assertEqual(self.index["source"]["source_key"], "oteryn:source.crystalserver")
        self.assertEqual(self.index["source"]["revision"], "00ce02a57ca5a12e48f32a3476e37471167e4c3f")
        self.assertEqual(self.index["coordinate_frame"], "global-target-2026-09-27")

    def test_held_groups_are_absent(self):
        self.assertEqual(len(self.held), 25)
        self.assertEqual(sum(h["point_count"] for h in self.held), 417)
        keys = {r["declaration"]["identity"]["key"] for r in self.records}
        self.assertFalse(keys & {h["key"] for h in self.held})

    def test_held_input_is_pinned_and_recorded(self):
        self.assertEqual(self.index["held_groups"]["sha256"], cs.HELD_SHA256)
        self.assertEqual(self.index["held_groups"]["group_count"], 25)
        self.assertEqual(self.index["held_groups"]["point_count"], 417)

    def test_canonical_tree_is_the_conversion(self):
        have = {p.name: p.read_text() for p in cs.OUT.glob("*.json")}
        self.assertEqual(have, self.files)

    def test_boss_spawn1_sources_are_provenance_bound_and_have_one_policy(self):
        bosses = json.loads(cs.BOSSES.read_bytes())["records"]
        self.assertEqual(len(bosses), 10)
        for record in bosses:
            declaration = record["declaration"]
            self.assertEqual(declaration["provenance"]["kind"], "canary-pin")
            self.assertEqual(declaration["provenance"]["commit"], "47dfd51f45280a59a1d3e50ba7edd573d7234446")
            for point in declaration["points"]:
                self.assertEqual(len(point["boss_policy"]), 1)
                self.assertNotIn("respawn_ms", point)
        self.assertEqual(self.index["additional_sources"][0]["record_count"], 10)
        held = json.loads(self.files["held-bosses.json"])["records"]
        self.assertEqual(len(held), 59)
        self.assertEqual(self.index["held_bosses"]["point_count"], 59)
        self.assertTrue(all("boss_policy" not in row["point"] for row in held))

    def test_other_hash_fails_closed(self):
        with self.assertRaises(SystemExit):
            cs.convert(b"<monsters/>")


if __name__ == "__main__":
    unittest.main()
