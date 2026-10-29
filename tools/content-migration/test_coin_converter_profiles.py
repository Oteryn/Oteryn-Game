"""The active provider must expose the coin converters the production conversion path carries."""
import json
from pathlib import Path
import unittest
import coin_converter_profiles as converters

ROOT = Path(__file__).resolve().parents[2]


class CoinConverterProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        cls.profiles = {r['production_definition']['production_key']: r for r in rows}

    def test_converters_are_carried_nonstackable_tools_with_500_charges(self):
        for number in sorted(converters.IDS):
            key = f'oteryn:item.tibia.i{number}'
            row = self.profiles[key]
            self.assertEqual(row['production_binding']['target']['key'], key)
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['admission']['stack_class'], 'NonStackable')
            self.assertIn('CharacterInventory', row['admission']['legal_destinations'])
            self.assertEqual(row['semantics']['charges'], converters.known({'count': converters.known(500)}))
            self.assertEqual(row['semantics']['container'], {'state': 'NOT_APPLICABLE'})
            self.assertEqual(row['semantics']['physical']['value']['weight'], converters.known(300))
            self.assertEqual(row['attributes']['speed_bonus'], 0)


if __name__ == '__main__':
    unittest.main()
