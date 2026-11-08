"""The chosen map contact is intermediate progress, with no whole-quest completion."""
import unittest
import targuna_contacts as author


class MorlaMap(unittest.TestCase):
    def test_map_is_exact_free_and_once_after_sterling_acceptance(self):
        callbacks, greetings = [], []
        author.extend_rewards(callbacks, greetings)
        matches = [row for row in callbacks if row['npc'] == 'oteryn:npc.morla']
        self.assertEqual(len(matches), 1)
        row, = matches
        self.assertEqual(row['quest'], 'oteryn:quest.targuna_quest')
        self.assertEqual(row['storage'], [dict(track=author.TRACK, minimum=0, maximum=0, set=1)])
        self.assertEqual(row['give'], dict(item='oteryn:item.tibia.i53158', count=1, contents=[]))
        self.assertEqual(row['take'], [])
        self.assertIsNone(row['outfit'])
        self.assertIsNone(row['achievement'])
        self.assertFalse(row.get('completes', False))
        self.assertFalse(row.get('requires_premium', False))
        self.assertNotIn('fee', row)
        self.assertNotIn('minimum_level', row)
        self.assertEqual(sum(g['npc'] == 'oteryn:npc.morla' for g in greetings), 1)
        self.assertTrue(any('data-global/npc/morla.lua' in s for s in row['sources']))


if __name__ == '__main__':
    unittest.main()
