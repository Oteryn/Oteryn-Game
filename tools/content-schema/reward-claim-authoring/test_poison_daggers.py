"""The target correction is limited to the exact source chest and full recipe."""
import copy
import unittest

import poison_daggers as quest


class PoisonDaggersTargetTests(unittest.TestCase):
    def test_target_preserves_arrows_and_serializes_two_physical_daggers(self):
        pairs = [[3448, 30], [3299, 1]]
        self.assertEqual(quest.source_reward(6068, quest.POSITION, pairs),
                         [[3448, 30], [3299, 1], [3299, 1]])
        self.assertEqual(pairs, [[3448, 30], [3299, 1]])

    def test_wrong_source_position_or_recipe_is_rejected(self):
        for position, pairs in [
                ({**quest.POSITION, 'x': 33156}, quest.SOURCE),
                (quest.POSITION, [[3448, 29], [3299, 1]]),
                (quest.POSITION, [[3448, 30], [3299, 2]]),
                (quest.POSITION, quest.TARGET)]:
            with self.subTest(position=position, pairs=pairs):
                with self.assertRaisesRegex(ValueError, 'POISON_DAGGERS_SOURCE_DRIFT'):
                    quest.source_reward(6068, position, pairs)

    def test_other_chests_keep_their_source_reward(self):
        self.assertEqual(quest.source_reward(6069, quest.POSITION, quest.SOURCE), quest.SOURCE)
        unrelated = {'items': []}
        quest.correct_canonical('canary:reward-claim/kv/orcshaman', quest.POSITION, unrelated)
        self.assertEqual(unrelated, {'items': []})

    def test_canonical_correction_preserves_source_and_copies_the_second_instance(self):
        def ref(item):
            return {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}',
                    'revision': 'definition-r1'}

        original = {'container': ref(2854), 'items': [
            {'item': ref(item), 'count': count} for item, count in quest.SOURCE]}
        reward = copy.deepcopy(original)
        quest.correct_canonical(quest.PILOT, quest.POSITION, reward)
        self.assertEqual(reward['items'], original['items'] + [original['items'][1]])
        self.assertIsNot(reward['items'][1], reward['items'][2])
        self.assertIsNot(reward['items'][1]['item'], reward['items'][2]['item'])
        for changed in [reward, {**original, 'container': ref(2853)},
                        {**original, 'key_binding': 'canary:door-key/6010'}]:
            with self.subTest(changed=changed):
                with self.assertRaisesRegex(ValueError, 'POISON_DAGGERS_CANONICAL_DRIFT'):
                    quest.correct_canonical(quest.PILOT, quest.POSITION, changed)
