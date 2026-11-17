"""The bounded corpse join cannot infer a different body or an unmounted policy."""
import unittest
import quest_shared_boss_corpses as corpses


class SharedBodies(unittest.TestCase):
    def test_exact_bound_policy(self):
        key = 'oteryn:creature.orshabaal'
        row = {'target': {'family': 'Creature', 'key': key, 'revision': 'definition-r1'},
               'data': {'profile': {'details': {'corpse_item':
                   {'family': 'Item', 'key': 'oteryn:item.tibia.i5995', 'revision': 'definition-r1'}}}}}
        self.assertEqual(corpses.roots({key: row}), {5995})
        row['data']['profile']['details']['corpse_item']['key'] = 'oteryn:item.tibia.i6006'
        with self.assertRaisesRegex(ValueError, 'SHARED_REWARD_BOSS_CORPSE_BINDING'):
            corpses.roots({key: row})

    def test_unmounted_policies_admit_nothing(self):
        self.assertEqual(corpses.roots({}), set())
