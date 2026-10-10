"""Tests for convert_placements.py: the Npc.Placement family is the scoped promotion candidates."""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import convert_placements as cp  # noqa: E402


class ConvertPlacementsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = cp.convert()
        cls.index = json.loads(cls.files["index.json"])
        cls.held = json.loads(cls.files["held.json"])["held"]
        cls.records = [
            r
            for name, text in sorted(cls.files.items())
            if name.startswith("placements-")
            for r in json.loads(text)["records"]
        ]

    def test_committed_family_is_the_conversion(self):
        for name, text in self.files.items():
            self.assertEqual((cp.OUT / name).read_text(), text, name)
        self.assertEqual({p.name for p in cp.OUT.glob("*.json")}, set(self.files))

    def test_only_scoped_admitted_npcs(self):
        scope = set(json.loads(cp.SCOPE.read_text())["npcs"])
        self.assertEqual({r["declaration"]["npc"] for r in self.records}, scope - {h["npc"] for h in self.held})
        self.assertLessEqual(scope, cp.admitted_keys())

    def test_records_sorted_unique_and_keyed_by_cell(self):
        keys = [r["declaration"]["identity"]["key"] for r in self.records]
        self.assertEqual(keys, sorted(set(keys)))
        for r in self.records:
            d = r["declaration"]
            c = d["cell"]
            self.assertEqual(
                d["identity"]["key"],
                f"oteryn:npc_placement.{d['npc'].removeprefix('oteryn:npc.')}.x{c['x']}_y{c['y']}_z{c['floor']}",
            )
            self.assertTrue(d["provenance"])

    def test_wiki_placement_is_north_and_says_so(self):
        zirella = next(r for r in self.records if r["declaration"]["npc"] == "oteryn:npc.zirella")["declaration"]
        self.assertEqual(zirella["direction"], "north")
        self.assertEqual(zirella["provenance"][0]["origin"], "wiki")
        self.assertIn("north", zirella["provenance"][0]["direction_note"])

    def test_no_cell_is_shared(self):
        cells = [(r["declaration"]["cell"]["x"], r["declaration"]["cell"]["y"], r["declaration"]["cell"]["floor"]) for r in self.records]
        self.assertEqual(len(cells), len(set(cells)))

    def test_index_counts_and_held_digest(self):
        self.assertEqual(self.index["record_count"], len(self.records))
        self.assertEqual(self.index["held"]["count"], len(self.held))
        import hashlib

        self.assertEqual(self.index["held"]["sha256"], hashlib.sha256(self.files["held.json"].encode()).hexdigest())
        self.assertTrue(all(h["reason"] in cp.REASONS for h in self.held))


if __name__ == "__main__":
    unittest.main()
