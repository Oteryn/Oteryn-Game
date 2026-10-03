"""A chest's quest owner uses the same exact curator identity as quest scripts."""
import unittest
from unittest.mock import patch
import ots_chests
import ots_doors
import ots_interactions


class QuestIdentityTests(unittest.TestCase):
    def test_explicit_progress_alias_keeps_mission_identity_and_separators(self):
        canonical = {'key': 'canary:quest-progress/example/a_b'}
        alias = {'key': 'crystalserver:quest-progress/example/a_b', 'alias_of': canonical['key']}
        distinct = {'key': 'canary:quest-progress/example/ab'}
        expected = {'example/a_b': canonical['key'], 'example/ab': distinct['key']}
        self.assertEqual(ots_interactions.declared_progress_paths([canonical, alias, distinct]), expected)
        self.assertEqual(ots_interactions.declared_progress_paths([distinct, alias, canonical]), expected)
        alias['alias_of'] = distinct['key']
        with self.assertRaises(ValueError):
            ots_interactions.declared_progress_paths([canonical, alias, distinct])

    def test_shared_npc_gate_does_not_borrow_namespace_owner(self):
        path = 'quest/u7_9/nightmare_outfits/knightwatch_tower_door'
        gate = {'condition': {'progress': 'canary:quest-progress/' + path}, 'label': None,
                'quest': {'key': 'canary:quest/nightmare_outfits_quest'}, 'quest_link_basis': 'storage_key'}
        owners = {path: {'shared_npc_gate': True, 'note': 'two distinct outfit quests'}}
        hold = ots_doors.link_gate_quest(gate, lambda _: self.fail('ambiguous owner must not be inferred'), owners)
        self.assertIsNone(gate['quest'])
        self.assertIn('UNKNOWN', hold)

    def test_desert_curated_key_matches_script_identity(self):
        with patch.object(ots_chests, 'SCRIPT_QUEST_KEYS', {'The Desert Dungeon Quest': 'desert_dungeon_quest'}):
            self.assertEqual(ots_chests.quest_key({'title': 'The Desert Dungeon Quest', 'canary': 'IMPLEMENTED'}),
                             'canary:quest/desert_dungeon_quest')

    def test_noncurated_title_keeps_original_identity(self):
        with patch.object(ots_chests, 'SCRIPT_QUEST_KEYS', {}):
            self.assertEqual(ots_chests.quest_key({'title': 'The Desert Dungeon Quest', 'canary': 'IMPLEMENTED'}),
                             'canary:quest/the_desert_dungeon_quest')

    def test_matching_is_exact_and_retains_source_namespace(self):
        with patch.object(ots_chests, 'SCRIPT_QUEST_KEYS', {'The Desert Dungeon Quest': 'desert_dungeon_quest'}):
            self.assertEqual(ots_chests.quest_key({'title': 'The Desert Dungeon Quest', 'canary': 'ABSENT'}),
                             'crystalserver:quest/desert_dungeon_quest')
            self.assertEqual(ots_chests.quest_key({'title': 'Desert Dungeon Quest', 'canary': 'IMPLEMENTED'}),
                             'canary:quest/desert_dungeon_quest')


if __name__ == '__main__':
    unittest.main()
