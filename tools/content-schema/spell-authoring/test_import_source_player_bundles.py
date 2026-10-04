"""Source arithmetic, player-guard and atomic package replacement boundaries."""
import copy
import gzip
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch

import import_source_player_bundles as importer


class SourceBundleTests(unittest.TestCase):
    def test_source_level_div_five_preserved_without_world_curve_rewrite(self):
        expr = {'op': 'div', 'args': [{'var': 'level'}, {'const': '5'}]}
        callback = {'kind': 'CALLBACK_PARAM_LEVELMAGICVALUE', 'formula': {
            'status': 'resolved', 'minimum': copy.deepcopy(expr),
            'maximum': {'op': 'add', 'args': [copy.deepcopy(expr), {'const': '5'}]}}}
        result = importer.source_formula(callback)
        self.assertEqual(result['minimum']['args'][0]['args'][0], expr)
        self.assertNotIn('level_base_damage_healing', json.dumps(result))

    def test_unqualified_library_function_blocked_instead_of_substitution(self):
        with self.assertRaisesRegex(importer.SourceBlocked, 'typed equivalent'):
            importer.expr_source({'fn': 'flat_damage_healing', 'args': [{'var': 'level'}]})

    def test_monster_plain_tier_cannot_bypass_player_custom_guard(self):
        with self.assertRaisesRegex(importer.SourceBlocked, 'custom guard/branch'):
            importer.convert_combat(None, {'tier': 'P1'}, {'cast': {'tier': 'custom', 'patterns': ['familiar']}}, {}, 'candidate:test')

    def test_native_cast_drift_blocks_template_reuse(self):
        raw = {'name': 'Food', 'file': 'source.lua', 'spell_type': 'instant'}
        with tempfile.TemporaryDirectory() as temp:
            with patch.object(importer, 'source_file', side_effect=[b'current', b'qualified']):
                with self.assertRaisesRegex(importer.SourceBlocked, 'cast bytes differ'):
                    importer.source_native_descriptor(raw, None, 'current', 'canary-main-current', temp)

    def test_native_helper_drift_blocks_template_reuse(self):
        raw = {'name': 'Magic Rope', 'file': 'source.lua', 'spell_type': 'instant'}
        with tempfile.TemporaryDirectory() as temp:
            with patch.object(importer, 'source_file', side_effect=[b'cast', b'cast', b'changed helper']):
                with self.assertRaisesRegex(importer.SourceBlocked, 'helper closure changed'):
                    importer.source_native_descriptor(raw, None, 'current', 'crystal-summer-current', temp)

    def test_native_external_overrides_and_source_mismatch_excluded(self):
        for name in ['Wild Growth Rune', 'Find Person', 'Find Fiend', 'Disintegrate Rune']:
            with self.subTest(name=name), patch.object(importer, 'source_file') as source:
                self.assertIsNone(importer.source_native_descriptor({'name': name}, None, '', 'canary-main-current', None))
                source.assert_not_called()

    def test_recursive_items_preserve_donor_identity_and_remove_duplicates(self):
        a = {'family': 'Item', 'key': 'candidate:item/source/canary-main-current/105', 'revision': importer.REVISION}
        b = {**a, 'key': 'candidate:item/source/crystal-summer-current/105'}
        self.assertEqual(importer.item_references({'nested': [a, {'deep': a}, b]}), [a, b])

    def test_source_native_item_schema_couples_revision_and_allowed_snapshot(self):
        schema = json.loads((importer.HERE / 'spell.schema.json').read_text())
        paths = [schema['$defs']['nativeBehavior']['allOf'][15]['then']['properties']['parameters']['properties']['pool']['items'],
                 schema['$defs']['nativeBehavior']['allOf'][17]['then']['properties']['parameters']['oneOf'][2]['properties']['field_items']['items']]
        valid = [
            {'family': 'Item', 'key': 'candidate:item/105', 'revision': 'spell-p2-r21'},
            {'family': 'Item', 'key': 'candidate:item/source/canary-main-current/105', 'revision': importer.REVISION},
            {'family': 'Item', 'key': 'candidate:item/source/crystal-summer-current/2131', 'revision': importer.REVISION}]
        invalid = [
            {**valid[0], 'revision': importer.REVISION},
            {**valid[1], 'revision': 'spell-p2-r21'},
            {**valid[1], 'key': 'candidate:item/source/canary-main/105'},
            {**valid[1], 'key': 'candidate:item/source/crystal-summer-current/0'}]
        for field in paths:
            validator = importer.validate_spell.Draft202012Validator(field)
            for ref in valid:
                validator.validate(ref)
            for ref in invalid:
                self.assertTrue(list(validator.iter_errors(ref)), ref)

    def test_generated_native_items_preserve_canary_crystal_field_difference(self):
        root = importer.HERE.parents[2] / 'docs/reference/spells/r28-source-closure/player-source-bundles'
        ids = {}
        for path in root.glob('*/*/spell.json'):
            spell = json.loads(path.read_text())['spell']
            native = spell['execution'].get('native_behavior', {})
            if native.get('key') == 'tile_item_operation' and native['parameters']['operation'] == 'remove_field':
                ids[path.parts[-3]] = {int(r['key'].rsplit('/', 1)[1]) for r in native['parameters']['field_items']}
        self.assertIn(21465, ids['canary-main-current'])
        self.assertNotIn(2131, ids['canary-main-current'])
        self.assertIn(2131, ids['crystal-summer-current'])
        self.assertNotIn(21465, ids['crystal-summer-current'])

    def test_cpp_function_slice_ignores_comments_strings_and_nested_braces(self):
        data = b'void X::f(int x) { const char *s = "} {"; /* } */ if (x) { x++; } }\nvoid X::g() {}'
        body = importer.source_cpp_function(data, 'X::f')
        self.assertEqual(body, data.split(b'\n')[0])
        with self.assertRaisesRegex(importer.SourceBlocked, 'not unique'):
            importer.source_cpp_function(data + b'\nvoid X::f(int x) {}', 'X::f')

    def test_source_signed_explicit_formula_precedes_positive_callback(self):
        class Converter:
            source_snapshot = 'canary-main-current'
            source_revision = 'source'
            source_player_rules = {'explicit_damage_precedes_callback': True, 'signed_health_delta': True, 'proofs': []}
            def combat_ability(self, key, combat, geometry, distance, deps, assets, notes):
                self.combat = combat
                deps['effects'].append({'formula': {'key': importer.canary_batch.CASTER_MAGNITUDE}, 'operation': 'heal', 'damage_type': 'healing'})
        converter = Converter()
        info = {'tier': 'P1', 'variants': [0], 'combats': {'0': {'formula': ['@COMBAT_FORMULA_DAMAGE', -5, 0, -9, 0], 'params': {}, 'callbacks': {}}}}
        raw = {'cast': {'tier': 'plain_combat'}, 'combats': [{'set_formula': [['COMBAT_FORMULA_DAMAGE', -5, 0, -9, 0]], 'callbacks': [
            {'kind': 'CALLBACK_PARAM_LEVELMAGICVALUE', 'formula': {'status': 'resolved', 'minimum': {'const': '5'}, 'maximum': {'const': '9'}}}]}]}
        with patch.object(importer.source_formula_evidence, 'scalar_free_combat_adapter', side_effect=lambda combat, *args: (combat, [])):
            _, deps, notes, _ = importer.convert_combat(converter, info, raw, {'targeting': {'needs_target': False, 'target_or_direction': False, 'needs_direction': False}}, 'candidate:test')
        self.assertIsNone(converter.combat['formula'])
        self.assertEqual(deps['effects'][0]['operation'], 'damage')
        self.assertEqual(deps['effects'][0]['damage_type'], 'healing')
        formula = deps['formulas'][0]
        self.assertEqual(formula['minimum']['args'][0]['args'][0], {'const': '-5'})
        self.assertEqual(formula['maximum']['args'][1]['args'][0], {'const': '-9'})
        self.assertTrue(any('override valueCallback' in note for note in notes))

    def test_aleta_helper_scope_accepts_only_unrelated_drift(self):
        functions = {
            'src/lua/functions/creatures/player/player_functions.cpp': ['PlayerFunctions::luaPlayerCreate', 'PlayerFunctions::luaPlayerSetEditHouse', 'PlayerFunctions::luaPlayerSendHouseWindow'],
            'src/creatures/players/player.cpp': ['Player::sendHouseWindow', 'Player::getEditHouse', 'Player::setEditHouse'],
            'src/game/game.cpp': ['Game::playerUpdateHouseWindow', 'Game::getPlayerByName']}
        def source(repo, revision, path):
            text = '\n'.join('void ' + name + '() { return; }' for name in functions.get(path, [])) or 'unchanged'
            if revision == 'current':
                text += '\nvoid Unrelated::different() { if (1) {} }'
                if path not in functions:
                    # Complete stable helpers must compare full-file bytes.
                    text = 'unchanged'
            return text.encode()
        with patch.object(importer, 'source_file', side_effect=source):
            execution, _, notes, gaps = importer.source_house_descriptor('house guest list', {}, None, 'current', 'canary', b'cast')
        self.assertEqual(execution['native_behavior']['key'], 'house_access')
        proof = json.loads(notes[1])
        game = [p for p in proof['source_helper_proofs'] if p['path'] == 'src/game/game.cpp']
        self.assertTrue(all(p['scope'] == 'function' for p in game))
        self.assertTrue(all(p['current_file_sha256'] != p['template_file_sha256'] for p in game))
        self.assertEqual(gaps[0]['source_field'], 'house.transitive_common_engine_services')
        def changed(repo, revision, path):
            value = source(repo, revision, path)
            if revision == 'current' and path == 'src/game/game.cpp':
                return value.replace(b'playerUpdateHouseWindow() { return;', b'playerUpdateHouseWindow() { changed();')
            return value
        with patch.object(importer, 'source_file', side_effect=changed):
            with self.assertRaisesRegex(importer.SourceBlocked, 'relevant source function changed'):
                importer.source_house_descriptor('house guest list', {}, None, 'current', 'canary', b'cast')

    def test_source_clicked_position_and_creature_target_flags_remain_independent(self):
        full_schema = json.loads((importer.HERE / 'spell.schema.json').read_text())
        schema = {**full_schema['$defs']['spell']['properties']['targeting'], '$defs': full_schema['$defs']}
        validator = importer.validate_spell.Draft202012Validator(schema, registry=importer.validate_spell.REGISTRY)
        flags = {'aggressive': True, 'self_target': False, 'needs_target': True, 'needs_direction': False,
                 'target_or_direction': False, 'block_walls': True, 'parameter': 'none', 'cast_at_position': True}
        validator.validate(flags)
        self.assertTrue(list(validator.iter_errors({**flags, 'cast_at_position': 'true'})))

    def test_scalar_free_selector_filters_numeric_alias_without_direct_damage(self):
        class Converter:
            source_snapshot = 'canary-main-current'
            source_revision = 'source'
            source_player_rules = {'caster_or_top_creature': True, 'proofs': []}
            def engine_params(self, calls, notes):
                return {'COMBAT_PARAM_TYPE': 'COMBAT_PHYSICALDAMAGE'} if calls else {}
            def combat_ability(self, key, combat, geometry, distance, deps, assets, notes):
                self.combat = combat
                deps['effects'].append({'operation': 'apply_condition'})
                deps['abilities'].append({'effects': []})
        converter = Converter()
        info = {'tier': 'P1', 'variants': [0], 'combats': {'0': {
            'params': {'COMBAT_PARAM_TARGETCASTERORTOPMOST': 1}, 'callbacks': {},
            'param_calls': [['COMBAT_PARAM_TARGETCASTERORTOPMOST', 1], ['COMBATPARAM_USECHARGES', 1]]}}}
        raw = {'cast': {'tier': 'plain_combat'}, 'combats': [{'parameters': {'COMBAT_PARAM_TYPE': 'COMBAT_PHYSICALDAMAGE'}}]}
        with patch.object(importer.source_formula_evidence, 'scalar_free_combat_adapter', side_effect=lambda combat, *args: (combat, [{'source': 'proof'}])):
            _, deps, _, gaps = importer.convert_combat(converter, info, raw, {'targeting': {'needs_target': True, 'target_or_direction': False, 'needs_direction': False}}, 'candidate:test')
        self.assertEqual(converter.combat['param_calls'], [])
        self.assertEqual(deps['abilities'][0]['target_selection'], 'caster_or_top_creature')
        self.assertTrue(deps['abilities'][0]['zero_damage_health_path'])
        self.assertEqual(deps['formulas'], [])
        self.assertEqual(deps['effects'][0]['operation'], 'apply_condition')
        self.assertEqual(gaps[0]['source_field'], 'target_selection.variant_route')

    def test_import_preserves_player_module_search_path(self):
        code = """import sys
sys.path.insert(0, sys.argv[1])
before = sys.path[:]
import import_source_player_bundles
assert sys.path == before, (before, sys.path)
from verify_formal_schema import LIGHT_HEALING, CATALOG
import verify_formal_schema
assert 'spell-authoring' in verify_formal_schema.__file__, verify_formal_schema.__file__
"""
        subprocess.run([sys.executable, '-c', code, str(importer.HERE)], check=True, capture_output=True, text=True)

    def old_package(self, root):
        root.mkdir()
        importer.write(root / 'import-summary.json', {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1'})
        directory = root / 'canary-main-current' / ('a' * 16)
        directory.mkdir(parents=True)
        for name in ('spell', 'dependencies', 'catalog', 'receipt', 'source-header'):
            importer.write(directory / (name + '.json'), {'old': True})
        return directory

    def staged_blocked(self, args):
        directory = args.out / 'canary-main-current' / ('a' * 16)
        directory.mkdir(parents=True)
        importer.write(directory / 'source-header.json', {'spell': {'name': 'Blocked'}})
        importer.write(directory / 'receipt.json', {'status': 'BLOCKED'})
        importer.write(args.out / 'import-summary.json', {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1'})
        return {'status_counts': {'BLOCKED': 1}}

    def test_schema_valid_to_blocked_rebuild_has_no_stale_executable_files(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'package'
            self.old_package(output)
            with patch.object(importer, 'generate', side_effect=self.staged_blocked):
                importer.run(SimpleNamespace(out=output))
            files = {p.name for p in output.rglob('*.json')}
            self.assertEqual(files, {'source-header.json', 'receipt.json', 'import-summary.json'})

    def test_failed_rebuild_keeps_previous_package_bytes(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'package'
            directory = self.old_package(output)
            before = (directory / 'spell.json').read_bytes()
            with patch.object(importer, 'generate', side_effect=ValueError('source identity mismatch')):
                with self.assertRaisesRegex(ValueError, 'identity mismatch'):
                    importer.run(SimpleNamespace(out=output))
            self.assertEqual((directory / 'spell.json').read_bytes(), before)

    def test_unrelated_file_prevents_package_replacement(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'package'
            self.old_package(output)
            (output / 'user-notes.txt').write_text('preserve')
            with patch.object(importer, 'generate') as generate:
                with self.assertRaisesRegex(ValueError, 'unexpected existing package file'):
                    importer.run(SimpleNamespace(out=output))
                generate.assert_not_called()
            self.assertEqual((output / 'user-notes.txt').read_text(), 'preserve')

    def test_conjure_exact_item_ids_counts_no_native_placeholder(self):
        raw = {'cast': {'tier': 'conjure', 'conjure': {'reagent_item_id': 3147, 'result_item_id': 3203, 'count': 1}}}
        execution, deps, _, gaps = importer.convert_conjure(None, raw, {'carrier': 'instant'}, 'canary-main-current')
        self.assertEqual(execution['conjure']['count'], 1)
        self.assertTrue(execution['conjure']['result']['key'].endswith('/3203'))
        self.assertEqual(deps, {'abilities': [], 'effects': [], 'formulas': []})
        self.assertNotIn('native_behavior', execution)
        self.assertEqual(gaps[0]['source_field'], 'conjure.default_effect')


    def test_generated_package_full_schema_and_exact_file_closure(self):
        root = importer.HERE.parents[2] / 'docs/reference/spells/r28-source-closure/player-source-bundles'
        summary = json.loads((root / 'import-summary.json').read_text())
        self.assertEqual(summary['records'], 483)
        self.assertEqual(summary['source_populations'], {'canary-main-current': 234, 'crystal-summer-current': 249})
        manifest = json.loads((root / 'package-manifest.json').read_text())['files']
        files = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()} - {'package-manifest.json'}
        self.assertEqual(set(manifest), files)
        for name, digest in manifest.items():
            self.assertEqual(importer.sha((root / name).read_bytes()), digest)
        validator = importer.validate_spell.Draft202012Validator(json.loads((root / 'receipt.schema.json').read_text()),
                                                                  registry=importer.validate_spell.REGISTRY)
        receipts = list(root.glob('*/*/receipt.json'))
        self.assertEqual(len(receipts), 483)
        callback_records = [json.loads(line) for line in gzip.decompress((root / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines()]
        self.assertEqual(len(callback_records), 483)
        self.assertEqual({r['registration_key'] for r in callback_records},
                         {json.loads(p.read_text())['registration_key'] for p in receipts})
        for path in receipts:
            receipt = json.loads(path.read_text())
            validator.validate(receipt)
            if receipt['status'] == 'BLOCKED':
                self.assertEqual({p.name for p in path.parent.iterdir()}, {'source-header.json', 'receipt.json'})
            else:
                bundle, deps, catalog = [json.loads((path.parent / name).read_text()) for name in ('spell.json', 'dependencies.json', 'catalog.json')]
                self.assertEqual(importer.validate_spell.validate(bundle, deps, catalog), [])
            self.assertFalse(receipt['runtime_activation'])
            self.assertFalse(receipt['external_sources_used'])


if __name__ == '__main__':
    unittest.main()
