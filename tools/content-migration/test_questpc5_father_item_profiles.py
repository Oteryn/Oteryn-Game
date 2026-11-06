"""Tereban's real-dispatch inputs must exist in the active native generation."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class FatherMaterialProfiles(unittest.TestCase):
    def test_all_eight_handins_have_portable_nonstackable_source_profiles(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = {row['production_definition']['production_key']: row
                for row in document['records']}
        for number, weight in zip((*range(11545, 11553), 11701),
                                  (10, 500, 500, 35, 222, 80, 60, 140, 3500)):
            with self.subTest(number=number):
                key = f'oteryn:item.tibia.i{number}'
                self.assertTrue(key in rows, key)
                row = rows[key]
                self.assertEqual(row['production_binding']['target']['key'], key)
                self.assertEqual(row['production_binding']['disposition'], 'EXACT')
                self.assertEqual(row['admission'], {
                    'materializable': True, 'stack_class': 'NonStackable',
                    'legal_destinations': ['CharacterInventory', 'Ground']})
                self.assertEqual(row['semantics']['physical']['value']['weight'],
                                 {'state': 'KNOWN', 'value': weight})
                self.assertEqual(row['semantics']['container']['state'], 'NOT_APPLICABLE')
                self.assertEqual(row['semantics']['charges']['state'], 'NOT_APPLICABLE')
                self.assertEqual(row['semantics']['temporal']['state'], 'NOT_APPLICABLE')


if __name__ == '__main__':
    unittest.main()
