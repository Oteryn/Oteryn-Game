"""SRVGAP-C1: native kit closure and the Monk's donor two-handed equipment policy."""
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


if __name__ == '__main__':
    unittest.main()
