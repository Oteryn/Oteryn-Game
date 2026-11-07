"""Authored NPC cells require an admitted identity and a pinned matching talk recipe."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import convert_placements as producer


class AuthoredPlacements(unittest.TestCase):
    def fixture(self, root):
        key = 'oteryn:npc.lai'
        path = 'recipe.json'
        raw = json.dumps({'recipes': [{'wiki_title': 'Mystery of the Valley Quest',
            'stages': [{'kind': 'talk', 'targets': ['Lai']}]}]}).encode()
        (root / path).write_bytes(raw)
        row = {'basis': 'CHOSEN_OTERYN_APPROXIMATION', 'recipe_path': path,
            'recipe_sha256': hashlib.sha256(raw).hexdigest(),
            'recipe_title': 'Mystery of the Valley Quest',
            'placements': [{'position': {'x': 32572, 'y': 32507, 'z': 7}, 'direction': 'SOUTH'}]}
        return key, {'npcs': [key], 'authored_placements': {key: row}}

    def test_admits_only_the_pinned_recipe_npc_and_keeps_donor_candidates(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key, scope = self.fixture(root)
            candidates = {'oteryn:npc.other': {'placements': []}}
            before = copy.deepcopy(candidates)
            actual = producer.apply_authored_placements(candidates, scope, {key}, root)
            self.assertEqual(candidates, before)
            self.assertEqual(actual['oteryn:npc.other'], before['oteryn:npc.other'])
            self.assertEqual(actual[key]['placements'], scope['authored_placements'][key]['placements'])
            provenance = producer.provenance(actual[key], actual[key]['placements'][0])
            self.assertEqual(provenance[0]['origin'], 'oteryn')
            self.assertEqual(provenance[0]['basis'], 'CHOSEN_OTERYN_APPROXIMATION')

    def test_refuses_unadmitted_unpinned_unrelated_or_unbounded_cells(self):
        for case in range(12):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                key, scope = self.fixture(root)
                row = scope['authored_placements'][key]
                candidates, admitted = {}, {key}
                if case == 0: admitted = set()
                elif case == 1: scope['npcs'] = []
                elif case == 2: row['recipe_sha256'] = '0' * 64
                elif case == 3: row['recipe_title'] = 'Unrelated'
                elif case == 4: row['placements'] = []
                elif case == 5: row['placements'] *= 9
                elif case == 6: row['placements'][0]['position']['z'] = 16
                elif case == 7: candidates[key] = {'placements': []}
                elif case == 8: row['basis'] = 'SOURCE_QUALIFIED'
                elif case == 9: row['placements'][0]['direction'] = 'UNKNOWN'
                elif case == 10: row['placements'] *= 2
                else: row['recipe_path'] = '../recipe.json'
                with self.assertRaises((ValueError, SystemExit)):
                    producer.apply_authored_placements(candidates, scope, admitted, root)


if __name__ == '__main__':
    unittest.main()
