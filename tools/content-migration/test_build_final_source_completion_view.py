import copy
from pathlib import Path
import tempfile
import unittest

import build_final_source_completion_view as completion
import build_source_spell_review_index as index


class PrivateSourceCompletionReviewTests(unittest.TestCase):
    def fixture(self):
        row = {'registration_key': 'canary-main-current/example.lua#1', 'status': 'CANDIDATE_SCHEMA_VALID'}
        receipt = {**row}
        projection = {**row, 'target_schema_family': 'private_source_complete_v2',
            'authoring_contract_extension_pending': True, 'source_consumer_implemented': False,
            'runtime_activation': False, 'native_execution_qualified': False,
            'input_provider_equivalence': False, 'reader_acceptance_qualified': False,
            'native_data_model_complete': True}
        return row, receipt, projection

    def test_private_data_preserves_pending_contract_and_absent_consumer(self):
        row, receipt, projection = self.fixture()
        qualified = index.private_qualification(row, receipt, projection)
        self.assertTrue(qualified['authoring_contract_extension_pending'])
        self.assertFalse(qualified['source_consumer_implemented'])
        self.assertFalse(qualified['reader_acceptance_qualified'])
        self.assertFalse(qualified['input_provider_equivalence'])
        completion.check_record(row, {'registration_key': row['registration_key'], 'structural_status': row['status'], 'candidate_set': 'r59', **qualified}, 59)

    def test_contract_consumer_or_execution_promotion_refused(self):
        row, receipt, projection = self.fixture()
        for field, value in [('authoring_contract_extension_pending', False), ('source_consumer_implemented', True),
                             ('native_execution_qualified', True), ('runtime_activation', True), ('input_provider_equivalence', True)]:
            changed = copy.deepcopy(projection); changed[field] = value
            with self.assertRaisesRegex(ValueError, 'qualification mismatch'):
                index.private_qualification(row, receipt, changed)
            current = {'registration_key': row['registration_key'], 'structural_status': row['status'], 'candidate_set': 'r59', **changed}
            with self.assertRaisesRegex(ValueError, 'cannot qualify'):
                completion.check_record(row, current, 59)

    def test_private_family_and_receipt_identity_are_required(self):
        row, receipt, projection = self.fixture()
        changed = dict(receipt, registration_key='other')
        with self.assertRaisesRegex(ValueError, 'qualification mismatch'): index.private_qualification(row, changed, projection)
        changed = dict(projection, target_schema_family='accepted_runtime_family')
        with self.assertRaisesRegex(ValueError, 'qualification mismatch'): index.private_qualification(row, receipt, changed)

    def test_removed_and_disabled_references_remain_blocked(self):
        for status in completion.REFERENCES:
            row = {'registration_key': 'reference', 'status': status}
            current = {'registration_key': 'reference', 'structural_status': 'BLOCKED', 'reference_only_status': status, 'reference_set': 'r59'}
            completion.check_record(row, current, 59)
            current['structural_status'] = 'CANDIDATE_SCHEMA_VALID'
            with self.assertRaisesRegex(ValueError, 'reference-only registration promoted'): completion.check_record(row, current, 59)

    def test_reference_projection_keeps_separate_standard_blocked_receipt(self):
        row = {'registration_key': 'source/reference#1', 'status': 'REFERENCE_ONLY_DISABLED'}
        standard = {'registration_key': row['registration_key'], 'status': 'BLOCKED'}
        projection = dict(row)
        index.verify_reference_receipts(row, standard, projection)
        with self.assertRaisesRegex(ValueError, 'reference receipt identity mismatch'):
            index.verify_reference_receipts(row, dict(standard, status=row['status']), projection)
        with self.assertRaisesRegex(ValueError, 'reference receipt identity mismatch'):
            index.verify_reference_receipts(row, standard, dict(projection, status='BLOCKED'))

    def test_candidate_and_reference_must_belong_to_the_owning_import(self):
        row, receipt, projection = self.fixture()
        current = {'registration_key': row['registration_key'], 'structural_status': row['status'],
                   'candidate_set': 'r60', **index.private_qualification(row, receipt, projection)}
        with self.assertRaisesRegex(ValueError, 'cannot qualify'): completion.check_record(row, current, 59)
        row = {'registration_key': 'reference', 'status': 'REFERENCE_ONLY_DISABLED'}
        current = {'registration_key': 'reference', 'structural_status': 'BLOCKED',
                   'reference_only_status': row['status'], 'reference_set': 'r62'}
        with self.assertRaisesRegex(ValueError, 'reference-only registration promoted'): completion.check_record(row, current, 59)

    def test_no_output_created_when_imports_not_materialized(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(FileNotFoundError): completion.build(root)
            self.assertEqual(list(root.iterdir()), [])


if __name__ == '__main__': unittest.main()
