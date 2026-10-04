import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import project_monk_formula_candidates as project
import source_monk_formula_library as original
import validate_spell

REPO = Path(__file__).resolve().parents[3]
CACHE = Path('/workspace/spell-source-closure/runtime1534-formula-read')


def rust_expression(tree):
    if 'const' in tree:
        return 'Expression::Const(' + tree['const'] + '_f64)'
    if 'var' in tree:
        return 'Expression::Var(Input::' + {'level': 'Level', 'attack_skill': 'AttackSkill', 'attack_value': 'AttackValue'}[tree['var']] + ')'
    args = [rust_expression(arg) for arg in tree['args']]
    if 'fn' in tree:
        if tree['fn'] != 'level_base_damage_healing':
            raise ValueError('unsupported consumed function')
        return 'Expression::LevelBaseDamageHealing(Box::new(' + args[0] + '))'
    name = tree['op']
    if name == 'abs':
        return 'Expression::Unary(Unary::Abs,Box::new(' + args[0] + '))'
    if name in ('min', 'max'):
        return 'Expression::Extremum(Extremum::' + name.title() + ',vec![' + ','.join(args) + '])'
    return 'Expression::Binary(Binary::' + name.title() + ',Box::new(' + args[0] + '),Box::new(' + args[1] + '))'


class CanonicalMonkProjectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = project.build(REPO, '/workspace/spell-sources', CACHE)
        cls.proof = cls.packet['projection-qualification.json']

    def bundle(self, record):
        row = 'canary-main-current/' + record['candidate_key'].rsplit('/', 1)[-1] + '/'
        return self.packet[row + 'spell.json'], self.packet[row + 'dependencies.json'], self.packet[row + 'catalog.json'], self.packet[row + 'source-header.json'], self.packet[row + 'receipt.json']

    def test_all_six_standard_bundles_validate_with_existing_shared_schema(self):
        self.assertEqual(6, len(self.proof['records']))
        for record in self.proof['records']:
            spell, deps, catalog, header, receipt = self.bundle(record)
            self.assertEqual([], validate_spell.validate(spell, deps, catalog))
            self.assertTrue(project.reader_adapter.validate_reader_shape(REPO, spell, deps))
            self.assertEqual({'abilities': 1, 'effects': 1, 'formulas': 1}, receipt['dependencies'])
            self.assertEqual('CANDIDATE_SCHEMA_VALID', receipt['status'])
            self.assertEqual('builder', spell['spell']['harmony_role'])
            self.assertNotIn('vocation_display_flags', spell['spell']['requirements'])
            self.assertEqual(header['spell']['requirements']['vocation_display_flags'], record['source_vocation_display_flags_retained'])
            self.assertIn('allow_on_self', spell['spell']['targeting'])
            self.assertTrue(spell['spell']['targeting']['check_floor'])
            self.assertFalse(receipt['native_execution_qualified'])
            self.assertTrue(receipt['remaining_mechanics'])
            for field in ('presentation', 'costs'):
                for name, value in header['spell'][field].items():
                    self.assertEqual(value, spell['spell'][field][name])

    def test_s5_and_s16_normalizations_and_source_retention_are_explicit(self):
        self.assertEqual('S5', self.proof['policy_proof']['decision'])
        self.assertEqual('S16', self.proof['policy_proof']['additional_decision'])
        for record in self.proof['records']:
            spell, deps, _, header, _ = self.bundle(record)
            self.assertFalse(spell['spell']['requirements']['learning_required'])
            self.assertEqual(record['source_learning_required'], spell['spell']['requirements']['wheel_unlock'])
            serialized = json.dumps(deps['formulas'])
            self.assertIn('level_base_damage_healing', serialized)
            self.assertNotIn('"fn": "flat_damage_healing"', serialized)
            self.assertTrue(record['canonical_normalization_used'])
            self.assertFalse(record['source_numeric_equivalence'])
            self.assertFalse(record['input_provider_equivalence'])
            self.assertEqual(1, record['source_combat_parameters']['COMBAT_PARAM_USECHARGES'])
            self.assertEqual(header['spell']['requirements'].get('learning_required', False), record['source_learning_required'])

    def test_pinned_actual_rust_formula_evaluates_all_six_consumed_trees(self):
        rows = []
        expected = []
        for record in self.proof['records']:
            _, deps, _, _, _ = self.bundle(record)
            formula = deps['formulas'][0]
            rows.append('Formula {minimum:' + rust_expression(formula['minimum']) + ',maximum:' + rust_expression(formula['maximum']) + '}')
            for level in (1, 100, 499, 500, 1100, 2500, 1000000):
                env = {'level': level, 'attack_skill': 130, 'attack_value': 100}
                expected.append(tuple(int(validate_spell.evaluate(formula[bound], env)) for bound in ('minimum', 'maximum')))
        with tempfile.TemporaryDirectory(prefix='canonical-monk-native-formula-') as temporary:
            path = Path(temporary)
            source = '#[path=' + json.dumps(str(CACHE / 'formula.rs')) + '] mod formula;\n'
            source += 'use formula::{Formula,FormulaInputs,Expression,Input,Unary,Binary,Extremum};\n'
            source += 'fn main(){let formulas=vec![' + ','.join(rows) + ']; for f in formulas {for level in [1,100,499,500,1100,2500,1000000] {'
            source += 'let inputs=FormulaInputs {level,magic_level:0,base_power:None,attack_skill:130,attack_value:100,attack_factor:1.0,shielding_skill:0,shield_defense:None};'
            source += 'let (low,high)=f.bounds(&inputs).unwrap(); println!("{} {}",low,high);}}}\n'
            (path / 'native.rs').write_text(source)
            subprocess.run(['rustc', '--edition=2021', '-Awarnings', str(path / 'native.rs'), '-o', str(path / 'native')], check=True, capture_output=True)
            output = subprocess.check_output([str(path / 'native')], text=True)
        actual = [tuple(map(int, line.split())) for line in output.splitlines()]
        self.assertEqual(expected, actual)

    def test_source_cpp_qualified_helper_difference_is_intentional_not_hidden(self):
        helper = original.SourceTierEvaluator(original.load_program(REPO))
        source_flat = helper.evaluate(1100)
        canonical_level = validate_spell.level_base_damage_healing(1100)
        self.assertEqual(284, source_flat)  # R50 exact C++ oracle qualification.
        self.assertNotEqual(source_flat, canonical_level)
        for record in self.proof['records']:
            _, deps, _, _, _ = self.bundle(record)
            formula = deps['formulas'][0]
            env = {'level': 1100, 'attack_skill': 130, 'attack_value': 100}
            captured = original.rows(REPO, original.CAPTURE_PATH, original.CAPTURE_SHA)[record['registration_key']]
            raw = captured['source_callback_facts']['combats'][0]['callbacks'][0]['formula']
            source_record = {'expressions': {key: original.link_expression(raw[key]) for key in ('minimum', 'maximum')}}
            source_pair = original.evaluate_formula(source_record, env, helper)
            target_pair = tuple(validate_spell.evaluate(formula[key], env) for key in ('minimum', 'maximum'))
            self.assertNotEqual(source_pair, target_pair)

    def test_unknown_helper_and_unsupported_reader_inputs_fail_closed(self):
        for value in ({'fn': 'unknown', 'args': [{'var': 'level'}]}, {'fn': 'flat_damage_healing', 'args': [{'var': 'attack_skill'}]}):
            with self.assertRaises(ValueError):
                project.normalize(value)
        with tempfile.TemporaryDirectory(prefix='consumer-tamper-') as temporary:
            for file in project.RUNTIME_FILES:
                (Path(temporary) / file).write_bytes((CACHE / file).read_bytes())
            changed = Path(temporary) / 'formula.rs'
            changed.write_bytes(changed.read_bytes() + b'\n')
            with self.assertRaises(ValueError):
                project.consumer_proof(temporary)

    def test_display_only_and_missing_required_targeting_are_rejected_by_reader(self):
        spell, deps, _, header, _ = self.bundle(self.proof['records'][0])
        for mutation in ('display_flags', 'allow_on_self', 'check_floor'):
            changed = copy.deepcopy(spell)
            if mutation == 'display_flags':
                changed['spell']['requirements']['vocation_display_flags'] = header['spell']['requirements']['vocation_display_flags']
            else:
                del changed['spell']['targeting'][mutation]
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                project.reader_adapter.validate_reader_shape(REPO, changed, deps)


if __name__ == '__main__':
    unittest.main()
