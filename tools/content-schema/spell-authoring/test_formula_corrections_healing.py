"""Regressions of source rounding and intentional S5 world-curve normalization."""
import copy
import unittest

from formula_corrections_healing import correct
from validate_spell import evaluate, level_base_damage_healing


def formula():
    level = {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    def bound(coefficient, offset):
        return {'op': 'add', 'args': [
            {'op': 'add', 'args': [copy.deepcopy(level),
                {'op': 'mul', 'args': [{'var': 'magic_level'}, {'const': coefficient}]}]},
            {'const': offset}]}
    return {'identity': {'key': 'candidate:test/uh', 'revision': 'test-r1'},
            'kind': 'player_expression', 'inputs': 'level_magic',
            'minimum': bound('7.3', '42'), 'maximum': bound('12.4', '90')}


class HealingCorrectionTests(unittest.TestCase):
    def test_fractional_maximum_keeps_source_ceil(self):
        original = formula()
        replacement, notes = correct('ultimate healing rune', 'rune', original, 250)
        env = {'level': 1000, 'magic_level': 13}
        self.assertAlmostEqual(evaluate(original['maximum'], env), 251.2 + level_base_damage_healing(1000))
        self.assertEqual([evaluate(replacement[b], env) for b in ('minimum', 'maximum')],
                         [136 + level_base_damage_healing(1000), 252 + level_base_damage_healing(1000)])
        self.assertTrue(any('ff7ede593c69d4c658b382c97443e8155926924a' in n for n in notes))

    def test_input_not_mutated_identity_preserved(self):
        original = formula(); before = copy.deepcopy(original)
        replacement, _ = correct('Ultimate Healing Rune', 'rune', original, 250)
        self.assertEqual(original, before)
        self.assertEqual(replacement['identity'], before['identity'])
        self.assertEqual(replacement['inputs'], before['inputs'])
        self.assertEqual(replacement['minimum']['args'][0], before['minimum'])
        self.assertEqual(replacement['maximum']['args'][0], before['maximum'])

    def test_idempotent_and_partial_rounding(self):
        original = formula(); original['minimum'] = {'op': 'floor', 'args': [original['minimum']]}
        replacement, _ = correct('ultimate healing rune', 'rune', original, 250)
        self.assertEqual(replacement['minimum'], original['minimum'])
        self.assertIsNone(correct('ultimate healing rune', 'rune', replacement, 250))

    def test_world_curve_retained_above_pre2022_scaling(self):
        replacement, _ = correct('ultimate healing rune', 'rune', formula(), 250)
        self.assertEqual(evaluate(replacement['minimum'], {'level': 2000, 'magic_level': 0}),
                         42 + level_base_damage_healing(2000))
        self.assertNotEqual(level_base_damage_healing(2000), 2000 / 5)

    def test_other_heals_conjuring_and_other_formula_kinds_unchanged(self):
        for name, carrier in [('intense healing rune', 'rune'), ("Nature's Embrace", 'instant'),
                              ('Spirit Mend', 'instant'), ('ultimate healing rune', 'instant')]:
            with self.subTest(name=name, carrier=carrier):
                self.assertIsNone(correct(name, carrier, formula(), 250))
        value = formula(); value['kind'] = 'fixed'
        self.assertIsNone(correct('ultimate healing rune', 'rune', value, 250))

    def test_restoration_scaled_legacy_level_uses_world_curve(self):
        original = formula()
        legacy = {'op': 'div', 'args': [
            {'op': 'mul', 'args': [{'var': 'level'}, {'const': '1.4'}]}, {'const': '5'}]}
        for bound in ('minimum', 'maximum'):
            original[bound]['args'][0]['args'][0] = copy.deepcopy(legacy)
        before = copy.deepcopy(original)
        replacement, notes = correct('Restoration', 'instant', original, 375)
        env = {'level': 1100, 'magic_level': 10}
        self.assertAlmostEqual(evaluate(original['minimum'], env) - evaluate(replacement['minimum'], env), 28)
        self.assertEqual(original, before)
        self.assertEqual(replacement['identity'], original['identity'])
        self.assertTrue(any('nested' in note for note in notes))
        self.assertIsNone(correct('Restoration', 'instant', replacement, 375))
        self.assertIsNone(correct('Restoration', 'rune', original, 375))

    def test_integral_bounds_not_changed_numerically(self):
        replacement, _ = correct('ultimate healing rune', 'rune', formula(), 250)
        for level in [1, 500, 1100, 2000]:
            env = {'level': level, 'magic_level': 10}
            self.assertEqual([evaluate(replacement[b], env) for b in ('minimum', 'maximum')],
                             [evaluate(formula()[b], env) for b in ('minimum', 'maximum')])


if __name__ == '__main__':
    unittest.main()
