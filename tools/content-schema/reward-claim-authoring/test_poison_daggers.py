"""The exact source chest and recipe retain their numeric quantities."""
import copy
import unittest

import poison_daggers as quest


class PoisonDaggersTargetTests(unittest.TestCase):
    def test_preserves_arrows_and_one_dagger(self):
        pairs = [[3448, 30], [3299, 1]]
        self.assertEqual(quest.source_reward(6068, quest.POSITION, pairs),
                         [[3448, 30], [3299, 1]])
        self.assertEqual(pairs, [[3448, 30], [3299, 1]])

    def test_wrong_source_position_or_recipe_is_rejected(self):
        for position, pairs in [
                ({**quest.POSITION, 'x': 33156}, quest.SOURCE),
                (quest.POSITION, [[3448, 29], [3299, 1]]),
                (quest.POSITION, [[3448, 30], [3299, 2]]),
                (quest.POSITION, [[3448, 30], [3299, 1], [3299, 1]])]:
            with self.subTest(position=position, pairs=pairs):
                with self.assertRaisesRegex(ValueError, 'POISON_DAGGERS_SOURCE_DRIFT'):
                    quest.source_reward(6068, position, pairs)

    def test_other_chests_keep_their_source_reward(self):
        self.assertEqual(quest.source_reward(6069, quest.POSITION, quest.SOURCE), quest.SOURCE)
        unrelated = {'items': []}
        quest.verify_canonical('canary:reward-claim/kv/orcshaman', quest.POSITION, unrelated)
        self.assertEqual(unrelated, {'items': []})

    def test_canonical_preserves_the_exact_source_recipe(self):
        def ref(item):
            return {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}',
                    'revision': 'definition-r1'}

        original = {'container': ref(2854), 'items': [
            {'item': ref(item), 'count': count} for item, count in quest.SOURCE]}
        reward = copy.deepcopy(original)
        quest.verify_canonical(quest.PILOT, quest.POSITION, reward)
        self.assertEqual(reward, original)
        for changed in [
                {**original, 'items': original['items'] + [original['items'][1]]},
                {**original, 'container': ref(2853)},
                        {**original, 'key_binding': 'canary:door-key/6010'}]:
            with self.subTest(changed=changed):
                with self.assertRaisesRegex(ValueError, 'POISON_DAGGERS_CANONICAL_DRIFT'):
                    quest.verify_canonical(quest.PILOT, quest.POSITION, changed)
