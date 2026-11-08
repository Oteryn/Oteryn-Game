"""Definition witness refresh cannot silently change Source associations."""
import copy
import unittest

import refresh_donor_crosswalk_witness as tool


class CrosswalkWitnessTests(unittest.TestCase):
    def setUp(self):
        self.previous = {'input_sha256': {'quest.json': 'old'},
                         'summary': {'components': 1}, 'components': [{'owner': 'exact'}]}
        self.expected = copy.deepcopy(self.previous)
        self.expected['input_sha256']['quest.json'] = 'new'
        self.coverage = {'component_batch': {'crosswalk': {'components': 1}},
                         'outputs': [{'path': tool.CROSSWALK, 'sha256': 'old'},
                                     {'path': 'other.json', 'sha256': 'untouched'}]}

    def test_only_replayed_witness_digest_changes(self):
        result = tool.amend(self.coverage, self.previous, self.expected, b'producer output')
        self.assertEqual(result['outputs'][0]['sha256'], tool.digest(b'producer output'))
        self.assertEqual(result['outputs'][1], self.coverage['outputs'][1])
        self.assertEqual(self.coverage['outputs'][0]['sha256'], 'old')

    def test_changed_association_requires_full_replay(self):
        self.expected['components'][0]['owner'] = 'invented'
        with self.assertRaisesRegex(ValueError, 'associations changed'):
            tool.amend(self.coverage, self.previous, self.expected, b'changed')

    def test_changed_summary_requires_full_replay(self):
        self.coverage['component_batch']['crosswalk']['components'] = 2
        with self.assertRaisesRegex(ValueError, 'summary changed'):
            tool.amend(self.coverage, self.previous, self.expected, b'changed')

    def test_npc_file_witness_changes_leave_occurrences_unchanged(self):
        old = {'quest_inputs': [{'sha256': 'old'}], 'occurrences': ['exact']}
        new = {'quest_inputs': [{'sha256': 'new'}], 'occurrences': ['exact']}
        tool.require_same(old, new, ('quest_inputs',))
        new['occurrences'] = ['invented']
        with self.assertRaisesRegex(ValueError, 'capture semantics changed'):
            tool.require_same(old, new, ('quest_inputs',))

    def test_dependency_digest_cannot_hide_changed_include(self):
        old = {'input_provenance': {'npc_capture_sha256': 'old'}, 'includes': ['exact']}
        new = {'input_provenance': {'npc_capture_sha256': 'new'}, 'includes': ['other']}
        with self.assertRaisesRegex(ValueError, 'capture semantics changed'):
            tool.require_same(old, new, ('input_provenance', 'npc_capture_sha256'))

    def test_duplicate_descriptor_rejected(self):
        self.coverage['outputs'].append(copy.deepcopy(self.coverage['outputs'][0]))
        with self.assertRaisesRegex(ValueError, 'exactly once'):
            tool.amend(self.coverage, self.previous, self.expected, b'changed')


if __name__ == '__main__':
    unittest.main()
