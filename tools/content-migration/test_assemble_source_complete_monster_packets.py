import copy
import unittest
import subprocess
import sys
from unittest.mock import patch

import assemble_source_complete_monster_packets as assembler
import source_complete_monster_common as common


class MonsterPacketCompositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = common.load_inputs()

    def row(self):
        original = next(iter(self.inputs[2][63].values()))
        return {name: copy.deepcopy(original[name]) for name in (
            'slot_identity', 'source', 'monster', 'original_slot_sha256', 'source_parameters')} | {
            'controller': {'kind': 'undefined_signed_health', 'parameters': {'source_model': 'fixture'}},
            'source_proofs': [{'path': 'fixture', 'sha256': 'a' * 64}],
            'target_schedule': {'interval_ms': 2000}, 'status': 'CANDIDATE_SCHEMA_VALID',
            'full_slot_projection_complete': True,
            'target_schema_family': common.PRODUCER_FAMILIES[63]}

    def validate(self, row):
        identity = common.key(row['slot_identity'])
        expected = self.inputs[2][63][identity]
        with patch.object(common, 'load_inputs', return_value=(None, None, {63: {identity: expected}})):
            return common.verify_rows(63, [row])

    def test_exact_cohort_and_producer_status_normalization_retain_pending_execution(self):
        self.assertEqual([len(self.inputs[2][r]) for r in range(63, 67)], [10, 21, 51, 83])
        row = self.row()
        self.validate(row)
        completed = assembler.complete(row)
        self.assertEqual(completed['source_parameters'], row['source_parameters'])
        self.assertEqual(completed['status'], 'SOURCE_SCHEMA_VALID')
        self.assertTrue(completed['authoring_contract_extension_pending'])
        self.assertFalse(completed['source_consumer_implemented'])
        self.assertFalse(completed['runtime_activation'])

    def test_source_drift_partial_missing_operations_alias_and_native_admission_refused(self):
        for field, value in [('status', 'BLOCKED'), ('full_slot_projection_complete', False),
                             ('required_operations_unrepresented', ['missing_callback']),
                             ('source_alias_to_existing_native_profile', True),
                             ('native_admission', True), ('source_consumer_implemented', True),
                             ('target_schema_family', 'accepted_runtime')]:
            row = self.row(); row[field] = value
            with self.assertRaises(ValueError): self.validate(row)
        row = self.row(); row['source_parameters']['fabricated'] = False
        with self.assertRaisesRegex(ValueError, 'source identity changed'): self.validate(row)

    def test_provenance_metadata_shape_is_closed_and_empty_arrays_do_not_admit_values(self):
        from jsonschema import Draft202012Validator, ValidationError
        validator = Draft202012Validator(assembler.shape({'proofs': [], 'count': 1}))
        validator.validate({'proofs': [], 'count': 1})
        for value in [{'proofs': [1], 'count': 1}, {'proofs': [], 'count': 1, 'opaque_lua': 'x'}]:
            with self.assertRaises(ValidationError): validator.validate(value)

    def test_import_restores_module_search_path_in_a_fresh_process(self):
        script = "import sys; sys.path.insert(0, 'tools/content-migration'); before=list(sys.path); import assemble_source_complete_monster_packets; assert sys.path == before"
        subprocess.run([sys.executable, '-c', script], cwd=assembler.ROOT, check=True, capture_output=True)


if __name__ == '__main__':
    unittest.main()
