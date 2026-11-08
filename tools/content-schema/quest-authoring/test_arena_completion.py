import unittest
import arena_completion as arena


class ArenaCompletionTests(unittest.TestCase):
    def test_all_difficulties_and_all_three_trophies_are_required(self):
        binding = arena.expected()
        self.assertEqual([stage['goal'] for stage in binding['stages']], [1, 10, 10, 10, 3])
        self.assertEqual(binding['completed_rank'], 3)
        self.assertEqual(binding['trophy_claims'], [f'oteryn:reward.svargrond-arena.{rank}.4' for rank in range(1, 4)])
        self.assertTrue(binding['requires_premium'])
        self.assertEqual(binding['prerequisite'], 'oteryn:quest.barbarian_test')

    def test_native_binding_retains_the_canonical_quest_identity(self):
        binding = arena.expected()
        self.assertEqual(binding['quest'], arena.QUEST)
        self.assertTrue(all(stage['track'].startswith(arena.PREFIX) for stage in binding['stages']))
        self.assertEqual(len(binding['recipe_sha256']), 64)


if __name__ == '__main__':
    unittest.main()
