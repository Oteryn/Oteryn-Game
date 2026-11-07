"""Coexisting source owners retain admitted Item policy without duplicate identities."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import qengine_reward_item_profiles as owner

ROOT = Path(__file__).resolve().parents[2]

class ItemOverlap(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads((ROOT / owner.PROOF).read_bytes())
        self.row = copy.deepcopy(next(r for r in self.packet['records']
            if r['production_definition']['production_key'] == 'oteryn:item.tibia.i139'))
        self.row['attributes']['earlier_owner_fact'] = 7

    def apply(self, row):
        generated = {'items': json.dumps({'records': [row]}).encode(),
                     'manifest': b'{"item_profiles":{}}',
                     'content/cosmetics/outfits/native-cosmetics.json': b'{}'}
        with patch.object(owner, 'capture', return_value=copy.deepcopy(self.packet)):
            owner.admit(ROOT, generated, 'items', 'manifest',
                lambda r: r['production_definition']['production_key'],
                lambda d: json.dumps(d).encode())
        return json.loads(generated['items'])['records']

    def test_existing_policy_and_unique_identity_survive_the_physical_cohort(self):
        rows = self.apply(self.row)
        keys = [r['production_definition']['production_key'] for r in rows]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertEqual(next(r for r in rows if r['production_definition'] == self.row['production_definition']), self.row)

    def test_unknown_physical_fact_is_refined_without_discarding_existing_policy(self):
        row = copy.deepcopy(self.row)
        expected = row['semantics']['physical']['value']['weight']
        row['semantics']['physical']['value']['weight'] = {'state': 'UNKNOWN'}
        actual = next(r for r in self.apply(row) if r['production_definition'] == row['production_definition'])
        self.assertEqual(actual['semantics']['physical']['value']['weight'], expected)
        self.assertEqual(actual['attributes'], row['attributes'])

    def test_conflicting_weight_binding_or_custody_is_rejected(self):
        for field in ('weight', 'production_binding', 'admission'):
            row = copy.deepcopy(self.row)
            if field == 'weight': row['semantics']['physical']['value']['weight']['value'] += 1
            else: row[field] = {}
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
                self.apply(row)

if __name__ == '__main__': unittest.main()
