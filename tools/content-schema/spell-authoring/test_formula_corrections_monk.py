"""Numerical regressions from genuine backend probes; no invented RNG average."""
import copy
import math
import unittest

from formula_corrections_monk import (
    BUILDER_BASE_POWERS, SPENDER_SOURCES, baseline_spender_formula, center_expression, correct,
)
from validate_spell import (
    Draft202012Validator, REGISTRY, SCHEMAS, check_formula, evaluate,
)


NAMES = ('chained penance', 'flurry of blows', 'greater flurry of blows',
         'mystic repulse', 'forceful uppercut', 'thousand fist blows', 'double jab')
# Unchanged genuine computeDamage@eeed345c executed with no perks/stances/targets.
# Ordering is NAMES; each row: level, skill, weapon attack, API raw averages.
API_CASES = (
    (1000, 140, 46, (651, 551, 758, 752, 1053, 598, 451)),
    (80, 140, 46, (484, 384, 591, 585, 886, 431, 284)),
    (1, 140, 46, (468, 368, 575, 569, 870, 415, 268)),
    (499, 140, 46, (567, 467, 674, 668, 969, 514, 367)),
    (501, 140, 46, (568, 468, 675, 669, 970, 515, 368)),
    (1101, 140, 46, (668, 568, 775, 769, 1070, 615, 468)),
    (1, 0, 7, (18, 14, 22, 21, 33, 16, 10)),
    (81, 50, 46, (195, 156, 235, 233, 348, 174, 118)),
    (1101, 200, 7, (316, 291, 342, 340, 415, 302, 266)),
)


def original():
    return {'identity': {'key': 'candidate:formula/spell/monk', 'revision': 'definition-r1'},
            'kind': 'player_expression', 'inputs': 'skill',
            'minimum': {'const': '1'}, 'maximum': {'const': '2'}}


class MonkFormulaCorrectionTests(unittest.TestCase):
    def test_backend_centers_and_source_spread_are_checked_separately(self):
        for level, skill, attack, averages in API_CASES:
            for name, expected in zip(NAMES, averages):
                with self.subTest(name=name, level=level, skill=skill, attack=attack):
                    power = BUILDER_BASE_POWERS[name]
                    formula, _ = correct(name, 'instant', original(), power)
                    env = {'level': level, 'attack_skill': skill, 'attack_value': attack, 'base_power': power}
                    center = evaluate(center_expression(), env)
                    self.assertEqual(center, expected)
                    low = math.trunc(evaluate(formula['minimum'], env))
                    high = math.trunc(evaluate(formula['maximum'], env))
                    self.assertLessEqual(low, expected)
                    self.assertGreaterEqual(high, expected)
                    self.assertLess(low, high)
                    self.assertEqual(formula['minimum']['args'][1], {'const': '0.9'})
                    self.assertEqual(formula['maximum']['args'][1], {'const': '1.1'})

    def test_replacement_preserves_identity_and_does_not_mutate_input(self):
        value = original()
        before = copy.deepcopy(value)
        fixed, notes = correct('Flurry of Blows', 'instant', value, 55)
        self.assertEqual(value, before)
        self.assertEqual(fixed['identity'], before['identity'])
        self.assertIsNot(fixed['identity'], value['identity'])
        self.assertTrue(notes)
        self.assertNotIn('harmony', str(fixed).lower())

    def test_each_repaired_formula_passes_schema_and_semantic_grid(self):
        fragment = {'$ref': SCHEMAS['spell.schema.json']['$id'] + '#/$defs/formula'}
        for name, power in BUILDER_BASE_POWERS.items():
            with self.subTest(name=name):
                formula, _ = correct(name, 'instant', original(), power)
                Draft202012Validator(fragment, registry=REGISTRY).validate(formula)
                errors = []
                check_formula(formula, power, name, errors)
                self.assertEqual(errors, [])

    def test_unqualified_records_fail_closed(self):
        for name, spell_type, value, power in (
            ('swift jab', 'instant', original(), 12),
            ('greater tiger clash', 'instant', original(), 44),
            ('devastating knockout', 'instant', original(), 62),
            ('tiger clash', 'instant', original(), 15),
            ('double jab', 'rune', original(), 40),
            ('double jab', 'instant', original(), 12),
            ('double jab', 'instant', original(), 40.0),
            ('double jab', 'instant', {**original(), 'inputs': 'level_magic'}, 40),
            ('double jab', 'instant', {}, 40),
        ):
            with self.subTest(name=name, spell_type=spell_type, power=power):
                self.assertIsNone(correct(name, spell_type, value, power))

    def test_runtime_inputs_and_positive_half_rounding_stay_live(self):
        formula, _ = correct('chained penance', 'instant', original(), 70)
        for skill, attack in ((0, 0), (0, 7), (50, 46), (140, 46), (200, 7)):
            env = {'level': 1, 'attack_skill': skill, 'attack_value': attack, 'base_power': 70}
            term = (70 / 1000) * skill * attack + 70 / 4
            self.assertEqual(evaluate(center_expression(), env), math.floor(term + 0.5))
        self.assertIn('attack_skill', str(formula))
        self.assertIn('attack_value', str(formula))
        self.assertNotIn("'const': '140'", str(formula))

    def test_three_source_baselines_keep_the_pre_harmony_operations_and_clamps(self):
        for name, qualification in SPENDER_SOURCES.items():
            with self.subTest(name=name):
                body = baseline_spender_formula(name, 'instant', qualification['power'], 'canary', qualification['error'])
                self.assertIsNotNone(body)
                formula = {'identity': original()['identity'], **body}
                fragment = {'$ref': SCHEMAS['spell.schema.json']['$id'] + '#/$defs/formula'}
                Draft202012Validator(fragment, registry=REGISTRY).validate(formula)
                errors = []
                check_formula(formula, qualification['power'], name, errors)
                self.assertEqual(errors, [])
                low = body['minimum']['args'][0] if name == 'tiger clash' else body['minimum']
                high = body['maximum']['args'][0] if name == 'tiger clash' else body['maximum']
                self.assertEqual(low['op'], 'sub')
                self.assertEqual(high['op'], 'add')
                env = {'level': 1, 'attack_skill': 0, 'attack_value': 0, 'base_power': qualification['power']}
                self.assertEqual([evaluate(body[k], env) for k in ('minimum', 'maximum')],
                                 [5, 10] if name == 'tiger clash' else [0, 0])
                self.assertNotIn('harmony', str(body).lower())

    def test_spender_fallback_rejects_unqualified_errors_sources_and_values(self):
        name = 'greater tiger clash'
        qualification = SPENDER_SOURCES[name]
        for source, error, power, kind in (
            ('crystal', qualification['error'], 44, 'instant'),
            ('canary', 'unknown helper error', 44, 'instant'),
            ('canary', qualification['error'], 45, 'instant'),
            ('canary', qualification['error'], 44, 'rune'),
            ('canary', qualification['error'], 44.0, 'instant'),
        ):
            self.assertIsNone(baseline_spender_formula(name, kind, power, source, error))
        self.assertIsNone(baseline_spender_formula('unknown spell', 'instant', 44, 'canary', qualification['error']))


if __name__ == '__main__':
    unittest.main()
