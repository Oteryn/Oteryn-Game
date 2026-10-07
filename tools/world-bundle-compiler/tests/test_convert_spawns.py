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
            if name != "index.json"
            for r in json.loads(text)["records"]
        ]
        cls.held = json.loads(cs.HELD.read_text())["records"]

    def test_counts(self):
        self.assertEqual(len(self.records), 54688)
        self.assertEqual(sum(len(r["declaration"]["points"]) for r in self.records), 87844)
        self.assertEqual(self.index["record_count"], 54688)
        self.assertEqual(self.index["point_count"], 87844)

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

    def test_other_hash_fails_closed(self):
        with self.assertRaises(SystemExit):
            cs.convert(b"<monsters/>")


if __name__ == "__main__":
    unittest.main()
