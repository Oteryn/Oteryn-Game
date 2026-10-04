import gzip
import json
from pathlib import Path
import tempfile
import unittest

from jsonschema import Draft202012Validator, ValidationError
import source_conjure_helper as module


class ConjureHelperTests(unittest.TestCase):
    def test_exact_helper_and_source_drift(self):
        text = 'function Player:conjureItem(reagentId, conjureId, conjureCount, effect)\n' + module.EXPECTED_BODY + '\nend\n'
        module.qualify_helper(text.encode())
        with self.assertRaises(ValueError):
            module.qualify_helper(text.replace('CONST_ME_MAGIC_RED', 'CONST_ME_MAGIC_GREEN').encode())

    def test_rune_overrides_explicit_effect(self):
        self.assertEqual(module.success_effect(True, 'CONST_ME_MAGIC_BLUE'), 'CONST_ME_MAGIC_RED')

    def test_non_rune_preserves_nil_false_and_zero(self):
        self.assertIsNone(module.success_effect(False, None))
        self.assertIs(module.success_effect(False, False), False)
        self.assertEqual(module.success_effect(False, 0), 0)

    def test_actual_population_and_closed_qualification(self):
        root = Path(__file__).resolve().parents[3]
        base = root / 'docs/reference/spells/r28-source-closure/player-source-bundles'
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'packet'
            receipt = module.generate(out, base, Path('/workspace/spell-sources'))
            records = json.loads(gzip.decompress((out / 'source-conjure-helper.json.gz').read_bytes()))
        self.assertEqual(receipt['records'], 100)
        self.assertEqual(receipt['status_counts'], {'CANDIDATE_SCHEMA_VALID': 94, 'BLOCKED': 6})
        self.assertEqual(receipt['fourth_argument_counts'], {'omitted_nil': 74, 'source_symbol': 26})
        self.assertEqual(len({r['registration_key'] for r in records}), 100)
        validator = Draft202012Validator(module.schema())
        record = records[0]
        with self.assertRaises(ValidationError):
            validator.validate(dict(record, runtime_activation=True))
        with self.assertRaises(ValidationError):
            validator.validate(dict(record, result_item_type_qualified=True))
        with self.assertRaises(ValidationError):
            validator.validate(dict(record, revision=module.PINS['crystal' if record['source'] == 'canary' else 'canary']))
        with self.assertRaises(ValidationError):
            validator.validate(dict(record, invented_default='CONST_ME_MAGIC_RED'))


if __name__ == '__main__':
    unittest.main()
