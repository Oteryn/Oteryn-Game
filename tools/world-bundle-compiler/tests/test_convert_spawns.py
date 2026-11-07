"""Tests for convert_spawns.py: the canonical spawn family is the pinned CrystalServer file minus
the held groups (SPAWN-ADMIT-1)."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

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
        self.assertEqual(len(self.records), 54684)
        self.assertEqual(sum(len(r["declaration"]["points"]) for r in self.records), 87812)
        self.assertEqual(self.index["record_count"], 54684)
        self.assertEqual(self.index["point_count"], 87812)

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
        self.assertEqual(len(held), 42)
        self.assertEqual(self.index["held_bosses"]["point_count"], 42)
        self.assertTrue(all("boss_policy" not in row["point"] for row in held))

    def test_other_hash_fails_closed(self):
        with self.assertRaises(SystemExit):
            cs.convert(b"<monsters/>")

    def test_held_world_bosses_are_scheduled_except_encounter_bound(self):
        held = json.loads(self.files["held-bosses.json"])["records"]
        self.assertTrue(all(row["reason"] == "encounter-bound" for row in held))
        scheduled = [r["declaration"] for r in self.records
                     if r["declaration"].get("provenance", {}).get("kind")
                     == "CHOSEN_OTERYN_APPROXIMATION"]
        self.assertTrue(scheduled)
        for declaration in scheduled:
            for point in declaration["points"]:
                self.assertEqual(point["boss_policy"], {"at_reset": {"chance": {"numerator": 1, "denominator": 50}}})
        self.assertTrue(any(row["point"]["creature"] == "oteryn:creature.gaffir" for row in held))

    def test_world_boss_chance_comes_from_per_boss_configuration(self):
        configured = json.loads(cs.BOSS_CHANCES.read_text())
        configured["bosses"]["oteryn:creature.heoni"]["at_reset"]["chance"]["numerator"] = 0
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "chances.json"
            path.write_text(json.dumps(configured))
            with patch.object(cs, "BOSS_CHANCES", path):
                files = cs.convert(cs.XML.read_bytes())
        points = [point for name, text in files.items() if name.startswith("spawns-")
                  for row in json.loads(text)["records"] for point in row["declaration"]["points"]
                  if point["creature"] == "oteryn:creature.heoni"]
        self.assertEqual(len(points), 1)
        self.assertEqual(points[0]["boss_policy"]["at_reset"]["chance"], {"numerator": 0, "denominator": 50})


if __name__ == "__main__":
    unittest.main()
