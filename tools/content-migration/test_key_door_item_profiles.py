"""The ordinary native provider must carry M06's playable door/key definitions."""
import json
import unittest
from pathlib import Path

import key_door_item_profiles as keys

ROOT = Path(__file__).resolve().parents[2]


class KeyDoorProfileTests(unittest.TestCase):
    def test_native_provider_contains_the_actual_key_and_door_physics(self):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        profiles = {r['production_definition']['production_key']: r for r in rows}
        for item, kind, solid in [(1628, 'DOOR', True), (1629, 'DOOR', True),
                                  (1630, 'DOOR', False), (1646, 'DOOR', True),
                                  (1647, 'DOOR', False), (2967, 'KEY', False),
                                  (2968, 'KEY', False), (2970, 'KEY', False), (5097, 'DOOR', True),
                                  (5098, 'DOOR', True), (5099, 'DOOR', False),
                                  (5102, 'DOOR', True), (5103, 'DOOR', False)]:
            with self.subTest(item=item):
                row = profiles.get(f'oteryn:item.tibia.i{item}')
                self.assertIsNotNone(row, f'Production native profile missing: {item}')
                self.assertEqual(row['semantics']['classification']['value']['item_type'],
                                 {'state': 'KNOWN', 'value': kind})
                self.assertIs(row['attributes']['blocks_movement'], solid)
                self.assertIs(row['attributes']['blocks_projectile'], solid)
                self.assertTrue(row['admission']['materializable'])

    def test_partial_profile_refinement_rejects_a_conflicting_known_fact(self):
        old = {'classification': {'state': 'UNKNOWN'}, 'weight': {'state': 'KNOWN', 'value': 100}}
        source = {'classification': {'state': 'KNOWN', 'value': 'KEY'}, 'weight': {'state': 'KNOWN', 'value': 100}}
        self.assertEqual(keys.refine_unknown(old, source), source)
        self.assertEqual(old['classification'], {'state': 'UNKNOWN'})
        source['weight']['value'] = 101
        with self.assertRaisesRegex(ValueError, 'KEY_DOOR_ITEM_PROFILE_KNOWN_CONFLICT'):
            keys.refine_unknown(old, source)

    def test_duplicate_identity_refuses_without_partially_changing_output(self):
        record = json.loads((ROOT / keys.PATH).read_bytes())['records'][0]
        planned = {'profiles': json.dumps({'records': [record]}).encode(), 'manifest': b'{}'}
        before = dict(planned)
        with self.assertRaisesRegex(ValueError, 'KEY_DOOR_ITEM_PROFILE_DUPLICATE'):
            keys.admit(ROOT, planned, 'profiles', 'manifest', lambda row: 0, lambda v: b'')
        self.assertEqual(planned, before)


if __name__ == '__main__':
    unittest.main()
