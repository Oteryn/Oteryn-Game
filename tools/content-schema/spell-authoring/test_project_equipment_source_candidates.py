"""R60 native source DATA; private schema and independent Lua numeric checks."""
import copy
import json
from pathlib import Path
import re
import unittest
from jsonschema import Draft202012Validator
from lupa import LuaRuntime
import project_equipment_source_candidates as project
import validate_spell

ROOT = Path(__file__).resolve().parents[3]
SOURCES = Path('/workspace/spell-sources')


class EquipmentSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet, cls.schema = project.build(ROOT, SOURCES)
        cls.records = cls.packet['import-summary.json']['records_index']

    def bundle(self, name, donor='crystal'):
        record = next(row for row in self.records if row['registration_key'].startswith(donor + '-')
                      and row['registration_key'].endswith('/' + name + '.lua#1'))
        prefix = record['source_header_path'].rsplit('/', 1)[0] + '/'
        return self.packet[prefix + 'spell.json']['spell'], self.packet[prefix + 'dependencies.json']

    def test_23_complete_data_headers_and_private_contract_flags(self):
        self.assertEqual(len(self.records), 23)
        for row in self.records:
            self.assertEqual(row['status'], 'CANDIDATE_SCHEMA_VALID')
            self.assertTrue(row['native_data_model_complete'])
            self.assertTrue(row['authoring_contract_extension_pending'])
            self.assertFalse(row['source_consumer_implemented'])
            self.assertFalse(row['runtime_activation'])
            self.assertEqual(row['required_operations_unrepresented'], [])
            raw = self.packet[row['source_header_path']]
            self.assertEqual(raw, (ROOT / 'imports/spells/r28/player-source-bundles' / row['source_header_path']).read_bytes())

    def test_closed_native_schema_rejects_unrepresented_key_and_route_mutation(self):
        Draft202012Validator.check_schema(self.schema)
        native_schema = self.schema['$defs']['nativeBehavior']
        spell, _ = self.bundle('double_jab')
        native = spell['execution']['native_behavior']
        Draft202012Validator(native_schema).validate(native)
        altered = copy.deepcopy(native)
        altered['parameters']['invented_operation'] = True
        self.assertFalse(Draft202012Validator(native_schema).is_valid(altered))
        altered = copy.deepcopy(native)
        altered['parameters']['routes'].pop()
        self.assertFalse(Draft202012Validator(native_schema).is_valid(altered))

    def test_elemental_and_area_branch_selection_is_explicit(self):
        spell, deps = self.bundle('flurry_of_blows')
        routes = spell['execution']['native_behavior']['parameters']['routes']
        self.assertEqual(len(routes), 6)
        self.assertEqual({row['damage_type'] for row in routes}, {'physical', 'energy', 'earth'})
        self.assertEqual([row['area_variant'] for row in routes], ['default'] * 3 + ['wheel_augmented'] * 3)
        self.assertEqual(len(deps['abilities']), 6)
        spell, _ = self.bundle('sweeping_takedown')
        self.assertEqual([row['hit_order'] for row in spell['execution']['native_behavior']['parameters']['routes']], [0] * 3 + [1] * 3)

    def test_source_focus_and_shield_are_not_canonical_aliases(self):
        focus, _ = self.bundle('focus_serenity')
        params = focus['execution']['native_behavior']['parameters']
        self.assertEqual(params['reset_cooldowns'], 'all_spell_and_group')
        self.assertFalse(params['gain_healing'])
        self.assertEqual(params['rearm_individual'], 281)
        shield, deps = self.bundle('shield_slam')
        params = shield['execution']['native_behavior']['parameters']
        self.assertEqual(params['equipment']['shield_selection'], 'highest_defense_left_tie')
        self.assertEqual(params['shield_debuff']['duration_ms'], 10000)
        self.assertEqual(params['shield_debuff']['wheel_grade_2_damage_dealt_percent'], 25)
        formula = deps['formulas'][0]
        self.assertIn('shield_defense', json.dumps(formula))
        self.assertTrue(shield['needs_shield'])

    def test_raw_spiritual_sign_and_event_rules_retained(self):
        spell, deps = self.bundle('spiritual_outburst')
        self.assertTrue(all(effect['operation'] == 'damage' for effect in deps['effects']))
        chain = spell['execution']['native_behavior']['parameters']['chain']
        self.assertEqual(chain['recast_delay_ms'], 1500)
        self.assertEqual(chain['source_direct_health_sign_route'], 'negate_and_swap_signed_bounds')
        self.assertEqual(chain['target_health_sign'], 'negative_offensive_delta')
        self.assertTrue(chain['canonical_health_sign_normalization'])
        self.assertEqual(chain['recast_caster'], 'captured_userdata_no_guard')
        self.assertEqual(chain['recast_grade'], 'captured_precast')
        self.assertEqual(chain['recast_damage_inputs'], 'captured_weapon_attack_then_fresh_skill_level_power')
        spell, deps = self.bundle('spiritual_outburst', 'canary')
        self.assertTrue(all(effect['operation'] == 'damage' for effect in deps['effects']))
        chain = spell['execution']['native_behavior']['parameters']['chain']
        self.assertEqual(chain['recast_delay_ms'], 1000)
        self.assertEqual(chain['source_direct_health_sign_route'], 'positive_skill_callback')
        self.assertEqual(chain['target_health_sign'], 'negative_offensive_delta')
        self.assertTrue(chain['canonical_health_sign_normalization'])
        self.assertEqual(chain['recast_caster'], 'guid_reacquired_if_present')
        self.assertEqual(chain['recast_grade'], 'fresh_callback_revelation_stage')
        self.assertEqual(spell['execution']['native_behavior']['parameters']['harmony']['builder_cooldown_reduction_ms_per_spent_charge'], 2000)

    def test_standard_formula_matches_independent_exact_source_lua_helper(self):
        lua = LuaRuntime(unpack_returned_tuples=True)
        source = project.base.source_file(SOURCES / 'crystal', project.prior.PINS['crystal'], 'data/scripts/lib/register_spells.lua').decode()
        for name in ('calculateBaseDamageHealing', 'calculateAttackValue', 'calculateMonkSpellDamage'):
            helper = re.search(r'(?ms)^function ' + name + r'\(.*?^end$', source).group(0)
            lua.execute(helper)
        function = lua.eval('function(level,skill,attack,power,factor) local p={getLevel=function() return level end}; return calculateMonkSpellDamage(p,skill,attack,power,factor) end')
        for name, factor in [('double_jab', .9), ('flurry_of_blows', .6), ('swift_jab', .7), ('thousand_fist_blows', .8)]:
            spell, deps = self.bundle(name)
            formula = deps['formulas'][0]
            for level, skill, attack in [(1, 0, 7), (100, 90, 36), (350, 110, 55), (1000, 180, 80), (10000, 500, 200)]:
                env = {'level': level, 'attack_skill': skill, 'attack_value': attack, 'base_power': spell['base_power']}
                expected = function(level, skill, attack, spell['base_power'], factor)
                self.assertAlmostEqual(float(validate_spell.evaluate(formula['minimum'], env)), expected * .9, places=8)
                self.assertAlmostEqual(float(validate_spell.evaluate(formula['maximum'], env)), expected * 1.1, places=8)


if __name__ == '__main__':
    unittest.main()
