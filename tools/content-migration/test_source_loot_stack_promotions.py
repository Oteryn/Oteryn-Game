from pathlib import Path
import unittest

import validate_world_project_v2_to_tree as validator


class SourceLootStackPromotionProof(unittest.TestCase):
    def profile(self):
        return {'semantics': {'stack': {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': True},
            'stack_max': {'state': 'KNOWN', 'value': 100}}}}}

    def test_only_finite_typed_native_stack_limit_is_promotable(self):
        self.assertEqual(validator.source_loot_stack_max(self.profile()), 100)
        for maximum in (0, 99, 101, True, '100'):
            profile = self.profile()
            profile['semantics']['stack']['value']['stack_max']['value'] = maximum
            with self.subTest(maximum=maximum), self.assertRaisesRegex(
                    validator.ValidationError, 'SOURCE_LOOT_STACK_BOUND'):
                validator.source_loot_stack_max(profile)

    def test_unknown_or_nonstackable_facts_never_promote(self):
        for state in ('UNKNOWN', 'NOT_APPLICABLE'):
            profile = {'semantics': {'stack': {'state': state}}}
            self.assertIsNone(validator.source_loot_stack_max(profile))
        for stackable in (False, 1, None):
            profile = self.profile()
            profile['semantics']['stack']['value']['stackable']['value'] = stackable
            self.assertIsNone(validator.source_loot_stack_max(profile))
        profile = self.profile()
        del profile['semantics']['stack']['value']['stack_max']
        with self.assertRaisesRegex(validator.ValidationError, 'SOURCE_LOOT_STACK_BOUND'):
            validator.source_loot_stack_max(profile)

    def test_real_pinned_loot_scope_does_not_admit_unrelated_or_nonstackable_items(self):
        promotions = validator.source_loot_stack_promotions(Path(__file__).resolve().parents[2])
        self.assertEqual(promotions['oteryn:item.tibia.i3031'], 100)
        self.assertEqual(promotions['oteryn:item.tibia.i7368'], 100)
        self.assertNotIn('oteryn:item.tibia.i3082', promotions)
        self.assertNotIn('oteryn:item.tibia.i999999', promotions)
        self.assertEqual(set(promotions.values()), {100})


if __name__ == '__main__':
    unittest.main()
