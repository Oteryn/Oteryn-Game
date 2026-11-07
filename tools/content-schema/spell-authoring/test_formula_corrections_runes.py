"""Rune correction behavior and genuine neutral calculator golden values."""
import copy
import math
import unittest

import formula_corrections_runes as corrections
from validate_spell import check_formula, evaluate, SCHEMAS, REGISTRY, Draft202012Validator


def original():
    return {'identity': {'key': 'candidate:formula/example', 'revision': 'offline-test'},
            'kind': 'player_expression', 'inputs': 'level_magic',
            'minimum': {'const': '0'}, 'maximum': {'const': '1'}}


# Genuine unchanged pinned backend: reference/cases.json paladin-042, L200/ML50.
# Values are retained outputs, not expectations computed by this correction implementation.
GOLDEN = {
    'avalanche rune': (107, 197, 152),
    'explosion rune': (130, 220, 175),
    'fireball rune': (141, 208, 175),
    'great fireball rune': (107, 197, 152),
    'heavy magic missile rune': (85, 130, 107),
    'holy missile rune': (141, 253, 197),
    'icicle rune': (141, 208, 175),
    'light magic missile rune': (62, 85, 73),
    'stalagmite rune': (85, 130, 107),
    'stone shower rune': (107, 197, 152),
    'sudden death rune': (298, 456, 377),
    'thunderstorm rune': (107, 197, 152),
}


class RuneCorrectionTests(unittest.TestCase):
    def test_all_twelve_real_backend_goldens_bounds_and_average_separately(self):
        self.assertEqual(set(GOLDEN), set(corrections.MODELS))
        for name, (minimum, maximum, average) in GOLDEN.items():
            with self.subTest(name=name):
                power = corrections.MODELS[name][0]
                formula, _ = corrections.correct(name, 'rune', original(), power)
                env = {'level': 200, 'magic_level': 50, 'base_power': power}
                self.assertEqual([evaluate(formula[k], env) for k in ('minimum', 'maximum')], [minimum, maximum])
                self.assertEqual(evaluate(corrections.average_expression(name), env), average)

    def test_preserves_identity_and_does_not_mutate_input(self):
        source = original()
        before = copy.deepcopy(source)
        replacement, notes = corrections.correct('Holy Missile Rune', 'rune', source, 70)
        self.assertEqual(source, before)
        self.assertEqual(replacement['identity'], source['identity'])
        replacement['identity']['revision'] = 'changed'
        self.assertEqual(source, before)
        self.assertIn(corrections.CALCULATOR_COMMIT, ' '.join(notes))

    def test_other_carrier_or_unknown_name_is_not_rewritten(self):
        self.assertIsNone(corrections.correct('avalanche rune', 'instant', original(), 50))
        self.assertIsNone(corrections.correct('intense healing rune', 'rune', original(), 50))

    def test_wrong_or_boolean_power_fails_closed(self):
        for power in (45, True, None, 0):
            with self.subTest(power=power), self.assertRaises(ValueError):
                corrections.correct('stone shower rune', 'rune', original(), power)

    def test_wrong_formula_kind_or_input_set_fails_closed(self):
        for key, value in [('kind', 'linear_level'), ('inputs', 'skill'), ('identity', None)]:
            formula = original()
            formula[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                corrections.correct('avalanche rune', 'rune', formula, 50)

    def test_level_flat_is_outside_variation(self):
        formula, _ = corrections.correct('avalanche rune', 'rune', original(), 50)
        for level in (1, 80, 499, 500, 501, 699, 700, 701, 999, 1000, 1001, 1100, 1101):
            env = {'level': level, 'magic_level': 50, 'base_power': 50}
            lo, hi = [evaluate(formula[k], env) for k in ('minimum', 'maximum')]
            self.assertEqual(hi - lo, 90)
            self.assertEqual(lo, math.floor(lo))
            self.assertEqual(hi, math.floor(hi))

    def test_average_is_not_silently_inferred_from_rounded_endpoints(self):
        formula, _ = corrections.correct('fireball rune', 'rune', original(), 60)
        env = {'level': 200, 'magic_level': 50, 'base_power': 60}
        lo, hi = [evaluate(formula[k], env) for k in ('minimum', 'maximum')]
        avg = evaluate(corrections.average_expression('fireball rune'), env)
        self.assertEqual(avg, 175)
        self.assertNotEqual((lo + hi) / 2, avg)

    def test_power_and_engine_model_conflicts_stay_explicit(self):
        for name in ('stone shower rune', 'thunderstorm rune'):
            _, notes = corrections.correct(name, 'rune', original(), 50)
            evidence = ' '.join(notes)
            self.assertIn('Tibiopedia states 45', evidence)
            self.assertIn('Crystal calculateMagicSpellDamage', evidence)
            self.assertIn('Canary rune callbacks', evidence)

    def test_every_corrected_formula_passes_schema_and_semantic_grid(self):
        fragment = {'$ref': SCHEMAS['spell.schema.json']['$id'] + '#/$defs/formula'}
        validator = Draft202012Validator(fragment, registry=REGISTRY)
        for name, (power, _, _) in corrections.MODELS.items():
            with self.subTest(name=name):
                formula, _ = corrections.correct(name, 'rune', original(), power)
                validator.validate(formula)
                errors = []
                check_formula(formula, power, name, errors)
                self.assertFalse(errors, errors)


if __name__ == '__main__':
    unittest.main()
