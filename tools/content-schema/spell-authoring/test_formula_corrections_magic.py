"""Independent pinned outputs detect coefficient and rounding-stage regressions."""
from copy import deepcopy
import math
import unittest

from formula_corrections_magic import PARAMETERS, AVERAGE_ONLY, correct, nominal_center
from validate_spell import evaluate

# Actual raw bounds from the unmodified independent backend. Six probes cover
# missing offset, nonzero F, magic-level extremes and the old >1100 level defect.
# Columns: (level, magic level) = (1,50), (1000,50), (1101,50),
#                              (200,0), (200,13), (200,100).
PROBES = [(1, 50), (1000, 50), (1101, 50), (200, 0), (200, 13), (200, 100)]
GOLDEN = {
    'Energy Strike': [(78,123),(261,306),(278,323),(48,53),(66,82),(188,273)],
    'Death Strike': [(78,123),(261,306),(278,323),(48,53),(66,82),(188,273)],
    'Flame Strike': [(78,123),(261,306),(278,323),(48,53),(66,82),(188,273)],
    'Ice Strike': [(78,123),(261,306),(278,323),(48,53),(66,82),(188,273)],
    'Terra Strike': [(78,123),(261,306),(278,323),(48,53),(66,82),(188,273)],
    'Eternal Winter': [(303,596),(486,779),(503,796),(73,106),(143,244),(613,1166)],
    'Forked Glacier': [(198,206),(381,389),(398,406),(62,63),(107,110),(414,430)],
    'Forked Thorns': [(213,222),(396,405),(413,422),(63,64),(113,116),(443,460)],
    'Great Death Beam': [(292,405),(475,588),(492,605),(72,85),(140,178),(592,805)],
    'Great Fire Wave': [(168,281),(351,464),(368,481),(58,71),(97,136),(358,571)],
    "Hell's Core": [(450,675),(633,858),(650,875),(90,115),(194,271),(890,1315)],
    'Terra Wave': [(180,360),(363,543),(380,560),(60,80),(101,163),(380,720)],
    'Wrath of Nature': [(315,472),(498,655),(515,672),(75,92),(147,201),(635,932)],
}


def original(key='candidate:formula/spell/great_death_beam/combat-1'):
    return {'identity': {'key': key, 'revision': 'regression-test'},
            'kind': 'player_expression', 'inputs': 'level_magic',
            'minimum': {'const': '1'}, 'maximum': {'const': '2'}}


class MagicFormulaCorrectionTests(unittest.TestCase):
    def test_all_independent_bounds_with_runtime_truncation(self):
        self.assertEqual(set(PARAMETERS), set(GOLDEN))
        for name, probes in GOLDEN.items():
            power = PARAMETERS[name][0]
            formula, notes = correct(name, 'instant', original(), power)
            self.assertTrue(notes)
            for (level, ml), expected in zip(PROBES, probes):
                with self.subTest(spell=name, level=level, magic_level=ml):
                    env = {'level': level, 'magic_level': ml, 'base_power': power}
                    actual = tuple(math.trunc(evaluate(formula[b], env))
                                   for b in ('minimum', 'maximum'))
                    self.assertEqual(actual, expected)

    def test_identity_and_input_are_preserved_without_mutation(self):
        formula = original()
        before = deepcopy(formula)
        replacement, _ = correct('Energy Strike', 'instant', formula, 45)
        self.assertEqual(formula, before)
        self.assertEqual(replacement['identity'], before['identity'])
        self.assertEqual(replacement['inputs'], before['inputs'])
        self.assertIsNot(replacement['identity'], formula['identity'])

    def test_actual_lowercase_converter_names_dispatch_all_26_corrections(self):
        powers = {name:spec[0]for name,spec in PARAMETERS.items()} | AVERAGE_ONLY
        self.assertEqual(len(powers),26)
        for name,power in powers.items():
            with self.subTest(spell=name):
                title_case = correct(name,'instant',original(),power)
                converter_case = correct(name.casefold(),'instant',original(),power)
                self.assertIsNotNone(converter_case)
                self.assertEqual(converter_case,title_case)
                self.assertEqual(converter_case[0]['identity'],original()['identity'])
        for guessed_alias in ('great-death-beam','hells core','ice_strike',
                              'Great Death Beam (Central)'):
            self.assertIsNone(correct(guessed_alias,'instant',original(),155))

    def test_fixed_power_is_a_guard_not_an_implicit_rewrite(self):
        for invalid in (None, 46, 45.0, True):
            with self.subTest(base_power=invalid):
                self.assertIsNone(correct('Energy Strike', 'instant', original(), invalid))

    def test_runes_unknown_and_non_player_formulas_are_unchanged(self):
        self.assertIsNone(correct('Energy Strike', 'rune', original(), 45))
        self.assertIsNone(correct('Unknown Spell', 'instant', original(), 45))
        self.assertIsNone(correct('Energy Strike', 'instant', {'kind': 'constant'}, 45))
        self.assertIsNone(correct('Energy Strike', 'instant',
                                 dict(original(), inputs='skill'), 45))

    def test_flank_formula_is_not_replaced_by_central_beam(self):
        self.assertIsNone(correct('Great Death Beam', 'instant',
                                 original('candidate:formula/spell/great_death_beam/combat-2'),155))

    def test_execution_limitations_are_kept_explicit(self):
        for name, expected in [('Forked Glacier', 'chain attenuation'),
                               ('Great Death Beam', 'side beams')]:
            _, notes = correct(name, 'instant', original(), PARAMETERS[name][0])
            self.assertIn(expected, ' '.join(notes))

    def test_whole_level_flat_is_never_scaled_by_range(self):
        f, _ = correct('Energy Strike', 'instant', original(), 45)
        low_1 = evaluate(f['minimum'], {'level':1,'magic_level':0,'base_power':45})
        low_1000 = evaluate(f['minimum'], {'level':1000,'magic_level':0,'base_power':45})
        world = {'fn':'level_base_damage_healing','args':[{'var':'level'}]}
        self.assertEqual(low_1000-low_1,
                         evaluate(world,{'level':1000})-evaluate(world,{'level':1}))

    def test_avg_only_centers_match_independent_values(self):
        # Genuine raw averages at level 200, ML13; bounds are unavailable.
        expected = {'Fire Wave':70,'Ice Wave':66,'Rage of the Skies':194,
                    'Physical Strike':78,'Lightning':124,'Strong Flame Strike':136,
                    'Strong Energy Strike':136,'Strong Ice Strike':128,
                    'Strong Terra Strike':128,'Ultimate Flame Strike':201,
                    'Ultimate Energy Strike':201,'Ultimate Ice Strike':190,
                    'Ultimate Terra Strike':190}
        self.assertEqual(set(expected),set(AVERAGE_ONLY))
        for name, average in expected.items():
            with self.subTest(spell=name):
                self.assertEqual(evaluate(nominal_center(),
                    {'level':200,'magic_level':13,'base_power':AVERAGE_ONLY[name]}), average)

    def test_avg_only_retains_source_width_and_is_not_deterministic(self):
        # Crystal Fire Wave's explicit magic bounds: 1.25 ML+4, 2 ML+12.
        f = original('candidate:formula/spell/fire_wave/combat-1')
        flat = {'fn':'level_base_damage_healing','args':[{'var':'level'}]}
        def source_bound(coefficient, offset):
            return {'op':'add','args':[flat,{'op':'add','args':[
                {'op':'mul','args':[{'var':'magic_level'},{'const':coefficient}]},
                {'const':offset}]}]}
        f['minimum']=source_bound('1.25','4')
        f['maximum']=source_bound('2','12')
        replacement, notes=correct('Fire Wave','instant',f,40)
        env={'level':200,'magic_level':50,'base_power':40}
        bounds=[evaluate(replacement[b],env)for b in('minimum','maximum')]
        self.assertEqual(bounds,[107,152])
        center=evaluate(nominal_center(),env)
        self.assertEqual(center,130)
        self.assertLess(bounds[0],center)
        self.assertGreater(bounds[1],center)
        self.assertNotEqual(sum(bounds)/2,center)
        self.assertIn('publishes no min/max',' '.join(notes))
        repeated,_=correct('Fire Wave','instant',replacement,40)
        self.assertEqual(repeated,replacement)

    def test_lightning_retains_source_interval_and_limits_chain_claim(self):
        # Crystal calculateMagicSpellDamage source model: sqrt(0.4 BP) ML
        # plus BP/6, ranged by .88/1.12 before negative damage sign.
        # Correction retains its magic-only width, never supplies oracle bounds.
        f=original('candidate:formula/spell/lightning/combat-1')
        def op(name,*args):return {'op':name,'args':list(args)}
        def const(value):return {'const':str(value)}
        flat={'fn':'level_base_damage_healing','args':[{'var':'level'}]}
        power={'var':'base_power'}
        source_magic=op('add',op('mul',{'var':'magic_level'},
                       op('sqrt',op('mul',power,const('0.4')))),op('div',power,const(6)))
        f['minimum']=op('floor',op('mul',op('add',flat,source_magic),const('0.88')))
        f['maximum']=op('floor',op('mul',op('add',flat,source_magic),const('1.12')))
        repaired,notes=correct('lightning','instant',f,110)
        env={'level':200,'magic_level':13,'base_power':110}
        bounds=[evaluate(repaired[b],env)for b in('minimum','maximum')]
        self.assertEqual(bounds,[112,137])
        self.assertEqual(evaluate(nominal_center(),env),124)
        self.assertEqual(repaired['identity'],f['identity'])
        self.assertEqual(bounds[1]-bounds[0],25)
        self.assertIn('per-target base chain component',' '.join(notes))
        self.assertIn('target order',' '.join(notes))
        self.assertIsNone(correct('lightning','instant',f,109))

    def test_avg_only_rejects_unqualified_width_inputs(self):
        f=original();f['maximum']={'var':'attack_skill'}
        self.assertIsNone(correct('Fire Wave','instant',f,40))

    def test_average_is_not_assumed_to_be_midpoint(self):
        # Range BP45/buckets20 at ML13: raw bounds 66,82, raw mean 74.
        # At other inputs independent discrete rounding need not preserve symmetry.
        # API carries bounds only; no invented average/mean field is introduced.
        f, _ = correct('Energy Strike', 'instant', original(),45)
        self.assertNotIn('average', f)
        self.assertNotIn('mean', f)


if __name__ == '__main__':
    unittest.main()
