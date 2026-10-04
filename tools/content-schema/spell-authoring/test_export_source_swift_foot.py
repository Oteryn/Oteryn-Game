import copy
import json
from pathlib import Path
import tempfile
import unittest
from jsonschema.exceptions import ValidationError
import export_source_swift_foot as exporter

class SourceSwiftFootTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source_root = Path('/workspace/spell-sources')
        cls.r28 = exporter.ROOT / 'docs/reference/spells/r28-source-closure/player-source-bundles'
        cls.data = exporter.base.source_file(cls.source_root / 'canary', exporter.PIN, exporter.SOURCE)
        cls.tmp = tempfile.TemporaryDirectory();cls.out = Path(cls.tmp.name) / 'packet'
        exporter.run(cls.out, cls.source_root, cls.r28)
        cls.fact = json.loads((cls.out / 'swift-foot-source.json').read_text())
    @classmethod
    def tearDownClass(cls): cls.tmp.cleanup()
    def test_complete_source_program_and_condition_order(self):
        self.assertTrue(exporter.validate(self.fact, self.data))
        callback = self.fact['program']['callback']['body']
        self.assertEqual(callback[0]['name'], 'summons')
        self.assertEqual(callback[2]['condition']['method'], 'execute')
        self.assertEqual(callback[2]['then'][0]['value']['method'], 'upgradeSpellsWOD')
        self.assertEqual([x['type'] for x in self.fact['grade_actions'][0]['conditions']], ['CONDITION_EXHAUST_COMBAT', 'CONDITION_PACIFIED', 'CONDITION_SPELLGROUPCOOLDOWN'])
        self.assertEqual(self.fact['grade_actions'][0]['conditions'][2]['parameters'], [{'parameter': 'CONDITION_PARAM_SUBID', 'value': 1}, {'parameter': 'CONDITION_PARAM_TICKS', 'value': 10000}])
        self.assertEqual(callback[2]['then'][-1]['value'], exporter.literal(True))
        self.assertEqual(callback[-1]['value'], exporter.literal(False))
    def test_greater_noop_cannot_be_legacy_100_write(self):
        altered = copy.deepcopy(self.fact);altered['grade_actions'][2]['action'] = 'apply_conditions'
        altered['grade_actions'][2]['conditions'] = [{'type': 'CONDITION_ATTRIBUTES', 'parameters': [{'parameter': 'CONDITION_PARAM_BUFF_DAMAGEDEALT', 'value': 100}]}]
        with self.assertRaises(ValidationError): exporter.base.validate_spell.Draft202012Validator(exporter.schema()).validate(altered)
        self.assertEqual(self.fact['grade_actions'][2]['action'], 'no_op')
        self.assertEqual(self.fact['grade_actions'][2]['existing_modifier_operation'], 'no_explicit_removal')
    def test_source_adapter_rejects_coefficient_order_and_gate_changes(self):
        for path in ('order', 'formula', 'gate'):
            altered = copy.deepcopy(self.fact)
            if path == 'order': altered['program']['callback']['body'][2]['then'].reverse()
            elif path == 'formula': altered['program']['initialization'][6]['expression']['arguments'][0]['value'] = 0.8
            else: altered['program']['callback']['body'][2]['condition']['method'] = 'familiar'
            with self.assertRaises(ValueError): exporter.validate(altered, self.data)
    def test_every_source_span_matches_exact_pinned_bytes(self):
        def walk(value):
            if isinstance(value, dict):
                if 'source_span' in value:
                    span = value['source_span'];lines = self.data.splitlines(keepends=True)
                    self.assertEqual(span['sha256'], exporter.base.sha(b''.join(lines[span['start_line'] - 1:span['end_line']])))
                for item in value.values(): walk(item)
            elif isinstance(value, list):
                for item in value: walk(item)
        walk(self.fact['program'])
        reference = json.loads((self.out / 'source-reference.json').read_text())
        self.assertFalse((self.out / 'source.lua').exists())
        self.assertEqual(reference['sha256'], exporter.base.sha(self.data))
        self.assertEqual(reference['bytes'], len(self.data))
        self.assertFalse(reference['raw_source_distributed'])
        with self.assertRaises(ValueError): exporter.source_program(self.data.replace(b'50)', b'70)'))
    def test_strict_unknown_fields_activation_and_source_pin_rejected(self):
        for field, value in [('runtime_activation', True), ('source_revision', '0' * 40), ('future_guess', {}), ('registration_key', 'canary-main-current/other.lua#1')]:
            altered = copy.deepcopy(self.fact);altered[field] = value
            with self.assertRaises(ValidationError): exporter.base.validate_spell.Draft202012Validator(exporter.schema()).validate(altered)
    def test_failed_input_validation_leaves_no_published_packet(self):
        out = Path(self.tmp.name) / 'failed-packet'
        with self.assertRaises(FileNotFoundError): exporter.run(out, self.source_root, Path(self.tmp.name) / 'missing-r28')
        self.assertFalse(out.exists())
        self.assertEqual(sorted(p.name for p in Path(self.tmp.name).iterdir()), ['packet'])
    def test_manifest_exact_closure_no_fake_candidate_and_immutable_output(self):
        manifest = json.loads((self.out / 'package-manifest.json').read_text())['files']
        self.assertEqual(set(manifest), {p.name for p in self.out.iterdir() if p.name != 'package-manifest.json'})
        for filename, digest in manifest.items(): self.assertEqual(digest, exporter.base.sha((self.out / filename).read_bytes()))
        self.assertFalse((self.out / 'spell.json').exists());self.assertFalse(self.fact['candidate_created'])
        with self.assertRaises(ValueError): exporter.run(self.out, self.source_root, self.r28)
        self.assertEqual(manifest, json.loads((self.out / 'package-manifest.json').read_text())['files'])
if __name__ == '__main__': unittest.main()
