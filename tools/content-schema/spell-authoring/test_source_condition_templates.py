import json
import gzip
from pathlib import Path
import unittest
import subprocess
import sys
import tempfile
import source_condition_templates as exporter
import spell_scripts

SOURCE = Path('/workspace/spells-r22-monster-import-current/source-inputs/canary')


def param(name, value):
    return {'line': 1, 'method': 'setParameter', 'arguments': [{'kind': 'source_symbol', 'value': name}, {'kind': 'scalar', 'value': value}], 'declaration_prefix': True}


class SourceConditionTemplateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.enums, cls.enum_proof = exporter.pinned_engine_enums('canary', SOURCE.parent)

    def test_import_does_not_shadow_player_formal_verifier(self):
        script = "import source_condition_templates; import verify_formal_schema; print(verify_formal_schema.__file__)"
        output = subprocess.check_output([sys.executable, '-c', script], cwd=Path(__file__).parent, text=True)
        self.assertEqual(Path(output.strip()).resolve(), Path(__file__).parent.resolve() / 'verify_formal_schema.py')

    def test_engine_enums_reject_tampered_staged_header(self):
        self.assertEqual(self.enum_proof['revision'], exporter.PINS['canary'])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            header = root / 'canary' / spell_scripts.ENGINE_DEFINITIONS
            header.parent.mkdir(parents=True)
            header.write_text('// changed staging bytes')
            with self.assertRaisesRegex(ValueError, 'pinned Git blob'):
                exporter.pinned_engine_enums('canary', root)

    def test_artifact_references_preserve_original_argument_shapes_and_partial_outfits(self):
        root = Path(__file__).resolve().parents[3]
        inventory = json.loads(gzip.decompress((root / 'docs/reference/spells/r28-source-closure/source-mechanics-inventory.json.gz').read_bytes()))
        files = {(f['source'], f['path']): f for f in inventory['files']}
        artifact = root / 'docs/reference/spells/r31-source-closure/source-condition-templates.jsonl.gz'
        records = [json.loads(line) for line in gzip.decompress(artifact.read_bytes()).splitlines()]
        self.assertEqual(len(records), 59)
        outfit_prefixes = 0
        for record in records:
            self.assertFalse(record['template_complete'])
            item = files[record['source'], record['path']]
            calls = record['calls'] + [{'source_call_ref': record['constructor_call_ref'], 'arguments': record['constructor_arguments']}]
            for call in calls:
                ref = call['source_call_ref']
                original = item['calls'][ref['inventory_call_index']]
                self.assertEqual((original['line'], original['source_order']), (ref['line'], ref['source_order']))
                for argument, raw in zip(call['arguments'], original['arguments']):
                    if argument['kind'] == 'unresolved_source_argument':
                        self.assertEqual(argument['source_argument_sha256'], exporter.sha(json.dumps(raw, sort_keys=True).encode()))
            if record['typed_effect'] and record['source_type_constant'] == 'CONDITION_OUTFIT':
                outfit_prefixes += 1
                self.assertTrue(record['appearance_binding_unresolved'])
                self.assertTrue(any('appearance' in gap for gap in record['mapping_gaps']))
        self.assertEqual(outfit_prefixes, 2)

    def test_bounded_numeric_arithmetic_preserves_exact_duration(self):
        self.assertEqual(exporter.scalar({'kind': 'expression_tokens', 'tokens': ['2', '*', '60', '*', '1000']}), 120000)
        self.assertEqual(exporter.scalar({'kind': 'expression_tokens', 'tokens': ['-', '1']}), -1)
        self.assertIs(exporter.scalar({'kind': 'literal', 'value': False}), False)

    def test_dynamic_expression_is_never_executed_or_coerced(self):
        for tokens in (['player', ':', 'getLevel', '(', ')'], ['dangerous', '(', ')'], ['duration']):
            argument = {'kind': 'expression_tokens', 'tokens': tokens}
            with self.subTest(tokens=tokens), self.assertRaises((ValueError, SyntaxError)):
                exporter.scalar(argument)
            fact = exporter.argument_fact(argument)
            self.assertEqual(fact['kind'], 'unresolved_source_argument')
            self.assertEqual(len(fact['source_argument_sha256']), 64)

    def test_actual_party_attribute_parameters_map_to_existing_effect(self):
        rows = [param('CONDITION_PARAM_SUBID', 3), param('CONDITION_PARAM_BUFF_SPELL', 1),
                param('CONDITION_PARAM_TICKS', 120000), param('CONDITION_PARAM_STAT_MAGICPOINTS', 1)]
        effect, formulas, gaps = exporter.map_template('CONDITION_ATTRIBUTES', rows, self.enums, 'candidate:test/party')
        self.assertEqual(effect['duration_ms'], 120000)
        self.assertEqual(effect['condition']['attribute_modifiers'], [{'attribute': 'stat_magicpoints', 'mode': 'add', 'value': 1}])
        self.assertTrue(effect['condition']['buff_spell'])
        self.assertEqual(formulas, [])
        self.assertEqual(gaps, [])

    def test_permanent_source_ticks_are_not_fabricated_as_finite_duration(self):
        effect, formulas, gaps = exporter.map_template('CONDITION_ATTRIBUTES',
                [param('CONDITION_PARAM_TICKS', -1), param('CONDITION_PARAM_SKILL_FISTPERCENT', 115)], self.enums, 'candidate:test/forever')
        self.assertIsNone(effect)
        self.assertTrue(gaps)

    def test_outfit_shape_is_not_guessed(self):
        row = {'line': 2, 'method': 'setOutfit', 'arguments': [{'kind': 'unresolved_source_argument', 'source_argument_sha256': 'a' * 64}], 'declaration_prefix': True}
        effect, _, gaps = exporter.map_template('CONDITION_OUTFIT', [row], self.enums, 'candidate:test/outfit')
        self.assertIsNone(effect)
        self.assertIn('setOutfit', gaps[0])

    def test_regeneration_and_haste_reuse_existing_typed_mapper(self):
        effect, _, gaps = exporter.map_template('CONDITION_REGENERATION',
                [param('CONDITION_PARAM_TICKS', 120000), param('CONDITION_PARAM_HEALTHGAIN', 20), param('CONDITION_PARAM_HEALTHTICKS', 2000)], self.enums, 'candidate:test/heal')
        self.assertEqual(effect['condition']['regeneration'], {'health_gain': 20, 'health_interval_ms': 2000})
        rows = [param('CONDITION_PARAM_TICKS', 20000), {'line': 2, 'method': 'setFormula',
                'arguments': [{'kind': 'scalar', 'value': v} for v in (0.7, 56, 0.7, 56)], 'declaration_prefix': True}]
        effect, formulas, gaps = exporter.map_template('CONDITION_HASTE', rows, self.enums, 'candidate:test/speed')
        self.assertEqual(formulas[0]['speed']['minimum_multiplier'], {'numerator': 7, 'denominator': 10})
        self.assertEqual(effect['condition']['speed_formula']['key'], 'candidate:test/speed/speed')


if __name__ == '__main__':
    unittest.main()
