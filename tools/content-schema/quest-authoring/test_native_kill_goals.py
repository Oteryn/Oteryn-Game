"""The executable native-goal capture must reproduce its committed source bindings."""
import json
import unittest

import native_kill_goals as goals


class NativeKillGoals(unittest.TestCase):
    def test_committed_goal_reproduces_exact_native_origins_and_chosen_cap(self):
        expected = goals.document()
        self.assertEqual(json.loads(goals.OUTPUT.read_text()), expected)
        goal, = expected['goals']
        self.assertEqual(goal['count'], 8)
        self.assertEqual(len(goal['origins']), 21)
        self.assertEqual({p['creature'] for p in goal['origins']},
                         {'oteryn:creature.dragon', 'oteryn:creature.dragon_lord'})
        self.assertEqual(len({(p['x'], p['y'], p['floor'], p['creature'])
                              for p in goal['origins']}), 21)


if __name__ == '__main__':
    unittest.main()
