"""Current Canary speed operator qualification preserves changed staff inputs."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import import_source_canary_companions as importer


class CanaryCompanionTests(unittest.TestCase):
    package = importer.ROOT / 'docs/reference/spells/r30-source-closure/player-canary-companion-source-candidates'

    def test_audit_slice_mismatch_and_changed_pin_refused(self):
        audit = json.loads(importer.AUDIT.read_text())
        wrong = copy.deepcopy(audit)
        wrong['records'][0]['current'][0]['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'source slice SHA'):
            importer.qualify_helpers(Path('/workspace/spell-sources/canary'), wrong)
        wrong = copy.deepcopy(audit)
        wrong['current_revision'] = importer.OLD_PIN
        with self.assertRaisesRegex(ValueError, 'pin mismatch'):
            importer.qualify_helpers(None, wrong)

    def test_called_control_block_cannot_be_a_function_definition(self):
        audit = json.loads(importer.AUDIT.read_text())
        wrong = copy.deepcopy(audit)
        wrong['records'][0]['old'][0]['signature'] = 'if (!ConditionSpeed::startCondition(creature))'
        with self.assertRaisesRegex(ValueError, 'control block'):
            importer.qualify_helpers(Path('/workspace/spell-sources/canary'), wrong)

    def test_staff_input_provider_difference_is_strictly_preserved(self):
        fact = json.loads((self.package / 'source-speed-inputs.json').read_text())
        validator = importer.base.validate_spell.Draft202012Validator(importer.inputs_schema())
        validator.validate(fact)
        self.assertEqual((fact['old_staff_cap'], fact['current_staff_cap']), (1500, 65535))
        self.assertEqual(fact['input_basis'], 'current_engine_getBaseSpeed')
        self.assertFalse(fact['input_provider_equivalence'])
        self.assertEqual(fact['runtime_admission'], 'blocked')
        for mutation in ({'current_staff_cap': 1500}, {'input_provider_equivalence': True}):
            self.assertTrue(list(validator.iter_errors({**fact, **mutation})))

    def test_three_source_candidates_and_explicit_provider_gap_validate(self):
        schema = json.loads((self.package / 'receipt.schema.json').read_text())
        validator = importer.base.validate_spell.Draft202012Validator(schema, registry=importer.base.validate_spell.REGISTRY)
        names = set()
        for path in self.package.glob('*/*/spell.json'):
            spell = json.loads(path.read_text())
            deps = json.loads((path.parent / 'dependencies.json').read_text())
            catalog = json.loads((path.parent / 'catalog.json').read_text())
            self.assertEqual(importer.base.validate_spell.validate(spell, deps, catalog), [])
            self.assertEqual(spell['spell']['identity']['revision'], 'source-player-r30')
            names.add(spell['spell']['name'].casefold())
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            validator.validate(receipt)
            self.assertFalse(receipt['runtime_activation'])
            self.assertTrue(any(g['source_field'] == 'base_speed.current_input_provider' for g in receipt['remaining_mechanics']))
        self.assertEqual(names, set(importer.NAMES))

    def test_manifest_closure_and_helper_changes_remain_distinct(self):
        manifest = json.loads((self.package / 'package-manifest.json').read_text())['files']
        files = {p.relative_to(self.package).as_posix() for p in self.package.rglob('*') if p.is_file()} - {'package-manifest.json'}
        self.assertEqual(files, set(manifest))
        for filename, digest in manifest.items():
            self.assertEqual(importer.base.sha((self.package / filename).read_bytes()), digest)
        proof = json.loads((self.package / 'helper-qualification-proof.json').read_text())
        self.assertEqual(len(proof['source_functions_and_storage']), 69)
        changed = {r['symbol'] for r in proof['source_functions_and_storage'] if r['status'] != 'byte_identical'}
        self.assertEqual(changed, importer.CHANGED)
        self.assertFalse(proof['global_input_provider_equivalent'])

    def test_immutable_output_and_failed_stage_preserve_prior_packages(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / 'package'
            out.mkdir();(out / 'keep').write_text('keep')
            with self.assertRaisesRegex(ValueError, 'immutable'):
                importer.run(out, None, None, None)
            self.assertEqual((out / 'keep').read_text(), 'keep')
            fresh = Path(temp) / 'fresh'
            with patch.object(importer, 'generate', side_effect=ValueError('proof failure')):
                with self.assertRaisesRegex(ValueError, 'proof failure'):
                    importer.run(fresh, None, None, None)
            self.assertFalse(fresh.exists())


if __name__ == '__main__':
    unittest.main()
