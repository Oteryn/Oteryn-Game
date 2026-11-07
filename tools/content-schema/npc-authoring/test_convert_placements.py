"""Tests for convert_placements.py: the Npc.Placement family is the scoped promotion candidates."""

import hashlib
import json
import re
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

    def test_reviewed_spawn_anchors_preserve_roaming_observations_and_source_hashes(self):
        names = {'a_restless_soul', 'chemar', 'eclesius', 'eshaya', 'hagor', 'malor'}
        capture_bytes = cp.CANDIDATES.read_bytes()
        captures = {r['identity']['key']: r for r in json.loads(capture_bytes)['candidates']}
        rows = json.loads((cp.HERE / 'samples/npcfix-retail-placements.json').read_text())
        actual = {r['declaration']['npc']: r['declaration']['cell'] for r in self.records}
        checked = set()
        for row in rows:
            key = row['npc']
            if key.removeprefix('oteryn:npc.') not in names:
                continue
            with self.subTest(npc=key):
                capture = captures[key]
                anchor = capture['placements'][0]['position']
                selected = row['position']
                self.assertEqual(actual[key], {'x': selected['x'], 'y': selected['y'], 'floor': selected['z']})
                self.assertEqual(row['captured_spawn_anchor'], anchor)
                self.assertEqual(selected['z'], anchor['z'])
                self.assertLessEqual(max(abs(selected['x']-anchor['x']), abs(selected['y']-anchor['y'])), capture['movement']['walk_radius'])
                observed = row['audited_roaming_position']
                self.assertEqual(observed['z'], anchor['z'])
                self.assertLessEqual(max(abs(observed['x']-anchor['x']), abs(observed['y']-anchor['y'])), capture['movement']['walk_radius'])
                self.assertEqual(row['native_source']['sha256'], hashlib.sha256(capture_bytes).hexdigest())
                self.assertEqual(row['origin'], 'native')
                checked.add(key.removeprefix('oteryn:npc.'))
        self.assertEqual(checked, names)

    def test_three_endpoints_preserve_original_sources_and_stationary_policy(self):
        rows = {r['npc']: r for r in json.loads((cp.HERE / 'samples/npcfix-retail-placements.json').read_text())}
        scope = json.loads(cp.SCOPE.read_text())
        authored = {r['npc']: r for r in scope['retail_placements']}
        for key in ('oteryn:npc.a_wandering_soul', 'oteryn:npc.goldro', 'oteryn:npc.doctor_marrow'):
            self.assertEqual(rows[key], authored[key])
        wandering = rows['oteryn:npc.a_wandering_soul']
        self.assertEqual(wandering['position'], {'x': 32915, 'y': 31489, 'z': 7})
        self.assertEqual(wandering['captured_spawn_anchor'], {'x': 32914, 'y': 31490, 'z': 7})
        self.assertEqual(wandering['walk_radius'], 2)
        self.assertEqual(wandering['native_source']['sha256'], hashlib.sha256(cp.CANDIDATES.read_bytes()).hexdigest())
        goldro = rows['oteryn:npc.goldro']
        self.assertEqual(goldro['position'], {'x': 34055, 'y': 32504, 'z': 7})
        self.assertEqual(goldro['captured_spawn_anchor'], {'x': 34055, 'y': 32503, 'z': 7})
        self.assertEqual(goldro['audited_roaming_position'], goldro['captured_spawn_anchor'])
        self.assertEqual(goldro['walk_radius'], 1)
        self.assertEqual(goldro['native_source']['donor_movement']['walk_radius'], 1)
        self.assertEqual(goldro['native_source']['donor_movement']['sha256'], '394da3c41a2e81fb8b2cb5ee637a95e4bb50393de1069a338452a1ab89ab15f8')
        doctor = rows['oteryn:npc.doctor_marrow']
        self.assertEqual(doctor['position'], {'x': 34009, 'y': 32643, 'z': 6})
        self.assertNotIn('captured_spawn_anchor', doctor)
        self.assertNotIn('held_reason', doctor)
        resolution = doctor['position_conflict_resolution']
        self.assertEqual(resolution['stationary_donor']['walk_radius'], 0)
        self.assertEqual(resolution['policy'], 'STRICTLY_ADMITTED_PRIMARY_AUDITED_WIKI_CELL')
        self.assertEqual({(r['cell']['x'], r['cell']['y'], r['cell']['z']) for r in resolution['previous_observations']}, {(34009, 32643, 6), (33992, 32662, 6), (34010, 32641, 6)})
        self.assertFalse(any(r['npc'] == doctor['npc'] for r in scope['position_conflicts']))

    def test_npcfix_retail_population_has_each_audited_cell(self):
        expected = json.loads((cp.HERE / "samples/npcfix-retail-placements.json").read_text())
        actual = {}
        for record in self.records:
            d = record["declaration"]
            actual.setdefault(d["npc"], set()).add((d["cell"]["x"], d["cell"]["y"], d["cell"]["floor"]))
        for row in expected:
            with self.subTest(npc=row["npc"]):
                if "held_reason" in row:
                    held = [h for h in self.held if h["npc"] == row["npc"]]
                    self.assertEqual([h["reason"] for h in held], [row["held_reason"]])
                    self.assertNotIn(row["npc"], actual)
                    continue
                self.assertTrue(row["npc"] in actual, row["npc"])
                expected_cells = {(cell["x"], cell["y"], cell["z"])
                                  for cell in row.get("positions", [row["position"]])}
                self.assertEqual(actual[row["npc"]], expected_cells)

    def test_rashid_population_matches_the_existing_weekday_selector(self):
        rows = json.loads((cp.HERE / "samples/npcfix-retail-placements.json").read_text())
        row = next(r for r in rows if r["npc"] == "oteryn:npc.rashid")
        source = (cp.ROOT / row["native_source"]["path"]).read_text()
        cells = re.findall(r'destination\("(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)", ([\d_]+), ([\d_]+), (\d+)\)', source)
        self.assertEqual(len(cells), 7)
        self.assertEqual(row["positions"], [{"x": int(x.replace("_", "")), "y": int(y.replace("_", "")), "z": int(z)} for x, y, z in cells])

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
