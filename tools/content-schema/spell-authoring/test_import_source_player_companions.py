"""Bounded current-donor companion qualification and immutable package closure."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import import_source_player_companions as importer


class SourceCompanionTests(unittest.TestCase):
    package = importer.ROOT / 'docs/reference/spells/r29-source-closure/player-companion-source-candidates'

    def raw(self, name='haste'):
        spec = importer.native_companions.SPECS[name]['sources'][importer.SOURCE]
        return {'name': name.title(), 'source': importer.SOURCE, 'spell_type': 'instant', 'file': spec['path'], 'blob': spec['blob']}

    def test_changed_cast_and_donor_refused(self):
        with patch.object(importer.base, 'source_file', side_effect=[b'changed', b'template']):
            with self.assertRaisesRegex(importer.base.SourceBlocked, 'full source cast differs'):
                importer.qualify('haste', self.raw(), None)
        with self.assertRaisesRegex(importer.base.SourceBlocked, 'source identity'):
            importer.qualify('haste', {**self.raw(), 'source': 'canary'}, None)

    def test_relevant_game_function_drift_refused(self):
        original = importer.base.source_file
        def changed(repo, revision, path):
            data = original(repo, revision, path)
            if path == 'src/game/game.cpp' and revision == importer.PIN:
                body = importer.base.source_cpp_function(data, 'Game::changeSpeed')
                data = data.replace(body, body.replace(b'{', b'{ /* deliberate changed helper */', 1), 1)
            return data
        with patch.object(importer.base, 'source_file', side_effect=changed):
            with self.assertRaisesRegex(importer.base.SourceBlocked, 'relevant helper changed'):
                importer.qualify('haste', self.raw(), Path('/workspace/spell-sources/crystal'))

    def test_existing_output_refused_and_failed_staging_not_published(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / 'existing'
            out.mkdir()
            marker = out / 'keep.txt'
            marker.write_text('keep')
            with self.assertRaisesRegex(ValueError, 'immutable'):
                importer.run(out, None, None)
            self.assertEqual(marker.read_text(), 'keep')
            fresh = Path(temp) / 'new'
            with patch.object(importer, 'generate', side_effect=ValueError('source drift')):
                with self.assertRaisesRegex(ValueError, 'source drift'):
                    importer.run(fresh, None, None)
            self.assertFalse(fresh.exists())

    def test_four_strict_candidates_keep_crystal_grade_familiar_behavior(self):
        summary = json.loads((self.package / 'import-summary.json').read_text())
        self.assertEqual(summary['records'], 4)
        self.assertEqual(summary['revision'], 'source-player-r29')
        schema = json.loads((self.package / 'receipt.schema.json').read_text())
        validator = importer.base.validate_spell.Draft202012Validator(schema, registry=importer.base.validate_spell.REGISTRY)
        names = set()
        for path in self.package.glob('*/*/spell.json'):
            bundle = json.loads(path.read_text())
            deps = json.loads((path.parent / 'dependencies.json').read_text())
            catalog = json.loads((path.parent / 'catalog.json').read_text())
            self.assertEqual(importer.base.validate_spell.validate(bundle, deps, catalog), [])
            spell = bundle['spell']
            self.assertEqual(spell['identity']['revision'], 'source-player-r29')
            names.add(spell['name'].casefold())
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            validator.validate(receipt)
            proof = json.loads(receipt['conversion_notes'][1])
            self.assertEqual(proof['selected_donor'], 'crystal')
            self.assertFalse(proof['canary_precedence_applied'])
            self.assertFalse(receipt['runtime_activation'])
            helpers = proof['helpers']
            self.assertTrue(any(h.get('symbol') == 'Game::changeSpeed' for h in helpers))
            if spell['name'].casefold() == 'swift foot':
                parameters = spell['execution']['native_behavior']['parameters']
                self.assertEqual(parameters['damage_dealt_percent'], {'none': 70, 'regular': 50, 'greater': 100})
                self.assertTrue(any(h.get('symbol') == 'PlayerWheel::getSpellUpgrade' for h in helpers))
        self.assertEqual(names, set(importer.NAMES))

    def test_swift_regular_to_greater_preserves_modifier_and_blocks_legacy_write(self):
        path = next(self.package.glob('*/*/source-grade-actions.json'))
        fact = json.loads(path.read_text())
        validator = importer.base.validate_spell.Draft202012Validator(importer.grade_actions_schema())
        validator.validate(fact)
        actions = {action['grade']: action for action in fact['actions']}
        existing = actions['regular']['percent']
        greater = actions['greater']
        self.assertEqual(greater['action'], 'no_op')
        self.assertTrue(greater['preserve_existing_modifier'])
        after_greater = existing if greater['action'] == 'no_op' else greater['percent']
        self.assertEqual(after_greater, 50)
        self.assertEqual(fact['descriptor_placeholder']['value'], 100)
        self.assertNotEqual(after_greater, fact['descriptor_placeholder']['value'])
        self.assertEqual(fact['descriptor_placeholder']['classification'], 'legacy_model_placeholder_not_source_write')
        self.assertFalse(fact['execution_equivalence'])
        self.assertEqual(fact['runtime_admission'], 'blocked')
        wrong = json.loads(json.dumps(fact))
        wrong['actions'][2] = {'grade': 'greater', 'action': 'apply_modifier', 'percent': 100, 'duration_ms': 10000}
        self.assertTrue(list(validator.iter_errors(wrong)))
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertTrue(any(g['source_field'] == 'swift_foot.greater_grade_no_op' for g in receipt['remaining_mechanics']))

    def test_manifest_file_closure_and_frozen_r28_binding(self):
        manifest = json.loads((self.package / 'package-manifest.json').read_text())['files']
        files = {p.relative_to(self.package).as_posix() for p in self.package.rglob('*') if p.is_file()} - {'package-manifest.json'}
        self.assertEqual(set(manifest), files)
        for name, digest in manifest.items():
            self.assertEqual(importer.base.sha((self.package / name).read_bytes()), digest)
        summary = json.loads((self.package / 'import-summary.json').read_text())
        r28 = importer.ROOT / 'docs/reference/spells/r28-source-closure/player-source-bundles/package-manifest.json'
        self.assertEqual(summary['input_proofs']['r28/package-manifest.json'], importer.base.sha(r28.read_bytes()))


if __name__ == '__main__':
    unittest.main()
