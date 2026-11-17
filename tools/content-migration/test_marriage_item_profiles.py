"""SRV9B: native wedding value is usable only with exact source-qualified facts."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MarriageItemProfiles(unittest.TestCase):
    def test_all_wedding_items_are_exact_portable_nonstackable_value(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        for number, weight in ((3004, 40), (9585, 40), (9586, 2500)):
            rows = [r for r in document['records']
                    if r['production_definition']['production_key'] == f'oteryn:item.tibia.i{number}']
            self.assertEqual(len(rows), 1, number)
            row = rows[0]
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['admission'], {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['semantics']['physical']['value']['weight'],
                             {'state': 'KNOWN', 'value': weight})
            self.assertEqual(row['semantics']['stack']['value']['stackable'],
                             {'state': 'KNOWN', 'value': False})


if __name__ == '__main__':
    unittest.main()
