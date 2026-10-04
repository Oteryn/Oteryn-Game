import copy
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from jsonschema import ValidationError
import compose_source_complete_schema as composition

HERE = Path(__file__).resolve().parent


class PrivateSchemaCompositionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.path = Path(self.tmp.name) / 'extension.json'
        self.schema = {'$schema': 'https://json-schema.org/draft/2020-12/schema',
                       '$id': 'urn:oteryn:test:closed-source-party:1', '$defs': {'nativeBehavior': {
                       'type': 'object', 'additionalProperties': False,
                       'properties': {'key': {'const': 'party_buff'}, 'parameters': {
                           'type': 'object', 'additionalProperties': False,
                           'properties': {'source_model': {'const': 'r61-party_buff'},
                                          'base_mana': {'type': 'integer', 'minimum': 0}},
                           'required': ['source_model', 'base_mana']}},
                       'required': ['key', 'parameters']}}}
        self.path.write_text(json.dumps(self.schema))
        monster = json.loads((HERE.parent / 'monster-authoring/monster.schema.json').read_bytes())
        base = json.loads((HERE / 'spell.schema.json').read_bytes())
        self.base_bytes = (HERE / 'spell.schema.json').read_bytes()
        self.validator, self.private, _ = composition.validator([self.path], [monster, base])
        profiles = json.loads((HERE / 'samples/executable-spell-catalog.json').read_bytes())['bundles']
        self.original = copy.deepcopy(next(p['bundle'] for p in profiles
                            if p['bundle']['spell']['execution'].get('native_behavior', {}).get('key') == 'party_buff'))

    def tearDown(self):
        self.tmp.cleanup()

    def test_original_party_payment_constraint_survives(self):
        self.validator.validate(self.original)
        changed = copy.deepcopy(self.original)
        changed['spell']['costs']['mana'] = 100
        with self.assertRaises(ValidationError):
            self.validator.validate(changed)
        self.assertEqual(self.base_bytes, (HERE / 'spell.schema.json').read_bytes())
        self.assertNotEqual(json.loads(self.base_bytes)['$id'], self.private['$id'])

    def source_party(self):
        changed = copy.deepcopy(self.original)
        changed['spell']['costs']['mana'] = 100
        changed['spell']['execution']['native_behavior']['parameters'] = {
            'source_model': 'r61-party_buff', 'base_mana': 100}
        return changed

    def test_only_closed_private_party_model_can_retain_source_registrar_mana(self):
        self.validator.validate(self.source_party())
        for field, value in [('source_model', 'unknown-schema'), ('invented_operation', 'execute-lua')]:
            changed = self.source_party()
            changed['spell']['execution']['native_behavior']['parameters'][field] = value
            with self.assertRaises(ValidationError):
                self.validator.validate(changed)

    def test_duplicate_resource_identity_and_missing_model_refused(self):
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            composition.compose([self.path, self.path])
        self.path.write_text(json.dumps({'$schema': self.schema['$schema'], '$id': self.schema['$id']}))
        with self.assertRaisesRegex(ValueError, 'nativeBehavior'):
            composition.compose([self.path])
