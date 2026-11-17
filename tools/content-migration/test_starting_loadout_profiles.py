"""SRVGAP-C1: native kit closure and the Monk's donor two-handed equipment policy."""
import copy
import json
from pathlib import Path
import unittest
import starting_loadout_profiles as source

ROOT = Path(__file__).resolve().parents[2]


class StartingLoadoutProfiles(unittest.TestCase):
    def test_reaudit_starting_progression_native_item_closure(self):
        generated = {'items': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        source.admit(ROOT, generated, 'items', 'manifest',
                     lambda row: row['production_definition']['production_key'],
                     lambda value: json.dumps(value).encode())
        records = json.loads(generated[source.FRAGMENT])['records']
        self.assertEqual(len(records), 8)
        self.assertEqual(generated['items'], b'{"records":[]}')
        for record in records:
            self.assertTrue(record['admission']['materializable'])
            self.assertIn('CharacterInventory', record['admission']['legal_destinations'])
        # Canary 47dfd51f items.xml:11278-11291,21318-21333,22257-22265.
        for number, armor in {3571: 7, 3572: 1, 7991: 6, 7992: 3, 8095: 4}.items():
            record, = [row for row in records if row['production_definition']['production_key'] == f'oteryn:item.tibia.i{number}']
            self.assertEqual(record['semantics']['protection'].get('value', {}).get('armor'),
                             {'state': 'KNOWN', 'value': armor}, f'source armor {number}')
        staff, = [row for row in records if row['production_definition']['production_key'] == 'oteryn:item.tibia.i50171']
        pattern, = staff['semantics']['equipment']['value']['patterns']['value']
        self.assertEqual(pattern['additional_reserved_slots'], {'state': 'KNOWN', 'value': ['SHIELD']})
        self.assertEqual(pattern['vocations'], {'state': 'KNOWN', 'value': ['MONK']})
        self.assertEqual(staff['semantics']['physical']['value']['weight'], {'state': 'KNOWN', 'value': 2100})

    def test_existing_owner_is_unique_and_conflicting_physics_refuse_atomically(self):
        canonical = lambda value: json.dumps(value).encode()
        order = lambda row: row['production_definition']['production_key']
        fresh = {'items': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        source.admit(ROOT, fresh, 'items', 'manifest', order, canonical)
        original = json.loads(fresh[source.FRAGMENT])['records'][0]
        existing = copy.deepcopy(original)
        existing['semantics']['protection'] = {'state': 'UNKNOWN'}
        planned = {'items': canonical({'records': [existing]}),
                   'manifest': b'{"item_profiles":{}}'}
        source.admit(ROOT, planned, 'items', 'manifest', order, canonical)
        kept = json.loads(planned['items'])['records']
        additions = json.loads(planned[source.FRAGMENT])['records']
        self.assertEqual(kept, [original])
        self.assertEqual(len(additions), 7)
        self.assertNotIn(original['production_definition']['production_key'],
                         [row['production_definition']['production_key'] for row in additions])
        existing['semantics']['physical']['value']['weight']['value'] += 1
        planned = {'items': canonical({'records': [existing]}),
                   'manifest': b'{"item_profiles":{}}'}
        before = dict(planned)
        with self.assertRaisesRegex(ValueError, 'STARTING_LOADOUT_PROFILE_CONFLICT'):
            source.admit(ROOT, planned, 'items', 'manifest', order, canonical)
        self.assertEqual(planned, before)


if __name__ == '__main__':
    unittest.main()
