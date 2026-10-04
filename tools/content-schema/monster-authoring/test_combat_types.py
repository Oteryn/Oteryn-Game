"""Regression for Crystal's C++ enum iteration aliases leaking into conversion."""
import unittest
from types import SimpleNamespace

import canary_batch as cb
from combat_types import reverse_combat_types


CRYSTAL_TYPES = {'COMBAT_PHYSICALDAMAGE': 0, 'COMBAT_HEALING': 7,
                 'COMBAT_NEUTRALDAMAGE': 13, 'COMBAT_COUNT': 14,
                 'COMBAT_FIRST': 0, 'COMBAT_LAST': 13, 'COMBAT_NONE': 255}


def converter(types=CRYSTAL_TYPES):
    result = cb.Converter.__new__(cb.Converter)
    result.spell_scripts = SimpleNamespace(enums={
        'CombatParam_t': {'COMBAT_PARAM_TYPE': 0}, 'CombatType_t': types,
        'ConditionType_t': {}})
    result.magic_effects, result.missiles, result.item_ids = {}, {}, {}
    return result


class CombatTypes(unittest.TestCase):
    def test_actual_physical_parameter_produces_damage_effect(self):
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        combat = {'param_calls': [('COMBAT_PARAM_TYPE', 'COMBAT_PHYSICALDAMAGE')],
                  'callbacks': {}, 'conditions': []}
        uses_magnitude = converter().combat_ability('canary:ability/regression', combat,
                                                   {}, 1, deps, None, [])
        self.assertTrue(uses_magnitude)
        self.assertEqual('physical', deps['effects'][0]['damage_type'])
        self.assertEqual('damage', deps['effects'][0]['operation'])

    def test_neutral_alias_keeps_registered_damage_name(self):
        params = converter().engine_params([('COMBAT_PARAM_TYPE', 'COMBAT_NEUTRALDAMAGE')], [])
        self.assertEqual('COMBAT_NEUTRALDAMAGE', params['COMBAT_PARAM_TYPE'])

    def test_canary_enum_without_aliases_is_unchanged(self):
        values = {k: v for k, v in CRYSTAL_TYPES.items() if k not in ('COMBAT_FIRST', 'COMBAT_LAST')}
        self.assertEqual({v: k for k, v in values.items()}, reverse_combat_types(values))

    def test_changed_alias_value_is_not_guessed(self):
        values = {**CRYSTAL_TYPES, 'COMBAT_FIRST': 99}
        self.assertEqual('COMBAT_FIRST', reverse_combat_types(values)[99])

    def test_unknown_type_still_refuses_conversion(self):
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        combat = {'param_calls': [('COMBAT_PARAM_TYPE', 99)], 'callbacks': {}, 'conditions': []}
        with self.assertRaisesRegex(cb.SpellUnresolved, 'no authoring damage type'):
            converter().combat_ability('canary:ability/unknown', combat, {}, 1, deps, None, [])


if __name__ == '__main__':
    unittest.main()
