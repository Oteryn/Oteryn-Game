"""SRVGAP-C1: native kit closure and the Monk's donor two-handed equipment policy."""
import copy
import hashlib
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


    def test_existing_native_profiles_move_without_changing_their_facts(self):
        generated = {'items': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        order = lambda row: row['production_definition']['production_key']
        canonical = lambda value: json.dumps(value).encode()
        source.admit(ROOT, generated, 'items', 'manifest', order, canonical)
        records = json.loads(generated[source.FRAGMENT])['records']
        existing = copy.deepcopy(records[0])
        existing['attributes']['existing_owner_fact'] = 7
        generated['items'] = canonical({'records': [existing]})
        source.admit(ROOT, generated, 'items', 'manifest', order, canonical)
        self.assertEqual(json.loads(generated['items'])['records'], [])
        fragment = json.loads(generated[source.FRAGMENT])['records']
        self.assertEqual(len(fragment), 8)
        actual, = [row for row in fragment if row['production_definition'] == existing['production_definition']]
        self.assertEqual(actual, existing)
        manifest = json.loads(generated['manifest'])
        self.assertEqual(manifest['item_profiles']['sha256'], hashlib.sha256(generated['items']).hexdigest())
        self.assertEqual(manifest['starting_loadout_item_profiles']['sha256'], hashlib.sha256(generated[source.FRAGMENT]).hexdigest())

    def test_overlapping_profile_cannot_substitute_binding_or_admission(self):
        generated = {'items': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        order = lambda row: row['production_definition']['production_key']
        canonical = lambda value: json.dumps(value).encode()
        source.admit(ROOT, generated, 'items', 'manifest', order, canonical)
        record = json.loads(generated[source.FRAGMENT])['records'][0]
        for field in ('production_definition', 'production_binding', 'admission'):
            with self.subTest(field=field):
                broken = copy.deepcopy(record)
                if field == 'production_definition':
                    broken[field]['revision_ref'] = 'forged-revision'
                else:
                    broken[field] = {}
                candidate = {'items': canonical({'records': [broken]}), 'manifest': b'{"item_profiles":{}}'}
                before = copy.deepcopy(candidate)
                with self.assertRaisesRegex(ValueError, 'STARTING_LOADOUT_PROFILE_CONFLICT'):
                    source.admit(ROOT, candidate, 'items', 'manifest', order, canonical)
                self.assertEqual(candidate, before)


if __name__ == '__main__':
    unittest.main()
