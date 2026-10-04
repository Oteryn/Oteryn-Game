import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
import source_profile_coverage as coverage


class SourceProfileCoverageTests(unittest.TestCase):
    def test_engine_symbol_is_distinct_from_literal_string(self):
        self.assertEqual(coverage.fact('@COMBAT_EARTHDAMAGE'), {'kind': 'symbol', 'value': 'COMBAT_EARTHDAMAGE'})
        self.assertEqual(coverage.fact('COMBAT_EARTHDAMAGE'), {'kind': 'string', 'value': 'COMBAT_EARTHDAMAGE'})
        self.assertEqual(coverage.fact(False), {'kind': 'boolean', 'value': False})

    def test_nonfinite_and_unknown_values_refused(self):
        for value in (float('nan'), float('inf'), object()):
            with self.subTest(value=type(value)), self.assertRaises(ValueError):
                coverage.fact(value)

    def test_destination_checks_and_exact_source_field_boundary(self):
        rows = [{'source_index': 0, 'source_field': 'sound.extra', 'status': 'mapped', 'destination': '/monster/sound'},
                {'source_index': 0, 'source_field': 'sounds', 'status': 'mapped', 'destination': '/monster/missing'},
                {'source_index': 1, 'source_field': 'sound', 'status': 'mapped', 'destination': '/monster/missing'}]
        result = coverage.field_coverage('sound', rows, {'monster': {'sound': 1}})
        self.assertEqual(result['destinations'], [{'pointer': '/monster/sound', 'present': True}])
        self.assertTrue(result['typed_mapper_claim_present'])
        missing = coverage.field_coverage('sounds', rows, {'monster': {}})
        self.assertEqual(missing['destinations'], [{'pointer': '/monster/missing', 'present': False}])

    def test_strict_recursive_schema(self):
        schema = json.loads((Path(__file__).parent / 'source-profile-facts.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        schema = {'$ref': '#/$defs/fact', '$defs': schema['$defs']}
        validator = Draft202012Validator(schema)
        validator.validate(coverage.fact({'elements': [{'type': 'COMBAT_EARTHDAMAGE', 'percent': 10}]}))
        self.assertTrue(list(validator.iter_errors({'kind': 'number', 'value': 7, 'extra': True})))
        self.assertTrue(list(validator.iter_errors({'kind': 'symbol', 'value': 7})))

    def test_json_pointer_escaping(self):
        self.assertEqual(coverage.lookup({'a/b': {'x~y': [7]}}, '/a~1b/x~0y/0'), 7)


if __name__ == '__main__':
    unittest.main()
