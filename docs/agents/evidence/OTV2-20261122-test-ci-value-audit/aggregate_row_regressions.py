#!/usr/bin/env python3
"""Required row identities must survive planning, extraction and aggregation."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(os.environ.get('NODE_BOOT_MATRIX_SOURCE', Path(__file__).resolve().parents[4] / 'tools/qualification/node_boot/matrix.py'))
spec = importlib.util.spec_from_file_location('matrix_under_test', SOURCE)
matrix = importlib.util.module_from_spec(spec)
spec.loader.exec_module(matrix)


class RequiredRowEvidenceTests(unittest.TestCase):
    def summary(self, rows, lines, *, missing_fatal=True):
        shards = matrix.plan('custom', rows=rows)
        self.assertEqual(len(shards), 1)
        shard = shards[0]
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            log = directory / 'node.log'
            log.write_text(lines + 'NODE_BOOT_RESULT=NODE_BOOT_PASS\n')
            result = matrix.extract(log, name=shard['name'], entry=shard['entry'],
                                    exit_code=0, seconds=3)
            output = directory / shard['name']
            output.mkdir()
            (output / 'result.json').write_text(json.dumps(result))
            return matrix.summarize(shards, directory, '', missing_fatal=missing_fatal)

    def test_other_scenario_cannot_replace_the_selected_row(self):
        _, problems = self.summary('206:207',
                                   'SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n')
        self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_a_bounded_slice_or_full_row_cannot_replace_the_selected_client_row(self):
        for scenario in ('S09.logout', 'S09.full'):
            with self.subTest(scenario=scenario):
                _, problems = self.summary('206:207',
                    f'SEAM_EVIDENCE e2e_all scenario={scenario} status=PASS seconds=1\n')
                self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_partially_executed_batch_is_not_complete(self):
        _, problems = self.summary('1:3',
                                   'SEAM_EVIDENCE e2e_all scenario=C03.full status=PASS seconds=1\n')
        self.assertTrue(any('C12.full' in problem for problem in problems), problems)

    def test_duplicate_expected_row_is_not_one_completed_execution(self):
        line = 'SEAM_EVIDENCE e2e_all scenario=S09.client status=PASS seconds=1\n'
        _, problems = self.summary('206:207', line + line)
        self.assertTrue(any('duplicate' in problem and 'S09.client' in problem for problem in problems), problems)

    def test_capture_does_not_replace_a_terminal_verdict(self):
        for capture in (
            'SEAM_EVIDENCE e2e_capture scenario=S09.client status=CAPTURED evidence=logout\n',
            'SEAM_EVIDENCE e2e_all scenario=S09.client status=CAPTURED seconds=1\n',
        ):
            with self.subTest(capture=capture):
                _, problems = self.summary('206:207', capture +
                    'SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n')
                self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_missing_informational_does_not_waive_absent_evidence(self):
        _, problems = self.summary('206:207',
            'SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n', missing_fatal=False)
        self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_complete_terminal_evidence_can_include_extra_bounded_slices(self):
        _, problems = self.summary('206:207',
            'SEAM_EVIDENCE e2e_all scenario=S09.client status=PASS seconds=1\n'
            'SEAM_EVIDENCE e2e_all scenario=S09.logout status=PASS seconds=1\n'
            'SEAM_EVIDENCE e2e_capture scenario=S09.client status=CAPTURED evidence=logout\n')
        self.assertEqual(problems, [])

    def test_existing_failure_and_missing_status_policy_is_not_redefined(self):
        for status in ('FAIL', 'MISSING-SERVER', 'MISSING-CLIENT', 'SKIPPED'):
            with self.subTest(status=status):
                _, problems = self.summary('206:207',
                    f'SEAM_EVIDENCE e2e_all scenario=S09.client status={status} seconds=1\n',
                    missing_fatal=False)
                self.assertEqual(problems, [])

    def test_plan_records_ids_for_all_selected_rows_including_unwrapped_emitters(self):
        plan = matrix.plan('full')
        rows = [entry for shard in plan for entry in shard['expected_rows']]
        self.assertEqual(sorted(row['index'] for row in rows), matrix.row_indices())
        self.assertEqual(len({row['scenario'] for row in rows}), len(rows))
        by_index = {row['index']: row['scenario'] for row in rows}
        self.assertEqual({index: by_index[index] for index in (85, 86, 87, 206, 227)},
                         {85: 'S04b.full', 86: 'C14.full', 87: 'E02.full',
                          206: 'S09.client', 227: 'C07.client'})

    def test_non_row_lanes_do_not_claim_indexed_row_coverage(self):
        plan = matrix.plan('custom', groups='core;f4', basic=True, spells=True, game_platform=True)
        self.assertTrue(all(shard['expected_rows'] == [] for shard in plan))

    def test_reference_requires_its_selected_rows_as_well_as_existing_group_evidence(self):
        reference = matrix.plan('reference')[0]
        expected = {row['scenario'] for row in reference['expected_rows']}
        self.assertIn('C03.full', expected)
        self.assertIn('W23.full', expected)
        self.assertNotIn('S09.client', expected)


if __name__ == '__main__':
    unittest.main()
