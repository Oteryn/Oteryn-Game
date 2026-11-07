"""Santa uses normal admitted container/item policies for its native gift tree."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class SeasonalItemProfiles(unittest.TestCase):
    def test_present_container_and_snowball_have_native_materialization_policies(self):
        profiles = {row['production_definition']['production_key']: row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for item in (6510, 2992):
            profile = profiles[f'oteryn:item.tibia.i{item}']
            self.assertEqual(profile['production_binding']['disposition'], 'EXACT')
            self.assertTrue(profile['admission']['materializable'])
            self.assertIn('CharacterInventory', profile['admission']['legal_destinations'])
        self.assertEqual(profiles['oteryn:item.tibia.i6510']['semantics']['container']['value']['capacity'],
                         {'state': 'KNOWN', 'value': 5})


if __name__ == '__main__':
    unittest.main()
