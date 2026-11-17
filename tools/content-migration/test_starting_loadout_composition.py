"""Shared native providers must retain one exact owner for a starter Item."""
import copy
import json
from pathlib import Path
import unittest

import starting_loadout_profiles as source

ROOT = Path(__file__).resolve().parents[2]


def admit(generated):
    source.admit(ROOT, generated, 'items', 'manifest',
                 lambda row: row['production_definition']['production_key'],
                 lambda value: json.dumps(value).encode())


class StarterComposition(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        generated = {'items': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        admit(generated)
        cls.rows = json.loads(generated[source.FRAGMENT])['records']

    def fixture(self):
        existing = copy.deepcopy(next(row for row in self.rows
                                     if row['production_definition']['production_key']
                                     == 'oteryn:item.tibia.i3571'))
        existing['authoring']['item'] = {
            'family': 'Item', 'key': 'oteryn:item.tibia.i3571', 'revision': 'definition-r1'}
        return existing

    def generated(self, existing):
        return {'items': json.dumps({'records': [existing]}).encode(),
                'manifest': b'{"item_profiles":{}}'}

    def test_existing_exact_owner_is_reused_without_duplicate_runtime_profile(self):
        existing = self.fixture()
        generated = self.generated(existing)
        admit(generated)
        base = json.loads(generated['items'])['records']
        fragment = json.loads(generated[source.FRAGMENT])['records']
        self.assertEqual(base, [existing])
        self.assertEqual(len(fragment), 7)
        keys = [row['production_definition']['production_key'] for row in base + fragment]
        self.assertEqual(len(keys), 8)
        self.assertEqual(len(set(keys)), 8)
        self.assertEqual(set(keys), {f'oteryn:item.tibia.i{item}' for item in source.IDS})

    def test_conflicting_known_weight_is_refused_without_partial_writes(self):
        existing = self.fixture()
        existing['semantics']['physical']['value']['weight']['value'] += 1
        generated = self.generated(existing)
        before = copy.deepcopy(generated)
        with self.assertRaises(ValueError):
            admit(generated)
        self.assertEqual(generated, before)

    def test_different_production_binding_is_refused_without_partial_writes(self):
        existing = self.fixture()
        existing['production_binding']['target']['revision'] = 'wrong-revision'
        generated = self.generated(existing)
        before = copy.deepcopy(generated)
        with self.assertRaises(ValueError):
            admit(generated)
        self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
