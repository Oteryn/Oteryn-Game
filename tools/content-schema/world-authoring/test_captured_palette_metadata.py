"""Closed-scope captured membership must reject new or reclassified identities."""
import copy
import unittest

from refresh_captured_palette_metadata import closed_membership


class CapturedMembershipTests(unittest.TestCase):
    def setUp(self):
        self.before = [
            {'source_item_id': 1, 'provisional': True},
            {'source_item_id': 2, 'provisional': True},
            {'source_item_id': 3, 'provisional': False},
        ]
        self.sample = {'rows': [{'id': 2}]}

    def test_only_removing_captured_appearance_provisional_status_is_allowed(self):
        after = copy.deepcopy(self.before)
        after[1]['provisional'] = False
        self.assertEqual(closed_membership(self.before, after, self.sample, {1}, {1, 2, 3}), {1})

    def test_new_palette_identity_is_refused(self):
        after = copy.deepcopy(self.before)
        after[1]['source_item_id'] = 4
        with self.assertRaisesRegex(ValueError, 'identities changed'):
            closed_membership(self.before, after, self.sample, {1}, {1, 2, 3})

    def test_previously_noncaptured_provisional_identity_is_refused(self):
        after = copy.deepcopy(self.before)
        after[2]['provisional'] = True
        with self.assertRaisesRegex(ValueError, 'outside closed scope'):
            closed_membership(self.before, after, self.sample, {1}, {1, 2, 3})

    def test_captured_nonclient_placeholder_remains_unknown(self):
        before = self.before + [{'source_item_id': 99, 'provisional': True}]
        self.assertEqual(closed_membership(before, before, self.sample, {1}, {1, 2, 3}), {1})

    def test_incomplete_captured_evidence_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'incomplete'):
            closed_membership(self.before, self.before, {'rows': []}, {1}, {1, 2, 3})


if __name__ == '__main__':
    unittest.main()
