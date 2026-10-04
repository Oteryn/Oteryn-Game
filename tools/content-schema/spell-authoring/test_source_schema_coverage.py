"""Fail-closed source coverage boundaries and variant retention."""
import copy
import gzip
import tempfile
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from jsonschema import ValidationError
import source_schema_coverage as coverage


class SourceCoverageTests(unittest.TestCase):
    def setUp(self):
        self.schema = json.loads((coverage.HERE / 'spell.schema.json').read_text())
        self.row = {'registration_key': 'canary/data/a.lua#1', 'logical_key': ['instant', 'a'],
                    'file': 'data/a.lua', 'git_blob': 'a' * 40, 'sha256': 'b' * 64,
                    'source_classification': 'registered_player_spell_or_rune',
                    'engine_enabled_path': True, 'catalog_match': 'active_catalog',
                    'cast_tier': 'custom', 'registrar': {'name': 'a', 'mana': 17}}
        self.spell = {'identity': {'key': 'candidate:spell/a', 'revision': '1'},
                      'name': 'A', 'costs': {'mana': 17}}
        self.catalog = {('instant', 'a'): [self.spell]}

    def audit(self, rows):
        return coverage.audit_rows(rows, self.catalog, self.schema)

    def test_unknown_field_cannot_silently_gain_coverage(self):
        self.row['registrar']['unmappedFutureMechanic'] = True
        with self.assertRaisesRegex(ValueError, 'unmapped registrar fields'):
            self.audit([self.row])

    def test_duplicate_source_identity_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'duplicate source registration identity'):
            self.audit([self.row, copy.deepcopy(self.row)])

    def test_same_logical_key_other_source_is_preserved(self):
        other = copy.deepcopy(self.row)
        other['registration_key'] = 'crystal/data/a.lua#1'
        other['registrar']['mana'] = 99
        result = self.audit([other, self.row])
        self.assertEqual(len(result), 2)
        fields = {f['source_field']: f for f in result[1]['field_coverage']}
        self.assertEqual(fields['mana']['status'], 'source_variant_value_missing_or_changed')
        self.assertEqual(result[1]['registrar']['mana'], 99)

    def test_absence_is_not_fabricated_as_default(self):
        result = self.audit([self.row])[0]
        self.assertNotIn('needLearn', result['registrar'])
        self.assertFalse(any(f['source_field'] == 'needLearn' for f in result['field_coverage']))
        self.assertEqual(result['execution_data_coverage'], 'not_established_by_registrar_capture')

    def test_boolean_does_not_pass_integer_schema(self):
        self.row['registrar']['mana'] = True
        with self.assertRaises(ValidationError):
            self.audit([self.row])

    def test_harmony_cost_remains_distinct_from_role(self):
        self.row['registrar']['harmony'] = True
        self.spell['harmony_role'] = 'spender'
        fields = {f['source_field']: f for f in self.audit([self.row])[0]['field_coverage']}
        self.assertEqual(fields['harmony']['status'], 'source_variant_value_missing_or_changed')
        self.assertEqual(fields['harmony']['authoring_path'], '/harmony_cost')
        self.assertTrue(fields['harmony']['normalized_expected_value'])

    def test_disabled_and_removed_source_is_not_dropped(self):
        self.row.update(engine_enabled_path=False, catalog_match='source_proven_removed',
                        source_classification='disabled_example_or_test_fixture')
        result = self.audit([self.row])
        self.assertEqual(len(result), 1)
        self.assertFalse(result[0]['engine_enabled_path'])
        self.assertEqual(result[0]['catalog_match'], 'source_proven_removed')

    def test_source_digest_tampering_fails(self):
        data = b'source bytes\n'
        import hashlib
        blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
        self.row['git_blob'] = blob
        tree = f'100644 blob {blob}\tdata/a.lua\0'.encode()
        payload = f'{blob} blob {len(data)}\n'.encode() + data + b'\n'
        with patch.object(coverage.subprocess, 'check_output', side_effect=[tree, payload]):
            with self.assertRaisesRegex(ValueError, 'digest mismatch'):
                coverage.verify_source_bytes([self.row], Path('/unused'), 'exact-revision')

    def test_schema_path_missing_is_not_reported_as_present(self):
        del self.schema['$defs']['spell']['properties']['costs']['properties']['mana']
        field = next(f for f in self.audit([self.row])[0]['field_coverage'] if f['source_field'] == 'mana')
        self.assertEqual(field['status'], 'schema_path_missing')


    def test_group_pair_exact_and_incomplete_secondary_refused(self):
        reg = {'group': ['attack', 'focus'], 'groupCooldown': [2000, 10000]}
        expected, _ = coverage.transformed('group', reg, {})
        self.assertEqual(expected, [{'group': 'attack', 'cooldown_ms': 2000},
                                    {'group': 'focus', 'cooldown_ms': 10000}])
        reg['groupCooldown'] = 2000
        with self.assertRaisesRegex(ValueError, 'arity mismatch'):
            coverage.transformed('groupCooldown', reg, {})

    def test_vocation_flags_preserved_no_promoted_inference(self):
        expected, metadata = coverage.transformed('vocation', {'vocation': ['druid;true', 'elder druid;false']}, {})
        self.assertEqual(expected, ['druid', 'elder_druid'])
        self.assertEqual(metadata['unrepresented_source_semantics'][1],
                         {'vocation': 'elder_druid', 'show_in_description': False})
        with self.assertRaisesRegex(ValueError, 'unknown vocation'):
            coverage.transformed('vocation', {'vocation': ['future vocation']}, {})

    def test_rune_source_id_cannot_select_provider_revision(self):
        expected, metadata = coverage.transformed('runeId', {'runeId': 3203}, {})
        self.assertEqual(expected, {'family': 'Item', 'key': 'candidate:item/3203'})
        self.assertIn('revision', metadata)
        self.assertNotIn('revision', expected)

    def test_blocking_arguments_preserve_creature_default(self):
        self.assertEqual(coverage.transformed('isBlocking', {'isBlocking': True}, {})[0],
                         {'solid': True, 'creature': False})
        self.assertEqual(coverage.transformed('isBlocking', {'isBlocking': [False, True]}, {})[0],
                         {'solid': False, 'creature': True})

    def test_param_source_branch_requires_has_params(self):
        self.assertEqual(coverage.transformed('hasPlayerNameParam', {'hasPlayerNameParam': True}, {})[0], 'none')
        self.assertEqual(coverage.transformed('hasParams', {'hasParams': True, 'hasPlayerNameParam': True}, {})[0], 'player_name')
        self.assertEqual(coverage.transformed('hasParams', {'hasParams': True}, {})[0], 'text')

    def test_sound_registered_owner_enum_required_and_silence_explicit(self):
        reg = {'castSound': 'SOUND_EFFECT_TYPE_SPELL_OR_RUNE'}
        self.assertEqual(coverage.transformed('castSound', reg, {'SPELL_OR_RUNE': 10})[0], 'canary.sound:spell_or_rune')
        with self.assertRaisesRegex(ValueError, 'not registered'):
            coverage.transformed('castSound', reg, {})
        with self.assertRaisesRegex(ValueError, 'same numeric value'):
            coverage.transformed('castSound', reg, {'SPELL_OR_RUNE': 999})
        self.assertIsNone(coverage.transformed('castSound', {'castSound': 'SOUND_EFFECT_TYPE_SILENCE'}, {'SILENCE': 0})[0])

    def test_monk_role_unknown_refused(self):
        self.assertEqual(coverage.transformed('monkSpellType', {'monkSpellType': 'MonkSpell_Spender'}, {})[0], 'spender')
        with self.assertRaisesRegex(ValueError, 'unknown MonkSpell'):
            coverage.transformed('monkSpellType', {'monkSpellType': 'MonkSpell_Future'}, {})

    def test_source_none_vocation_is_preserved_in_schema(self):
        self.row['registrar']['vocation'] = ['none']
        field = next(f for f in self.audit([self.row])[0]['field_coverage'] if f['source_field'] == 'vocation')
        self.assertEqual(field['normalized_expected_value'], ['none'])
        self.assertEqual(field['status'], 'source_vocation_display_flags_missing_in_selected_catalog')
        self.assertEqual(field['unsupported_authoring_vocations'], [])


    def source_report(self):
        return {'sources': {'canary': {'revision': 'c' * 40, 'verified_source_files': 1,
                                      'records': [self.audit([self.row])[0]]}}}

    def test_export_is_deterministic_small_source_only_and_validated(self):
        with tempfile.TemporaryDirectory() as temp:
            a, b = Path(temp) / 'a.gz', Path(temp) / 'b.gz'
            proof = coverage.export_source_data(self.source_report(), a)
            coverage.export_source_data(self.source_report(), b)
            self.assertEqual(a.read_bytes(), b.read_bytes())
            record = json.loads(gzip.decompress(a.read_bytes()))
            self.assertEqual(record['registrar'], self.row['registrar'])
            self.assertEqual(record['source_revision'], 'c' * 40)
            self.assertFalse(record['execution_mapped'])
            self.assertNotIn('field_coverage', record)
            self.assertTrue(proof['all_records_schema_valid'])
            self.assertEqual(proof['records'], 1)

    def test_export_rejects_unknown_registrar_field_without_file_write(self):
        report = self.source_report()
        report['sources']['canary']['records'][0]['registrar']['futureMechanic'] = 1
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'data.gz'
            with self.assertRaises(ValidationError):
                coverage.export_source_data(report, output)
            self.assertFalse(output.exists())

    def test_export_rejects_duplicate_and_mismatched_source_keys(self):
        report = self.source_report()
        records = report['sources']['canary']['records']
        records.append(copy.deepcopy(records[0]))
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaisesRegex(ValueError, 'duplicate or mismatched'):
                coverage.export_source_data(report, Path(temp) / 'data.gz')
            records.pop()
            records[0]['registration_key'] = 'crystal/data/a.lua#1'
            with self.assertRaisesRegex(ValueError, 'duplicate or mismatched'):
                coverage.export_source_data(report, Path(temp) / 'data.gz')

    def test_export_retains_removed_source_variant_and_exact_digest(self):
        report = self.source_report()
        row = report['sources']['canary']['records'][0]
        row['catalog_match'] = 'source_proven_removed'
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'data.gz'
            coverage.export_source_data(report, output)
            record = json.loads(gzip.decompress(output.read_bytes()))
            self.assertEqual(record['catalog_match'], 'source_proven_removed')
            self.assertEqual(record['source_sha256'], self.row['sha256'])


    def test_export_roundtrip_preserves_source_only_harmony_and_vocation_flags(self):
        report = self.source_report()
        source = report['sources'].pop('canary')
        report['sources']['crystal'] = source
        row = source['records'][0]
        row['registration_key'] = 'crystal/data/a.lua#1'
        row['registrar'].update(harmony=True, needPosition=True,
                                vocation=['monk;true', 'exalted monk;false'])
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'data.gz'
            coverage.export_source_data(report, output)
            record = json.loads(gzip.decompress(output.read_bytes()))
            self.assertEqual(record['registrar'], row['registrar'])
            self.assertTrue(record['registrar']['harmony'])
            self.assertTrue(record['registrar']['needPosition'])
            self.assertEqual(record['registrar']['vocation'], ['monk;true', 'exalted monk;false'])
            self.assertNotIn('monkSpellType', record['registrar'])
            self.assertFalse(record['execution_mapped'])


    def test_projection_preserves_source_only_fields_no_defaults_or_execution(self):
        self.row['registrar'].update(harmony=True, needPosition=True, vocation=['none'],
                                     element='COMBAT_FIREDAMAGE', stance='elemental', cooldown=0)
        row = self.audit([self.row])[0]
        spell, gaps = coverage.project_source_row(row)
        self.assertEqual(gaps, [])
        self.assertTrue(spell['harmony_cost'])
        self.assertEqual(spell['elemental_cast_type'], 'fire')
        self.assertEqual(spell['stance_slot'], 'elemental')
        self.assertEqual(spell['cooldown_ms'], 0)
        self.assertEqual(spell['requirements']['vocations'], ['none'])
        self.assertEqual(spell['requirements']['vocation_display_flags'],
                         [{'vocation': 'none', 'show_in_description': False}])
        self.assertNotIn('learning_required', spell['requirements'])
        self.assertNotIn('soul', spell['costs'])
        self.assertNotIn('execution', spell)
        self.assertNotIn('identity', spell)

    def test_projection_exports_all_variants_schema_valid_and_deterministic(self):
        report = self.source_report()
        with tempfile.TemporaryDirectory() as temp:
            a, b = Path(temp) / 'a.gz', Path(temp) / 'b.gz'
            proof = coverage.export_source_projection(report, a)
            coverage.export_source_projection(report, b)
            self.assertEqual(a.read_bytes(), b.read_bytes())
            self.assertTrue(proof['all_records_schema_valid'])
            row = json.loads(gzip.decompress(a.read_bytes()))
            self.assertEqual(row['spell']['costs']['mana'], 17)
            self.assertFalse(proof['native_consumption_established'])
            self.assertFalse(proof['full_executable_spell'])

    def test_projection_unknown_sound_is_explicit_gap_not_guessed_alias(self):
        self.row['registrar']['castSound'] = 'SOUND_EFFECT_TYPE_UNBOUND_FUTURE_SOUND'
        spell, gaps = coverage.project_source_row(self.audit([self.row])[0])
        self.assertNotIn('cast_cue', spell['presentation'])
        self.assertEqual(spell['presentation']['source_cast_sound'], {'constant': 'SOUND_EFFECT_TYPE_UNBOUND_FUTURE_SOUND', 'binding_status': 'unbound'})
        self.assertEqual(gaps, [])

    def test_candidate_source_semantics_reject_invalid_enum_and_harmony_type(self):
        from referencing import Registry, Resource
        monster = json.loads((coverage.HERE.parent / 'monster-authoring/monster.schema.json').read_text())
        registry = Registry().with_resources((x['$id'], Resource.from_contents(x)) for x in [self.schema, monster])
        validator = coverage.Draft202012Validator({'$ref': self.schema['$id'] + '#/$defs/sourceProjection'}, registry=registry)
        validator.validate({'harmony_cost': True, 'stance_slot': 'standard', 'elemental_cast_type': 'death', 'cooldown_ms': 0})
        for invalid in ({'harmony_cost': 1}, {'stance_slot': 'invented'}, {'elemental_cast_type': 'holy'}, {'cooldown_ms': -1}):
            with self.assertRaises(ValidationError):
                validator.validate(invalid)
        with self.assertRaisesRegex(ValueError, 'unsupported source elemental'):
            coverage.transformed('element', {'element': 'COMBAT_HOLYDAMAGE'}, {})


    def test_source_sound_references_distinguish_cpp_absent_and_not_lua_registered(self):
        value = 'SOUND_EFFECT_TYPE_TEST_KNOWN'
        self.assertEqual(coverage.source_sound_reference(value, {}), {'constant': value, 'binding_status': 'unbound'})
        self.assertEqual(coverage.source_sound_reference(value, {'__cpp_values__': {'TEST_KNOWN': 123}}),
                         {'constant': value, 'binding_status': 'not_lua_registered', 'source_numeric_value': 123})
        self.assertEqual(coverage.source_sound_reference(value, {'TEST_KNOWN': 123, '__cpp_values__': {'TEST_KNOWN': 123}})['binding_status'], 'knownbound')

    def test_unbound_reference_cannot_fabricate_numeric_value(self):
        from referencing import Registry, Resource
        monster = json.loads((coverage.HERE.parent / 'monster-authoring/monster.schema.json').read_text())
        registry = Registry().with_resources((x['$id'], Resource.from_contents(x)) for x in [self.schema, monster])
        validator = coverage.Draft202012Validator({'$ref': self.schema['$id'] + '#/$defs/sourceProjection'}, registry=registry)
        good = {'presentation': {'source_cast_sound': {'constant': 'SOUND_EFFECT_TYPE_UNBOUND', 'binding_status': 'unbound'}}}
        validator.validate(good)
        good['presentation']['source_cast_sound']['source_numeric_value'] = 0
        with self.assertRaises(ValidationError):
            validator.validate(good)

    def test_projection_refuses_native_sound_numeric_mismatch_before_write(self):
        report = self.source_report()
        row = report['sources']['canary']['records'][0]
        row['registrar']['castSound'] = 'SOUND_EFFECT_TYPE_SPELL_OR_RUNE'
        row['field_coverage'].append({'source_field': 'castSound', 'authoring_path': '/presentation/cast_cue',
                                      'normalized_expected_value': 'canary.sound:spell_or_rune',
                                      'transform_details': {'source_enum_value': 999}})
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / 'data.gz'
            with self.assertRaisesRegex(ValueError, 'numeric mismatch before'):
                coverage.export_source_projection(report, out)
            self.assertFalse(out.exists())

    def test_disabled_symbolic_carrier_stays_source_only_and_not_gameplay_carrier(self):
        self.row['logical_key'][0] = '@spell_instant'
        self.row['source_classification'] = 'disabled_example_or_test_fixture'
        spell, gaps = coverage.project_source_row(self.audit([self.row])[0])
        self.assertEqual(spell['source_carrier_symbol'], '@spell_instant')
        self.assertNotIn('carrier', spell)
        self.assertEqual(gaps, [])
        self.assertNotIn('source_carrier_symbol', self.schema['$defs']['spell']['properties'])


if __name__ == '__main__':
    unittest.main()
