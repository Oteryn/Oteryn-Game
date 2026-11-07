"""Santa uses normal admitted container/item policies for its native gift tree."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class SeasonalItemProfiles(unittest.TestCase):
    def test_cake_phase_map_and_brutus_rewards_are_source_bound(self):
        overlay = json.loads((ROOT / 'tools/content-schema/native-gameplay/seasonal-item-profiles.json').read_bytes())
        profiles = {row['production_definition']['production_key']: row for row in overlay['records']}
        expected = {12085, 12086, 12147, 12148, 12152, 12153, 12154, 12155,
                    12090, 12091, 12092, 12093, 12094, 12095,
                    14756, 20347, 20350, 28484, 28485, 28486}
        self.assertEqual(len(profiles), 25)
        for item in expected:
            profile = profiles[f'oteryn:item.tibia.i{item}']
            self.assertEqual(profile['production_binding']['disposition'], 'EXACT')
            self.assertTrue(profile['admission']['materializable'])
        sources = {row['path'] for row in overlay['sources']}
        self.assertIn('data-global/npc/brutus_the_baker.lua', sources)
        self.assertIn('data-global/scripts/lib/a_piece_of_cake_config.lua', sources)
        self.assertIn('data-global/scripts/quests/a_piece_of_cake/actions_cake_bite.lua', sources)
        self.assertNotIn('oteryn:item.tibia.i48227', profiles)

    def test_cake_corpse_and_residue_have_closed_native_decay_policies(self):
        profiles = {row['production_definition']['production_key']: row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for item in (12141, 2886, 2887):
            profile = profiles[f'oteryn:item.tibia.i{item}']
            self.assertTrue(profile['admission']['materializable'])
            self.assertIn('Ground', profile['admission']['legal_destinations'])
        corpse = profiles['oteryn:item.tibia.i12141']['semantics']
        self.assertEqual(corpse['container']['value']['capacity'], {'state': 'KNOWN', 'value': 10})
        self.assertEqual(corpse['temporal']['value']['duration'], {'state': 'KNOWN', 'value': 60000})
        self.assertEqual(profiles['oteryn:item.tibia.i2886']['semantics']['temporal']['value']['decay_target'],
                         {'state': 'KNOWN', 'value': {'key': 'oteryn:item.tibia.i2887', 'revision': 'definition-r1'}})

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
