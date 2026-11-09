"""Daily Bank Robbery bindings preserve the chosen recipe and custody boundary."""
import unittest
from unittest.mock import patch
import json
import qmech_m55_daily_bank as bank


class DailyBank(unittest.TestCase):
    def test_recipe_reward_change_requires_requalification(self):
        source = json.loads((bank.ROOT / bank.INPUT).read_text())
        source['records'][88]['definition']['recipe']['reward_intents'][0]['count'] = 20
        parse = json.loads
        with patch.object(bank.json, 'loads', side_effect=lambda raw:
                          source if 'OTERYN_QUEST_TREE_SHARD/v1' in raw else parse(raw)):
            with self.assertRaises(ValueError):
                bank.generate(bank.ROOT)

    def test_recipe_graph_change_requires_requalification(self):
        source = json.loads((bank.ROOT / bank.INPUT).read_text())
        source['records'][88]['definition']['recipe']['stages'][0]['next'] = ['s4']
        parse = json.loads
        with patch.object(bank.json, 'loads', side_effect=lambda raw:
                          source if 'OTERYN_QUEST_TREE_SHARD/v1' in raw else parse(raw)):
            with self.assertRaises(ValueError):
                bank.generate(bank.ROOT)

    def test_recovery_requires_guaranteed_single_bag(self):
        loot = {'entries': [{'item': {'key': 'oteryn:item.tibia.i13429'},
                             'min_count': 1, 'max_count': 1, 'probability_ppm': 999999}]}
        with self.assertRaises(ValueError):
            bank.validate_recovery_loot(loot)
        loot['entries'][0]['probability_ppm'] = 1000000
        bank.validate_recovery_loot(loot)
        loot['entries'][0]['min_count'] = 0
        with self.assertRaises(ValueError):
            bank.validate_recovery_loot(loot)

    def test_recipe_bound_daily_route(self):
        quest = bank.generate(bank.ROOT)['quests'][0]
        self.assertEqual([s['key'] for s in quest['stages']], ['s1', 's2', 's3', 's4'])
        self.assertEqual(quest['occurrence']['world_change'], 'server_save')
        reset = quest['occurrence']['restart']
        self.assertEqual(reset['npc'], 'oteryn:npc.naji')
        self.assertEqual({s['track'] for s in reset['storage']}, {t['key'] for t in quest['tracks']})
        self.assertTrue(all(s['set'] == 0 for s in reset['storage']))
        self.assertFalse(reset['take'])
        self.assertIsNone(reset['give'])
        self.assertFalse(any(s['completes'] for s in quest['stages'][:-1]))
        kill = quest['stages'][1]['trigger']['counter']
        self.assertEqual(kill['creature'], 'oteryn:creature.jesse_the_wicked')
        self.assertEqual(kill['maximum'], 1)
        collection = quest['stages'][2]['trigger']['collection']
        self.assertEqual(collection['items'], [{'item': 'oteryn:item.tibia.i13429', 'count': 1}])
        reward = quest['stages'][3]['trigger']['callback']
        self.assertEqual(reward['take'], collection['items'])
        self.assertEqual(reward['give'], {'item': 'oteryn:item.tibia.i3035', 'count': 10})
        self.assertEqual(reward['achievement'], 'oteryn:achievement/honest_finder')


if __name__ == '__main__':
    unittest.main()
