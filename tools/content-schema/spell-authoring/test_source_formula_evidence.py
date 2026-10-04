import json
from pathlib import Path
import unittest
from unittest.mock import patch
from jsonschema import Draft202012Validator
import source_formula_evidence as evidence

CRYSTAL = ('crystal-summer-current', '00ce02a57ca5a12e48f32a3476e37471167e4c3f')
CANARY = ('canary-main-current', '04b83b512114bfd888000d6e1433ed8ecaec7c5b')


class SourceFormulaEvidenceTests(unittest.TestCase):
    def test_parser_preserves_coefficients_order_and_refuses_dynamic_code(self):
        self.assertEqual(evidence.expression('0.88 * (level / 5)'),
                         {'op': 'mul', 'args': [{'const': '0.88'}, {'op': 'div', 'args': [{'var': 'level'}, {'const': '5'}]}]})
        for value in ('dangerous(level)', 'level ** 2', 'a[1]', '__import__("os")'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                evidence.expression(value)

    def test_exact_pinned_crystal_helper_inlines_without_worldcurve_function(self):
        helper, proof = evidence.crystal_helper(*CRYSTAL)
        result = evidence.adapt_expression({'fn': 'base_damage_healing', 'args': [{'var': 'level'}]}, helper)
        self.assertEqual(result, helper)
        self.assertNotIn('fn', json.dumps(result))
        self.assertIn('2025', json.dumps(result))
        self.assertEqual(proof['path'], 'data/scripts/lib/register_spells.lua')
        self.assertEqual(proof['line_start'], 602)
        self.assertEqual(len(proof['body_sha256']), 64)

    def test_unknown_helpers_are_not_replaced_by_crystal_curve(self):
        with self.assertRaises(ValueError):
            evidence.adapt_expression({'fn': 'flat_damage_healing', 'args': [{'var': 'level'}]}, None)

    def test_shield_input_requires_actual_source_getter_or_helper_proof(self):
        with self.assertRaises(ValueError):
            evidence.adapt_expression({'var': 'skill:SKILL_SHIELD'}, None)
        enabled, proof = evidence.shielding_evidence(*CRYSTAL, 'calculateKnightHealing(player, magicLevel, basePower)')
        self.assertTrue(enabled)
        self.assertEqual(proof['function'], 'calculateKnightHealing')
        self.assertEqual(evidence.adapt_expression({'var': 'skill:SKILL_SHIELD'}, None, enabled), {'var': 'shielding_skill'})

    def test_canary_cpp_tier_program_retains_actual_state_types_and_clamp(self):
        program, proof = evidence.flat_helper(*CANARY)
        initial = {row['target']: row for row in program['initializers']}
        self.assertEqual(initial['threshold']['value'], {'const': '500'})
        self.assertEqual(initial['thresholdStep']['value'], {'const': '600'})
        self.assertEqual(initial['previousLevelsAggregatedBaseline']['scalar_type'], 'double')
        self.assertEqual(program['while']['comparison'], 'gte')
        self.assertEqual(program['return'], {'op': 'min', 'args': [{'var': 'computed'}, {'const': '65535'}]})
        self.assertEqual(proof['function'], 'Player::calculateFlatDamageHealing')

    def test_scalar_free_adapter_removes_both_parameter_views_only_on_safe_player_path(self):
        for snapshot in (CANARY, CRYSTAL):
            with self.subTest(snapshot=snapshot):
                original = {'params': {'COMBAT_PARAM_TYPE': 'COMBAT_ENERGYDAMAGE', 'COMBAT_PARAM_CREATEITEM': 1490},
                            'param_calls': [('COMBAT_PARAM_TYPE', 'COMBAT_ENERGYDAMAGE'), ('COMBAT_PARAM_CREATEITEM', 1490)], 'conditions': []}
                result, proofs = evidence.scalar_free_combat_adapter(original, {'parameters': {}}, *snapshot)
                self.assertNotIn('COMBAT_PARAM_TYPE', result['params'])
                self.assertEqual(result['param_calls'], [('COMBAT_PARAM_CREATEITEM', 1490)])
                self.assertEqual(len(proofs), 6)
                self.assertIn('COMBAT_PARAM_TYPE', original['params'])
                self.assertEqual(len(original['param_calls']), 2)

    def test_value_callback_or_explicit_formula_never_removed(self):
        combat = {'params': {'COMBAT_PARAM_TYPE': 'COMBAT_FIREDAMAGE'}, 'conditions': [{}]}
        for raw in ({'callbacks': [{'kind': 'CALLBACK_PARAM_LEVELMAGICVALUE'}]}, {'set_formula': [['COMBAT_FORMULA_DAMAGE', 0, 0, 0, 0]]}):
            with self.subTest(raw=raw):
                result, proofs = evidence.scalar_free_combat_adapter(combat, raw, *CANARY)
                self.assertEqual(result, combat)
                self.assertEqual(proofs, [])

    def test_source_default_change_is_rejected(self):
        actual = evidence.source
        def tampered(snapshot, revision, path):
            data, proof = actual(snapshot, revision, path)
            if path.endswith('combat.hpp'):
                data = data.replace(b'formulaType_t formulaType = COMBAT_FORMULA_UNDEFINED;', b'formulaType_t formulaType = COMBAT_FORMULA_DAMAGE;')
            return data, proof
        with patch.object(evidence, 'source', side_effect=tampered), self.assertRaises(ValueError):
            evidence.scalar_free_combat_adapter({'params': {'COMBAT_PARAM_TYPE': 'COMBAT_FIREDAMAGE'}, 'conditions': [{}]}, {}, *CANARY)

    def test_schema_rejects_untyped_function_and_unknown_program_fields(self):
        schema = json.loads((Path(__file__).parent / 'source-formula-evidence.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator({'$ref': '#/$defs/expression', '$defs': schema['$defs']})
        validator.validate(evidence.expression('math.sqrt(2 * level + 2025)'))
        self.assertTrue(list(validator.iter_errors({'fn': 'worldcurve', 'args': [{'var': 'level'}]})))
        self.assertTrue(list(validator.iter_errors({'op': 'sqrt', 'args': [{'const': '2'}, {'const': '3'}]})))
        self.assertTrue(list(validator.iter_errors({'const': 'arbitrary_source_code'})))


if __name__ == '__main__':
    unittest.main()
