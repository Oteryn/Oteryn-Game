"""Source diagnostics are prose; only exact progress keys are graph references."""
import json
import unittest
import bundle_semantics as s


class ProgressReferenceShapeTests(unittest.TestCase):
    def data(self, value):
        return {'quests': [], 'claims': [], 'gates': [], 'interactions': [], 'progress': [{
            'key': 'canary:quest-progress/quest/example/declared', 'missions': [],
            'start_of': [], 'read_by_gates': [], 'source_checks': {'storage_declaration': value}}]}

    def test_unknown_diagnostic_is_not_an_undeclared_progress_reference(self):
        value = 'UNKNOWN: no exact Storage declaration for canary:quest-progress/quest/example/declared'
        data = self.data(value)
        self.assertEqual(s.reference_gaps(data), [])
        self.assertEqual(data['progress'][0]['source_checks']['storage_declaration'], value)

    def test_exact_reference_remains_a_real_gap(self):
        for namespace in ['canary', 'crystalserver']:
            target = namespace + ':quest-progress/quest/example/missing'
            data = self.data('UNKNOWN: declaration')
            data['interactions'] = [{'identity': {'key': 'canary:interaction/example'},
                                     'rules': [{'quest_stage': {'progress': target}}]}]
            gaps = [json.loads(x) for x in s.reference_gaps(data)]
            self.assertEqual(len(gaps), 1)
            self.assertEqual(gaps[0]['target_key'], target)
            self.assertEqual(gaps[0]['field'], '/rules/0/quest_stage/progress')

    def test_substrings_and_trailing_suffixes_are_not_typed_keys(self):
        key = 'canary:quest-progress/quest/example/missing'
        for value in ['look up ' + key, key + ' trailing prose', key + '#mission',
                      key + '?source=canary', key + '/', 'other:quest-progress/quest/example/missing']:
            with self.subTest(value=value):
                self.assertEqual(s.reference_gaps(self.data(value)), [])

    def test_structured_family_reference_still_qualifies(self):
        target = 'canary:quest-progress/quest/example/missing'
        data = self.data('UNKNOWN: declaration')
        data['interactions'] = [{'identity': {'key': 'canary:interaction/example'},
                                 'condition': {'family': 'Progress', 'key': target, 'revision': 1}}]
        self.assertEqual(json.loads(s.reference_gaps(data)[0])['target_key'], target)


if __name__ == '__main__':
    unittest.main()
