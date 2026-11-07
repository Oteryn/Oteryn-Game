"""Tests for convert_placements.py: the Npc.Placement family is the scoped promotion candidates."""

import json
import sys
import unittest
from unittest.mock import patch
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import convert_placements as cp  # noqa: E402
sys.path.insert(0,str(cp.ROOT/"tools/content-migration"))
import support_npc_placements


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

    def test_l1_reward_npcs_use_only_the_global_source_cells(self):
        expected = {
            'oteryn:npc.cael': {'x':33028,'y':31513,'floor':10},
            'oteryn:npc.cranky_lizard_crone': {'x':33192,'y':31044,'floor':6},
            'oteryn:npc.palimuth': {'x':32801,'y':31223,'floor':7},
            'oteryn:npc.yalahari': {'x':32801,'y':31188,'floor':7},
        }
        actual=[r['declaration'] for r in self.records if r['declaration']['npc'] in expected]
        self.assertEqual(len(actual),4)
        self.assertEqual({r['npc']:r['cell'] for r in actual},expected)
        self.assertTrue(all(r['provenance'] for r in actual))

    def test_only_scoped_admitted_npcs(self):
        scope = set(json.loads(cp.SCOPE.read_text())["npcs"])
        summer = json.loads((cp.ROOT / "tools/content-migration/summer_npcs.json").read_text())
        furniture = json.loads((cp.ROOT / "tools/content-migration/furniture_npc_placements.json").read_text())
        cosmetic = json.loads((cp.ROOT / "tools/content-migration/cosmetic_npc_placements.json").read_text())
        support = json.loads((cp.ROOT / "tools/content-migration/support_npc_placements.json").read_text())
        source = ({row["npc"] for row in summer["positions"]} | {row["npc"] for row in furniture["rows"]}
                  | {row["npc"] for row in cosmetic["rows"]} | {row["npc"] for row in support["rows"]})
        self.assertEqual({r["declaration"]["npc"] for r in self.records},
                         (scope - {h["npc"] for h in self.held}) | source)
        self.assertLessEqual(scope | source, cp.admitted_keys())

    def test_monk_pilgrimage_npcs_use_the_reviewed_crystal_cells(self):
        expected = {
            'oteryn:npc.ambassador_manop': {'x': 32057, 'y': 31898, 'floor': 4},
            'oteryn:npc.enpa_deia_pema': {'x': 33603, 'y': 31468, 'floor': 7},
        }
        actual = {r['declaration']['npc']: r['declaration'] for r in self.records
                  if r['declaration']['npc'] in expected}
        self.assertEqual(set(actual), set(expected))
        for key, cell in expected.items():
            self.assertEqual(actual[key]['cell'], cell)
            self.assertTrue(any(p['origin'] == 'crystal' for p in actual[key]['provenance']))

    def test_shards_sorted_unique_and_keyed_by_cell(self):
        keys = [r["declaration"]["identity"]["key"] for r in self.records]
        self.assertEqual(len(keys), len(set(keys)))
        for name, text in self.files.items():
            if name.startswith("placements-"):
                shard_keys = [row["declaration"]["identity"]["key"]
                              for row in json.loads(text)["records"]]
                self.assertEqual(shard_keys, sorted(shard_keys), name)
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

    def test_summer_mayor_uses_his_admitted_cell_and_actual_source_revision(self):
        mayor = next(r["declaration"] for r in self.records
                     if r["declaration"]["npc"] == "oteryn:npc.mayor_pocaro")
        self.assertEqual(mayor["cell"], {"x": 34073, "y": 32504, "floor": 6})
        self.assertEqual(mayor["direction"], "north")
        self.assertEqual(mayor["provenance"][0]["revision"],
                         "00ce02a57ca5a12e48f32a3476e37471167e4c3f")
        candidate = next(c for c in json.loads(cp.CANDIDATES.read_text())["candidates"]
                         if c["identity"]["key"] == mayor["npc"])
        self.assertEqual(mayor["provenance"][0]["definition_sha256"],
                         candidate["provenance"]["crystal"]["sha256"])

    def test_summer_overlay_keeps_base_shards_and_all_twenty_source_cells(self):
        import summer_npcs
        import furniture_npc_placements
        import cosmetic_npc_placements

        with patch.object(support_npc_placements, "augment_placements", side_effect=lambda files, frame: files), \
             patch.object(cosmetic_npc_placements, "augment_placements", side_effect=lambda files, frame: files), \
             patch.object(summer_npcs, "augment_placements", side_effect=lambda files, frame: files), \
             patch.object(furniture_npc_placements, "augment_placements", side_effect=lambda files, frame: files):
            base = cp.convert()
        for name, text in base.items():
            if name.startswith("placements-"):
                self.assertEqual(self.files[name], text, "existing base shard is preserved")
        self.assertEqual(json.loads(base["index.json"])["record_count"], 33)
        with patch.object(support_npc_placements, "augment_placements", side_effect=lambda files, frame: files), \
             patch.object(cosmetic_npc_placements, "augment_placements", side_effect=lambda files, frame: files), \
             patch.object(furniture_npc_placements, "augment_placements", side_effect=lambda files, frame: files):
            original_summer = cp.convert()
        self.assertEqual(json.loads(original_summer["index.json"])["record_count"], 50)
        for name, text in original_summer.items():
            if name.startswith("placements-"):
                self.assertEqual(self.files[name], text, "all original50 placements retain their exact bytes")
        source = json.loads((cp.ROOT / "tools/content-migration/summer_npcs.json").read_text())
        actual = {(row["declaration"]["npc"], tuple(row["declaration"]["cell"][key]
                    for key in ("x", "y", "floor"))) for row in self.records}
        for row in source["positions"]:
            self.assertIn((row["npc"], tuple(row["cell"][key] for key in ("x", "y", "floor"))), actual)

    def test_cosmetic_overlay_retains_all_original_seventy_nine_placements(self):
        import cosmetic_npc_placements
        with patch.object(support_npc_placements, "augment_placements", side_effect=lambda files, frame: files), \
             patch.object(cosmetic_npc_placements, "augment_placements", side_effect=lambda files, frame: files):
            before = cp.convert()
        self.assertEqual(json.loads(before["index.json"])["record_count"], 79)
        for name, text in before.items():
            if name.startswith("placements-"):
                self.assertEqual(self.files[name], text, "all79 original placements retain exact bytes")
        self.assertEqual(self.index["record_count"], 138)
        source = json.loads((cp.ROOT / "tools/content-migration/cosmetic_npc_placements.json").read_text())
        self.assertEqual(len(source["rows"]), 56)
        actual = {(r["declaration"]["npc"], tuple(r["declaration"]["cell"][k] for k in ("x", "y", "floor"))) for r in self.records}
        for row in source["rows"]:
            self.assertIn((row["npc"], tuple(row["cell"][k] for k in ("x", "y", "floor"))), actual)
            self.assertEqual(row["source"]["revision"], "201189f871d23597b82caf88fd611454a2ddefb8")
            self.assertGreater(row["source"]["line"], 0)

    def test_support_overlay_preserves_all_original_one_hundred_thirty_five_placements(self):
        with patch.object(support_npc_placements,"augment_placements",side_effect=lambda files,frame:files):
            before=cp.convert()
        self.assertEqual(json.loads(before["index.json"])["record_count"],135)
        for name,text in before.items():
            if name.startswith("placements-"):
                self.assertEqual(self.files[name],text,"all135 original placement shard bytes preserved")
        self.assertEqual(self.index["record_count"],138)
        self.assertEqual(len(json.loads((cp.ROOT/"tools/content-migration/support_npc_placements.json").read_text())["rows"]),3)

    def test_index_counts_and_held_digest(self):
        self.assertEqual(self.index["record_count"], len(self.records))
        self.assertEqual(self.index["held"]["count"], len(self.held))
        import hashlib

        self.assertEqual(self.index["held"]["sha256"], hashlib.sha256(self.files["held.json"].encode()).hexdigest())
        self.assertTrue(all(h["reason"] in cp.REASONS for h in self.held))


if __name__ == "__main__":
    unittest.main()
