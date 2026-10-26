"""The active provider must expose the identities the production quick-loot path consumes."""
import json
from pathlib import Path
import unittest
import quick_loot_item_profiles as quick

ROOT = Path(__file__).resolve().parents[2]


class QuickLootProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        cls.profiles = {r['production_definition']['production_key']: r for r in rows}

    def test_quivers_keep_bound_identities_and_both_global_slots(self):
        for number in sorted(quick.IDS - {23721}):
            key = f'oteryn:item.tibia.i{number}'
            row = self.profiles[key]
            self.assertEqual(row['production_binding']['target']['key'], key)
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            patterns = row['semantics']['equipment']['value']['patterns']['value']
            self.assertEqual([p['primary_slot'] for p in patterns], [quick.known('SHIELD'), quick.known('EXTRA')])
            self.assertEqual(row['semantics']['weapon'], {'state': 'NOT_APPLICABLE'})
            self.assertEqual(row['semantics']['charges'], {'state': 'NOT_APPLICABLE'})
            self.assertEqual(row['semantics']['temporal'], {'state': 'NOT_APPLICABLE'})
        basic = self.profiles['oteryn:item.tibia.i35562']
        self.assertEqual(basic['semantics']['container']['value']['capacity'], quick.known(6))
        self.assertEqual(basic['semantics']['equipment']['value']['patterns']['value'][0]['level'], quick.known(0))

    def test_pouch_is_not_admitted_to_ground_and_sorting_uses_taxonomy(self):
        self.assertEqual(self.profiles['oteryn:item.tibia.i23721']['admission']['legal_destinations'], ['CharacterInventory'])
        primaries = {r['authoring']['taxonomy']['primary'] for r in self.profiles.values() if 'taxonomy' in r['authoring']}
        self.assertTrue({'Creature Products', 'Food', 'Ammunition', 'Quivers'} <= primaries)


if __name__ == '__main__':
    unittest.main()
