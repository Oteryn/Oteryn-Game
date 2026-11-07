"""Known-value conflicts remain fatal across nested native equipment facts."""
import unittest

from native_profile_composition import merge_fact


class NativeProfileComposition(unittest.TestCase):
    def test_nested_ordered_patterns_only_enrich_unknown_facts(self):
        old = {'state': 'KNOWN', 'value': [
            {'pattern_id': 1, 'compatibility_rule': {'state': 'UNKNOWN'}}]}
        source = {'state': 'KNOWN', 'value': [
            {'pattern_id': 1, 'compatibility_rule': {'state': 'NOT_APPLICABLE'}}]}
        self.assertEqual(merge_fact(old, source, 'patterns'), source)
        self.assertEqual(old['value'][0]['compatibility_rule'], {'state': 'UNKNOWN'})
        self.assertEqual(merge_fact(source, old, 'patterns'), source)

    def test_conflicting_values_types_states_and_pattern_identities_are_refused(self):
        for old, new in [(1, 2), (1, True),
                         ({'state': 'KNOWN', 'value': 1},
                          {'state': 'KNOWN', 'value': True}),
                         ({'state': 'NOT_APPLICABLE'}, {'state': 'KNOWN', 'value': 1}),
                         ([{'pattern_id': 1}], [{'pattern_id': 2}]),
                         ([1], [1, 2]), ({'a': 1}, {'b': 1})]:
            with self.subTest(old=old, new=new):
                with self.assertRaisesRegex(ValueError, 'NATIVE_PROFILE_FACT_CONFLICT'):
                    merge_fact(old, new, 'profile')


if __name__ == '__main__':
    unittest.main()
