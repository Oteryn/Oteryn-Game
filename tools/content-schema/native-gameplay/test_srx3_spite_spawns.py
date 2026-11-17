"""Retain exact pinned XML evidence, refusing changed or incomplete source."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("spite_spawns", Path(__file__).with_name("build_srx3_spite_spawns.py"))
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class SpiteSourceSpawns(unittest.TestCase):
    def test_retained_source_is_two_ten_second_weeping_souls(self):
        rows = [row for row in json.loads(TOOL.OUTPUT.read_bytes()) if row["creature_key"] == "oteryn:creature.weeping_soul"]
        self.assertEqual([r["position"] for r in rows], [[33738, 31632, 14], [33748, 31634, 14]])
        self.assertEqual([r["line"] for r in rows], [177816, 177836])
        self.assertTrue(all(r["respawn_seconds"] == 10 and r["radius"] == 1 for r in rows))

    def test_retained_source_covers_all_five_literal_private_ambient_spawners(self):
        rows = json.loads(TOOL.OUTPUT.read_bytes())
        rows = [row for row in rows if row["creature_key"] in {"oteryn:creature.weeping_soul", "oteryn:creature.spiteful_spitter", "oteryn:creature.symbol_of_hatred"}]
        self.assertEqual(len(rows), 5)
        by_key = {}
        for row in rows:
            by_key.setdefault(row["creature_key"], []).append((row["position"], row["line"]))
        self.assertEqual(by_key["oteryn:creature.spiteful_spitter"], [([33744, 31636, 14], 177858)])
        self.assertEqual(by_key["oteryn:creature.symbol_of_hatred"], [([33738, 31599, 14], 177650), ([33748, 31599, 14], 177653)])

    def test_retained_source_covers_cruelty_and_megalomania_ambient_rows(self):
        rows = json.loads(TOOL.OUTPUT.read_bytes())
        self.assertEqual(len(rows), 10)
        by_key = {}
        for row in rows:
            by_key.setdefault(row["creature_key"], []).append((row["position"], row["line"]))
        self.assertEqual(by_key["oteryn:creature.poor_soul"], [([33851, 31870, 7], 36787)])
        self.assertEqual(by_key["oteryn:creature.necromantic_focus"], [([33704, 31634, 14], 177830), ([33716, 31634, 14], 177833)])
        self.assertEqual(by_key["oteryn:creature.lesser_splinter_of_madness"], [([33712, 31635, 14], 177848)])

    def test_ragiaz_capsule_preserves_its_separate_sixty_second_source_clock(self):
        rows = [row for row in json.loads(TOOL.OUTPUT.read_bytes()) if row["creature_key"] == "oteryn:creature.bone_capsule"]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["position"], [33485, 32333, 14])
        self.assertEqual((rows[0]["line"], rows[0]["radius"], rows[0]["respawn_seconds"]), (182951, 1, 60))

    def test_changed_xml_is_not_accepted_as_pinned_spawn_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world-monster.xml"
            path.write_bytes(b'<monsters/>')
            with self.assertRaisesRegex(ValueError, "pinned Crystal monster XML changed"):
                TOOL.build(path)


if __name__ == "__main__":
    unittest.main()
