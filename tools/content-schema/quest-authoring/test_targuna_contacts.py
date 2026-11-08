"""Only Sterling's selected first contact starts this nine-stage quest."""
import unittest
import json
from unittest.mock import patch
import targuna_contacts as author


class TargunaContacts(unittest.TestCase):
    def test_first_contact_is_free_once_only_without_quest_completion_or_rewards(self):
        rows, greetings = [], []
        author.extend_rewards(rows, greetings)
        row, = [row for row in rows if row['npc'] == 'oteryn:npc.sterling']
        self.assertEqual(row['npc'], 'oteryn:npc.sterling')
        self.assertEqual(row['quest'], 'oteryn:quest.targuna_quest')
        self.assertEqual(row['storage'], [dict(track='oteryn:quest-progress/targuna/stage', minimum=-1, maximum=-1, set=0)])
        self.assertEqual(row['take'], [])
        self.assertIsNone(row['give'])
        self.assertIsNone(row['outfit'])
        self.assertIsNone(row['achievement'])
        self.assertFalse(row.get('completes', False))
        self.assertFalse(row.get('premium_required', False))
        self.assertNotIn('minimum_level', row)
        self.assertEqual(greetings[0]['npc'], row['npc'])
        self.assertTrue(row['sources'][0].startswith('oteryn:selected-recipe:'))

    def test_existing_unrelated_rows_and_greetings_are_preserved(self):
        old = {'key': 'unrelated'}
        rows, greetings = [old.copy()], [old.copy()]
        author.extend_rewards(rows, greetings)
        self.assertEqual(rows[0], old)
        self.assertEqual(greetings[0], old)
        self.assertEqual(len(rows), 3)
        self.assertEqual(len(greetings), 3)

    def test_full_producer_preserves_every_unrelated_callback_and_greeting(self):
        import qengine_rewards
        with patch.object(author, 'extend_rewards', lambda rows, greetings: None):
            before = qengine_rewards.build()
        after = qengine_rewards.build()
        for path, field, selector in (
                ('content/quests/shared-reward-callbacks.json', 'callbacks', 'npc'),
                ('content/dialogues/quests/shared-reward-greetings.json', 'npcs', 'npc')):
            expected = json.loads(before[path])[field]
            actual = json.loads(after[path])[field]
            self.assertEqual([row for row in actual if row[selector] not in {'oteryn:npc.sterling', 'oteryn:npc.morla'}], expected)
            self.assertEqual(len(actual), len(expected) + 2)


if __name__ == '__main__':
    unittest.main()
