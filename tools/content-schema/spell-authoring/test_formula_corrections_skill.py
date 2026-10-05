"""Concrete regression vectors for the bounded Knight/Paladin AST repair."""
import copy
import json
import math
import unittest

from formula_corrections_skill import MODELS, UNRESOLVED, correct
from validate_spell import check_formula, evaluate, REGISTRY, SCHEMAS, Draft202012Validator

# Captured genuine backend vectors: knight skill80/attack14; paladin skill80/ML50.
VECTORS = {
    'annihilation': ((116, 226), (299, 409)),
    'berserk': ((44, 77), (227, 260)),
    'brutal strike': ((40, 67), (223, 250)),
    'fierce berserk': ((92, 160), (275, 343)),
    'groundshaker': ((32, 56), (215, 239)),
    'whirlwind throw': ((32, 56), (215, 239)),
    'divine barrage': ((270, 315), (453, 498)),
    'divine caldera': ((292, 382), (475, 565)),
    'ethereal barrage': ((175, 203), (358, 386)),
    'ethereal spear': ((57, 86), (240, 269)),
    'strong ethereal spear': ((88, 132), (271, 315)),
    'divine missile': ((101, 168), (284, 351)),
}

# All39 genuine Strong Ethereal vectors, frozen189-case backend reference.
STRONG_REFERENCE = [
    ('paladin-035', 1, 80, 88, 132),
    ('paladin-036', 8, 80, 89, 133),
    ('paladin-037', 80, 80, 104, 148),
    ('paladin-038', 99, 80, 107, 151),
    ('paladin-039', 100, 80, 108, 152),
    ('paladin-040', 101, 80, 108, 152),
    ('paladin-041', 199, 80, 127, 171),
    ('paladin-042', 200, 80, 128, 172),
    ('paladin-043', 201, 80, 128, 172),
    ('paladin-044', 299, 80, 147, 191),
    ('paladin-045', 300, 80, 148, 192),
    ('paladin-046', 301, 80, 148, 192),
    ('paladin-047', 399, 80, 167, 211),
    ('paladin-048', 400, 80, 168, 212),
    ('paladin-049', 401, 80, 168, 212),
    ('paladin-050', 499, 80, 187, 231),
    ('paladin-051', 500, 80, 188, 232),
    ('paladin-052', 501, 80, 188, 232),
    ('paladin-053', 699, 80, 221, 265),
    ('paladin-054', 700, 80, 221, 265),
    ('paladin-055', 701, 80, 221, 265),
    ('paladin-056', 999, 80, 271, 315),
    ('paladin-057', 1000, 80, 271, 315),
    ('paladin-058', 1001, 80, 271, 315),
    ('paladin-059', 1099, 80, 287, 331),
    ('paladin-060', 1100, 80, 288, 332),
    ('paladin-061', 1101, 80, 288, 332),
    ('paladin-062', 200, 80, 128, 172),
    ('paladin-063', 200, 80, 128, 172),
    ('paladin-064', 200, 80, 128, 172),
    ('paladin-065', 200, 10, 55, 63),
    ('paladin-066', 200, 140, 190, 265),
    ('paladin-067', 200, 80, 128, 172),
    ('paladin-068', 200, 80, 128, 172),
    ('paladin-178', 200, 80, 128, 172),
    ('paladin-179', 200, 80, 128, 172),
    ('paladin-180', 200, 80, 128, 172),
    ('paladin-181', 200, 80, 128, 172),
    ('paladin-182', 200, 80, 128, 172),
]


def original(name):
    return {'identity': {'key': 'candidate:formula/test/' + name.replace(' ', '_'), 'revision': 'test-r1'},
            'kind': 'player_expression', 'inputs': 'level_magic' if MODELS[name][3] == 'magic' else 'skill',
            'minimum': {'const': '1'}, 'maximum': {'const': '2'}}


def bounds(formula, model, level=1, skill=80, attack=14, magic=50):
    env = {'level': level, 'attack_skill': skill, 'attack_value': attack,
           'magic_level': magic, 'base_power': model[0]}
    return tuple(math.trunc(evaluate(formula[key], env)) for key in ('minimum', 'maximum'))


class SkillFormulaRepairTests(unittest.TestCase):
    def test_genuine_low_and_high_level_bounds(self):
        for name, expected in VECTORS.items():
            with self.subTest(name=name):
                formula, notes = correct(name, 'instant', original(name), MODELS[name][0])
                self.assertEqual(bounds(formula, MODELS[name], 1), expected[0])
                self.assertEqual(bounds(formula, MODELS[name], 1000), expected[1])
                self.assertTrue(notes)

    def test_level_flat_is_added_outside_variation(self):
        for name in MODELS:
            formula, _ = correct(name, 'instant', original(name), MODELS[name][0])
            low = bounds(formula, MODELS[name], 1)
            high = bounds(formula, MODELS[name], 1000)
            self.assertEqual(tuple(b - a for a, b in zip(low, high)), (183, 183), name)

    def test_identity_and_source_formula_are_preserved(self):
        source = original('berserk')
        saved = copy.deepcopy(source)
        repaired, _ = correct('Berserk', 'instant', source, 44)
        self.assertEqual(source, saved)
        self.assertEqual(repaired['identity'], source['identity'])
        repaired['identity']['revision'] = 'changed'
        self.assertEqual(source, saved)

    def test_distance_model_does_not_read_weapon_attack(self):
        for name in ('ethereal spear', 'ethereal barrage', 'strong ethereal spear'):
            formula, notes = correct(name, 'instant', original(name), MODELS[name][0])
            self.assertEqual(bounds(formula, MODELS[name], attack=1), bounds(formula, MODELS[name], attack=200))
            self.assertNotIn('attack_value', json.dumps(formula))
            self.assertTrue(any('conflict' in note for note in notes))

    def test_weapon_model_uses_both_skill_and_attack(self):
        formula, _ = correct('berserk', 'instant', original('berserk'), 44)
        lower = bounds(formula, MODELS['berserk'], attack=14)
        higher = bounds(formula, MODELS['berserk'], attack=50)
        self.assertGreater(higher[0], lower[0])
        self.assertGreater(higher[1], lower[1])
        self.assertNotIn('attack_factor', json.dumps(formula))

    def test_schema_and_semantic_bounds(self):
        schema = {'$ref': SCHEMAS['spell.schema.json']['$id'] + '#/$defs/formula'}
        for name, model in MODELS.items():
            with self.subTest(name=name):
                formula, _ = correct(name, 'instant', original(name), model[0])
                Draft202012Validator(schema, registry=REGISTRY).validate(formula)
                errors = []
                check_formula(formula, model[0], name, errors)
                self.assertEqual(errors, [])

    def test_half_up_rounding_keeps_integer_magnitude(self):
        # P44, skill0 => powered11, lower half8. Half-up must return8,
        # upper14; no truncation of an old factor*whole-level expression.
        formula, _ = correct('berserk', 'instant', original('berserk'), 44)
        self.assertEqual(bounds(formula, MODELS['berserk'], skill=0, attack=0), (8, 14))

    def test_unknown_power_rune_stage_and_input_family_fail_closed(self):
        source = original('berserk')
        for bp in (None, 0, -1, True, 45, float('nan'), float('inf'), '44'):
            self.assertIsNone(correct('berserk', 'instant', source, bp))
        self.assertIsNone(correct('berserk', 'rune', source, 44))
        self.assertIsNone(correct('unknown spell', 'instant', source, 44))
        self.assertIsNone(correct("Executioner's Throw (Stage 1)", 'instant', source, 44))
        self.assertIsNone(correct('berserk', 'instant', {**source, 'stage': 1}, 44))
        self.assertIsNone(correct('berserk', 'instant', {**source, 'inputs': 'level_magic'}, 44))

    def test_strong_ethereal_uses_user_selected_calculator_power(self):
        self.assertNotIn('strong ethereal spear', UNRESOLVED)
        self.assertEqual(MODELS['strong ethereal spear'][0], 25)
        self.assertIsNone(correct('strong ethereal spear', 'instant', original('strong ethereal spear'), 38))
        source = original('strong ethereal spear')
        formula, notes = correct('strong ethereal spear', 'instant', source, 25)
        self.assertEqual(formula['identity'], source['identity'])
        self.assertIn('base_power', json.dumps(formula))
        self.assertTrue(any('kalkulator ma rację' in note and 'BP38' in note and 'BP25' in note
                            for note in notes))
        self.assertFalse(any('UNRESOLVED' in note for note in notes))

    def test_all_39_genuine_strong_ethereal_bounds(self):
        formula, _ = correct('strong ethereal spear', 'instant', original('strong ethereal spear'), 25)
        self.assertEqual(len(STRONG_REFERENCE), 39)
        for case_id, level, skill, minimum, maximum in STRONG_REFERENCE:
            with self.subTest(case_id=case_id):
                self.assertEqual(bounds(formula, MODELS['strong ethereal spear'], level, skill),
                                 (minimum, maximum))


if __name__ == '__main__':
    unittest.main()
