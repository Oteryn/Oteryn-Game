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

    def test_wcontrib_dialogue_and_travel_actors_have_source_cells(self):
        expected = {
            'alaistar': (33619, 31884, 4),
            'appaloosa': (32852, 32124, 7),
            'asralius': (32096, 32219, 5),
            'captain_gulliver': (33479, 31986, 7),
            'captain_seahorse': (33175, 31763, 6),
            'chavis': (33548, 31948, 7),
            'flint': (33635, 31882, 6),
            'junkar_mines': (32556, 32069, 10),
            'junkar_robsons': (32525, 32038, 14),
            'junkar_thais': (32368, 32181, 13),
            'king_tibianus': (32311, 32172, 6),
            'marvin': (33662, 31878, 5),
            'ottokar': (32883, 32113, 6),
            'palomino': (32450, 32226, 7),
            'yawno': (32614, 32651, 7),
        }
        declarations = {r['declaration']['npc']: r['declaration'] for r in self.records}
        for name, (x, y, floor) in expected.items():
            with self.subTest(npc=name):
                declaration = declarations[f'oteryn:npc.{name}']
                self.assertEqual(declaration['cell'], {'x': x, 'y': y, 'floor': floor})
                self.assertTrue(any(p['origin'] in {'canary', 'crystal'}
                                    and p['revision'] and p['definition_sha256']
                                    for p in declaration['provenance']))
        # These identities are the exact services used by the paid travel owner;
        # a source alias with a different actor name cannot expose the route.
        routes = json.loads((cp.ROOT / 'crates/gameplay/content/src/content/npc_catalogue/travel_routes.json').read_text())
        if isinstance(routes, dict):
            routes = routes['routes']
        keys = {r['key'] for r in routes}
        for name in ('junkar_mines', 'junkar_robsons', 'junkar_thais',
                     'marvin', 'palomino', 'appaloosa',
                     'captain_gulliver', 'captain_seahorse'):
            self.assertTrue(any(key.startswith(f'oteryn:service.travel.{name}/')
                                for key in keys), name)

    def test_only_scoped_admitted_npcs(self):
        scope = set(json.loads(cp.SCOPE.read_text())["npcs"])
        self.assertEqual({r["declaration"]["npc"] for r in self.records}, scope - {h["npc"] for h in self.held})
        self.assertLessEqual(scope, cp.admitted_keys())

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

    def test_index_counts_and_held_digest(self):
        self.assertEqual(self.index["record_count"], len(self.records))
        self.assertEqual(self.index["held"]["count"], len(self.held))
        import hashlib

        self.assertEqual(self.index["held"]["sha256"], hashlib.sha256(self.files["held.json"].encode()).hexdigest())
        self.assertTrue(all(h["reason"] in cp.REASONS for h in self.held))


if __name__ == "__main__":
    unittest.main()
