import copy
import gzip
import json
import pathlib
import unittest

import source_paralyze_partial_templates as adapter
import validate_spell

REPO = pathlib.Path(__file__).resolve().parents[3]
ROOT = pathlib.Path('/workspace/spell-sources')


class ParalyzePartialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = adapter.build(REPO, ROOT)
        cls.schema = json.loads(adapter.SCHEMA_PATH.read_text())
        cls.validator = validate_spell.Draft202012Validator(cls.schema, registry=validate_spell.REGISTRY)

    def test_exact_use_rune_population_and_no_spell_candidates(self):
        self.assertEqual({snapshot + '/' + adapter.SOURCE_PATH + '#1' for snapshot in adapter.PINS}, {row['registration_key'] for row in self.rows})
        for row in self.rows:
            self.validator.validate(row)
            self.assertEqual('BLOCKED', row['full_spell_status'])
            self.assertEqual(0, row['candidate_count'])
            self.assertFalse(row['executable_spell_exported'])
            self.assertNotIn('spell', row)
            self.assertTrue(row['runtime_blocked'])
            self.assertFalse(row['model_projection_equivalence'])
            self.assertFalse(row['source_program_fully_represented'])

    def test_existing_condition_formula_and_ability_schemas(self):
        for row in self.rows:
            deps = row['dependency_templates']
            self.assertEqual([], validate_spell.structural('spell-dependencies.schema.json', deps))
            self.assertTrue(deps['abilities'][0]['zero_damage_health_path'])
            self.assertEqual(6000, deps['effects'][0]['duration_ms'])
            self.assertEqual('fixed_duration', deps['effects'][0]['condition']['lifetime'])
            self.assertEqual('paralyze', deps['effects'][0]['condition']['type'])
            self.assertEqual('speed_modifier', deps['formulas'][0]['kind'])
            self.assertEqual({'mina': '-1', 'minb': '0', 'maxa': '-1', 'maxb': '0'}, row['source_condition_declaration']['speed_formula'])
            self.assertTrue(all(effect['operation'] != 'damage' for effect in deps['effects']))
            self.assertEqual('COMBAT_UNDEFINEDDAMAGE', row['cpp_control_proof']['source_combat_type'])
            self.assertEqual('non_COMBAT_NONE_to_health_path', row['cpp_control_proof']['route'])

    def test_donor_namespace_is_not_inherited_from_generic_completion(self):
        crystal = next(row for row in self.rows if row['source_identity']['source'] == 'crystal')
        presentation = crystal['dependency_templates']['effects'][0]['presentation']
        self.assertEqual('crystal.appearance:effect/magic_red', presentation['impact_asset_binding'])
        self.assertEqual('crystal.appearance:effect/magic_green', presentation['caster_effect_asset_binding'])
        self.assertNotIn('canary.appearance', json.dumps(presentation))

    def test_source_affine_clamp_is_base_domain_only(self):
        for row in self.rows:
            proof = row['cpp_control_proof']['speed_normalization']
            self.assertEqual('base_speed_minus_40', proof['source_formula_variable'])
            self.assertEqual([-1, 0, -1, 0], proof['source_coefficients'])
            self.assertFalse(proof['final_creature_speed_qualified'])
            self.assertFalse(proof['stacking_and_var_speed_provider_qualified'])
            for base in [0, 1, 39, 40, 100, 300, 65535]:
                source_formula_value = -1 * (base - 40)
                delta = max(source_formula_value - base, 40 - base)
                self.assertEqual(40, base + delta)
            self.assertEqual(40, row['dependency_templates']['formulas'][0]['speed']['minimum_offset'])

    def test_execute_boolean_not_health_success_is_preserved(self):
        for row in self.rows:
            cpp = row['cpp_control_proof']
            self.assertEqual('initialized_true_doCombat_return_not_assigned', cpp['number_variant_result'])
            self.assertEqual('Lua_Combat_execute_boolean_not_health_or_condition_application', cpp['caster_success_scope'])
            self.assertEqual(['combatBlockHit', 'combatChangeHealth', 'conditional_CombatConditionFunc', 'conditional_CombatDispelFunc'], cpp['owning_order'])
            presentation = row['dependency_templates']['effects'][0]['presentation']
            self.assertEqual('after_success', presentation['caster_effect_timing'])
            self.assertEqual(row['source_identity']['source'] + '.appearance:effect/magic_green', presentation['caster_effect_asset_binding'])
            self.assertEqual(row['source_identity']['source'] + '.appearance:effect/magic_red', presentation['impact_asset_binding'])
            self.assertEqual('caster_position_magic_effect_if_execute_true', row['source_cast_program'][2]['operation'])
            self.assertFalse(cpp['final_damage_value_qualified'])
            self.assertIn('target immune', row['contract_correction_needed']['expectation_quote'])

    def test_changed_lua_identity_and_promoted_flags_are_refused(self):
        data = adapter.source_file(ROOT, 'canary', adapter.PINS['canary-main-current'], adapter.SOURCE_PATH)
        with self.assertRaises(ValueError):
            adapter.condition_from_source(data.replace(b'6000', b'7000'))
        for field, value in [('candidate_count', 1), ('model_projection_equivalence', True), ('runtime_blocked', False)]:
            modified = copy.deepcopy(self.rows[0]); modified[field] = value
            with self.subTest(field=field), self.assertRaises(Exception):
                self.validator.validate(modified)
        modified = copy.deepcopy(self.rows[0]); modified['spell'] = {}
        with self.assertRaises(Exception):
            self.validator.validate(modified)

    def test_changed_cpp_boolean_mapping_fails_closed(self):
        source = 'canary'; revision = adapter.PINS['canary-main-current']
        paths = ['src/creatures/combat/combat.cpp', 'src/lua/functions/creatures/combat/combat_functions.cpp', 'src/creatures/creatures_definitions.hpp', 'src/creatures/combat/combat.hpp', 'src/creatures/creature.hpp', 'src/creatures/players/player.hpp', 'src/creatures/combat/condition.cpp', 'src/lua/functions/creatures/combat/condition_functions.cpp']
        current = {path: adapter.source_file(ROOT, source, revision, path) for path in paths}
        before = {path: adapter.source_file(ROOT, source, adapter.OLD[source], path) for path in [paths[0], paths[1], paths[6]]}
        modified = dict(current); bridge = paths[1]
        modified[bridge] = modified[bridge].replace(b'bool result = true;', b'bool result = false;')
        with self.assertRaisesRegex(ValueError, 'boolean scope changed'):
            adapter.qualify_cpp(modified, before)


if __name__ == '__main__':unittest.main()
