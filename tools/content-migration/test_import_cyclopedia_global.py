#!/usr/bin/env python3
"""Captured Global client weights survive legacy regeneration (ITEMGLOB)."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import import_cyclopedia_global as cyclopedia  # noqa: E402


def definition(key, weight, state='KNOWN'):
    value = {'weight': {'state': 'KNOWN', 'value': weight}} if state == 'KNOWN' else None
    physical = {'state': state, 'value': value} if value else {'state': state}
    return {'identity': {'family': 'Item', 'key': key}, 'semantics': {'physical': physical}}


class ClientWeightAdmission(unittest.TestCase):
    def test_capture_weights_match_client(self):
        weights = cyclopedia.load_weights(ROOT)
        for identifier, expected in {3031: 10, 3035: 10, 3043: 10,
                                     9147: 100, 19371: 1700,
                                     25729: 200, 25730: 200}.items():
            with self.subTest(identifier=identifier):
                self.assertEqual(weights[identifier], expected)
        self.assertEqual(weights[7439], 290)
        self.assertEqual(weights[7443], 290)

    def test_legacy_weight_is_replaced_and_unknown_is_left(self):
        records = [definition('oteryn:item.tibia.i7439', 200),
                   definition('oteryn:item.tibia.i7443', 200, state='UNKNOWN'),
                   definition('oteryn:item.tibia.i999999999', 7)]
        cyclopedia.apply_admissions(records, ROOT)
        self.assertEqual(records[0]['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 290})
        self.assertEqual(records[1]['semantics']['physical'], {'state': 'UNKNOWN'})
        self.assertEqual(records[2]['semantics']['physical']['value']['weight']['value'], 7)

    def test_only_weight_changed_accepts_weight_leaf_alone(self):
        before = definition('oteryn:item.tibia.i7439', 200)
        after = definition('oteryn:item.tibia.i7439', 290)
        self.assertTrue(cyclopedia.only_weight_changed(before, after))
        after['identity']['key'] = 'other'
        self.assertFalse(cyclopedia.only_weight_changed(before, after))
        self.assertFalse(cyclopedia.only_weight_changed(
            definition('k', 1, state='UNKNOWN'), definition('k', 2)))

    def test_generated_tree_carries_client_weights(self):
        weights = cyclopedia.load_weights(ROOT)
        stale = []
        for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json')):
            for row in json.loads(path.read_bytes())['records']:
                probe = json.loads(json.dumps(row['definition']))
                if cyclopedia.apply_weight(probe, weights):
                    stale.append(row['definition']['identity']['key'])
        self.assertEqual(stale, [])

    def test_generated_native_profiles_carry_client_weights(self):
        weights = cyclopedia.load_weights(ROOT)
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        stale = [identifier for identifier, weight in cyclopedia._overlay_profiles(document['records'])
                 if identifier in weights and weights[identifier] != weight['value']]
        self.assertEqual(stale, [])

    def test_profile_weights_replace_known_weight_only(self):
        record = {'production_definition': {'production_key': 'oteryn:item.tibia.i7439'},
                  'semantics': {'physical': {'state': 'KNOWN', 'value': {
                      'weight': {'state': 'KNOWN', 'value': 200},
                      'movable': {'state': 'KNOWN', 'value': True}}}}}
        cyclopedia.apply_profile_weights([record], ROOT)
        self.assertEqual(record['semantics']['physical']['value'],
                         {'weight': {'state': 'KNOWN', 'value': 290},
                          'movable': {'state': 'KNOWN', 'value': True}})


if __name__ == '__main__':
    unittest.main()
