import copy
import gzip
import json
import pathlib
import unittest
from unittest.mock import patch

import jsonschema
import source_call_corrections as module

REPO = pathlib.Path(__file__).resolve().parents[3]


class CorrectionOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(gzip.decompress((REPO / module.OUTPUT_PATH).read_bytes()))
        cls.schema = json.loads((REPO / module.SCHEMA_PATH).read_text())
        cls.validator = jsonschema.Draft202012Validator(cls.schema)

    def test_strict_schema_and_source_only_flags(self):
        self.validator.validate(self.data)
        self.assertTrue(all(self.data[key] is False for key in module.FLAGS))
        modified = copy.deepcopy(self.data); modified['runtime_activation'] = True
        with self.assertRaises(jsonschema.ValidationError):
            self.validator.validate(modified)
        modified = copy.deepcopy(self.data); modified['corrections'][0]['runtime_identity'] = 1
        with self.assertRaises(jsonschema.ValidationError):
            self.validator.validate(modified)
        modified = copy.deepcopy(self.data); modified['corrections'][0]['operator'] = 'foo'
        with self.assertRaises(jsonschema.ValidationError):
            self.validator.validate(modified)

    def test_all_files_conserved_and_call_counts_reconciled(self):
        rows = self.data['per_file_reconciliation']
        self.assertEqual(2346, len({module.key(row['source_identity']) for row in rows}))
        self.assertEqual(32, sum(row['excluded_operator_count'] for row in rows))
        for row in rows:
            self.assertEqual(row['frozen_call_count'] - row['excluded_operator_count'], row['corrected_call_count'])
            self.assertEqual(row['ast_call_invoke_count'], row['corrected_call_count'])
        self.assertEqual(50286, sum(row['corrected_call_count'] for row in rows))
        self.assertEqual({'and': 13, 'or': 11, 'not': 8}, self.data['counts']['operators'])
        self.assertEqual(11966, self.data['counts']['corrected_unsupported_call_references'])

    def test_all_32_references_match_immutable_call_argument_facts(self):
        inventory = json.loads(gzip.decompress((REPO / module.INVENTORY_PATH).read_bytes()))
        lookup = {module.key(module.identity(row)): row for row in inventory['files']}
        for correction in self.data['corrections']:
            row = lookup[module.key(correction['source_identity'])]
            call = row['calls'][correction['frozen_call_index']]
            self.assertEqual(correction['frozen_source_order'], call['source_order'])
            self.assertEqual(correction['line'], call['line'])
            self.assertEqual(correction['operator'], call['call_identity'])
            self.assertEqual('unsupported_call_reference', call['category'])
            self.assertEqual(correction['argument_fact_sha256'], module.digest(module.canonical(call['arguments'])))
            self.assertEqual(correction['frozen_call_fact_sha256'], module.digest(module.canonical(call)))
        self.assertEqual(32, len(self.data['corrections']))

    def test_frozen_input_sha_mismatch_is_refused(self):
        with patch.object(module, 'EXPECTED_INVENTORY', '0' * 64):
            with self.assertRaisesRegex(ValueError, 'frozen gzip SHA mismatch'):
                module.build(REPO, '/unused')


if __name__ == '__main__':
    unittest.main()
