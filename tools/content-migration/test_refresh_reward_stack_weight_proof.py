"""A weight correction cannot reseal changed reward or stack semantics."""
import copy
import unittest

from refresh_reward_stack_weight_proof import qualified_digest, semantic_digest


class RewardStackWeightProof(unittest.TestCase):
    def setUp(self):
        self.before = {'identity': {'key': 'oteryn:item.tibia.i3031'}, 'semantics': {
            'physical': {'state': 'KNOWN', 'value': {
                'weight': {'state': 'KNOWN', 'value': 1},
                'movable': {'state': 'KNOWN', 'value': True}}},
            'stack': {'state': 'KNOWN', 'value': {'stack_max': 100}}}}
        self.after = copy.deepcopy(self.before)
        self.after['semantics']['physical']['value']['weight']['value'] = 10

    def test_accepts_only_the_verified_global_weight_leaf(self):
        self.assertEqual(qualified_digest(self.before, self.after,
                         semantic_digest(self.before), 10), semantic_digest(self.after))

    def test_rejects_a_changed_baseline(self):
        with self.assertRaisesRegex(ValueError, 'BASE_DEFINITION_DRIFT'):
            qualified_digest(self.before, self.after, '0' * 64, 10)

    def test_rejects_a_non_global_weight(self):
        with self.assertRaisesRegex(ValueError, 'NON_GLOBAL_WEIGHT_CHANGE'):
            qualified_digest(self.before, self.after, semantic_digest(self.before), 100)

    def test_rejects_other_physical_and_stack_changes(self):
        for group, field, value in [('physical', 'movable', False),
                                     ('stack', 'stack_max', 99)]:
            changed = copy.deepcopy(self.after)
            changed['semantics'][group]['value'][field] = value
            with self.subTest(group=group):
                with self.assertRaisesRegex(ValueError, 'NON_GLOBAL_WEIGHT_CHANGE'):
                    qualified_digest(self.before, changed, semantic_digest(self.before), 10)


if __name__ == '__main__':
    unittest.main()
