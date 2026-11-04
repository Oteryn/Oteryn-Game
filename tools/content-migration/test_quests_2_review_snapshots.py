"""Reject lost source rows, inflated delivery claims and unverifiable evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import quests_2_review_snapshots as review


class FixtureSources:
    def witness(self, ref):
        return review.witness(ref, b'-- pinned fixture\nreturn true\n')


def fixture():
    source = f"zimbadev/crystalserver@{review.PINS['zimbadev/crystalserver']}:data-global/npc/fixture.lua:1-2"
    row = {'quest': 'mysterious_ornate', 'step': 'Source step', 'patterns': ['P09'],
           'trigger': 'fixture', 'condition': 'source condition', 'effect': 'source effect',
           'donor': [source], 'oteryn': 'NIE', 'import_method': 'RĘCZNIE', 'notes': ''}
    disposition = {'index': 1, 'quest': 'mysterious_ornate', 'step': 'Source step',
                   'original_import': 'RĘCZNIE', 'status': 'LEFT_OUT',
                   'reason': 'An explicitly deferred source owner.', 'sources': [source],
                   'answer': 'https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017'}
    return row, disposition


class ReviewSnapshotTests(unittest.TestCase):
    def make(self):
        return review.make_document('mysterious_ornate', 'classic', [fixture()],
                                    FixtureSources(), [], {})

    def test_check_is_self_contained_and_original_values_survive(self):
        document = self.make()
        row, disposition = fixture()
        self.assertEqual(document['rows'][0]['source_row'], row)
        self.assertEqual(document['rows'][0]['source_disposition'], disposition)
        self.assertTrue(document['rows'][0]['normalized']['left_out'])
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            (output / 'mysterious_ornate.json').write_text(json.dumps(document))
            checked = review.check_collection(output, expected_quests=1)
            self.assertEqual(checked['inventory_rows'], 1)
            self.assertFalse(checked['pinned_donor_bytes_verified'])
            with self.assertRaisesRegex(ValueError, 'Expected 21'):
                review.check_collection(output)

    def test_duplicate_or_missing_dispositions_cannot_reduce_coverage(self):
        row, disposition = fixture()
        for rows, dispositions in [([row, row], [disposition]), ([row], []),
                                   ([row], [disposition, disposition])]:
            with self.assertRaises(ValueError):
                review.join_rows('classic', rows, dispositions)

    def test_modified_normalized_claim_and_source_fact_are_rejected(self):
        document = self.make()
        forged = copy.deepcopy(document)
        forged['rows'][0]['normalized']['status'] = 'IMPLEMENTED'
        forged['summary'] = review.summary(forged['rows'])
        with self.assertRaisesRegex(ValueError, 'Normalized disposition drift'):
            review.check_document(forged)
        forged = copy.deepcopy(document)
        forged['rows'][0]['source_row']['effect'] = 'invented reward'
        with self.assertRaisesRegex(ValueError, 'Original source row digest differs'):
            review.check_document(forged)

    def test_unknown_status_or_left_out_without_decision_is_rejected(self):
        _, disposition = fixture()
        for replacement in ({'status': 'PROBABLY_IMPLEMENTED'}, {'answer': None}):
            with self.assertRaises(ValueError):
                review.normalize('classic', dict(disposition, **replacement))

    def test_donor_pin_path_and_line_range_are_verified(self):
        row, _ = fixture()
        ref = review.reference(row['donor'][0])
        for value in [row['donor'][0].replace(review.PINS['zimbadev/crystalserver'], '0' * 40),
                      row['donor'][0].replace('data-global/npc/fixture.lua', '../fixture.lua'),
                      row['donor'][0].replace(':1-2', ':3-2')]:
            with self.assertRaises(ValueError):
                review.reference(value)
        with self.assertRaisesRegex(ValueError, 'Source range exceeds blob'):
            review.witness(ref, b'only one line\n')
        changed = self.make()
        changed['rows'][0]['donor_witnesses'][0]['blob_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'Pinned donor bytes or excerpt differ'):
            review.check_document(changed, FixtureSources())

    def test_count_changes_and_pattern_invention_are_rejected(self):
        self.assertEqual(len(review.QUEST_ROWS), 21)
        self.assertEqual(sum(review.QUEST_ROWS.values()), 1390)
        document = self.make()
        document['rows'].append(copy.deepcopy(document['rows'][0]))
        with self.assertRaisesRegex(ValueError, 'Frozen count differs'):
            review.check_document(document)
        document = self.make()
        document['rows'][0]['source_row']['patterns'] = ['P31']
        document['source_rows_sha256'] = review.digest(review.canonical(
            [row['source_row'] for row in document['rows']]))
        with self.assertRaisesRegex(ValueError, 'Pattern outside P01-P30'):
            review.check_document(document)

    def test_explicit_new_pattern_note_preserves_the_absence_of_an_invented_id(self):
        row, disposition = fixture()
        row['patterns'] = []
        with self.assertRaisesRegex(ValueError, 'Empty pattern list'):
            review.make_document('mysterious_ornate', 'classic', [(row, disposition)],
                                 FixtureSources(), [], {})
        row['notes'] = 'NOWE WZORCE: literal source speech without a newly assigned ID.'
        document = review.make_document('mysterious_ornate', 'classic', [(row, disposition)],
                                        FixtureSources(), [], {})
        self.assertEqual(document['rows'][0]['source_row']['patterns'], [])
        self.assertEqual(document['summary']['new_pattern_rows_without_assigned_id'], 1)

    def test_root_partial_callback_is_not_promoted_to_implemented(self):
        original, reference_disposition = fixture()
        disposition = {'row': 1, 'quest': original['quest'], 'inventory': original,
                       'disposition': 'AUTHORED_EXISTING_CALLBACK_CORE', 'completion': 'PARTIAL',
                       'reason': 'The callback core is authored; presentation and physical admission remain held.',
                       'question_or_answer': [reference_disposition['answer']]}
        document = review.make_document('mysterious_ornate', 'root', [(original, disposition)],
                                        FixtureSources(), [], {})
        self.assertEqual(document['rows'][0]['normalized']['status'], 'PARTIAL')
        self.assertEqual(document['summary']['normalized_status'], {'PARTIAL': 1})
        self.assertEqual(document['rows'][0]['source_disposition'], disposition)
        disposition['disposition'] = 'LEFT_OUT'
        disposition['completion'] = 'IMPLEMENTED'
        with self.assertRaisesRegex(ValueError, 'Root completion contradicts'):
            review.make_document('mysterious_ornate', 'root', [(original, disposition)],
                                 FixtureSources(), [], {})

    def test_root_header_cannot_override_actual_auto_manual_counts(self):
        original, reference_disposition = fixture()
        disposition = {'quest': original['quest'], 'inventory': original,
                       'disposition': 'LEFT_OUT', 'completion': 'LEFT_OUT',
                       'reason': reference_disposition['reason'],
                       'question_or_answer': [reference_disposition['answer']]}
        context = {'inventory_rows': 1, 'summary': {'LEFT_OUT': 1},
                   'auto_rows': 1, 'manual_rows': 0}
        with self.assertRaisesRegex(ValueError, 'Root declared auto_rows differs'):
            review.make_document('mysterious_ornate', 'root', [(original, disposition)],
                                 FixtureSources(), [], context)
        context.update(auto_rows=0, manual_rows=1)
        document = review.make_document('mysterious_ornate', 'root', [(original, disposition)],
                                        FixtureSources(), [], context)
        self.assertEqual(document['summary']['import_kinds'], {'RĘCZNIE': 1})
        document['source_context']['summary'] = {'IMPLEMENTED': 1}
        with self.assertRaisesRegex(ValueError, 'Root declared summary differs'):
            review.check_document(document)


if __name__ == '__main__':
    unittest.main()
