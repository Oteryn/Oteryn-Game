import copy
import json
import pathlib
import subprocess
import unittest

import source_restore_balance_candidate as adapter
import validate_spell

REPO = pathlib.Path(__file__).resolve().parents[3]
SOURCE_ROOT = pathlib.Path('/workspace/spell-sources')


class RestoreBalanceQualificationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.values, cls.fact = adapter.build(REPO, SOURCE_ROOT)
        cls.source = adapter.base.source_file(SOURCE_ROOT / 'crystal', adapter.REVISION, adapter.SOURCE_PATH)

    def test_existing_schemas_and_semantic_validation_pass(self):
        self.assertEqual([], validate_spell.validate(self.values['spell.json'], self.values['dependencies.json'], self.values['catalog.json']))
        receipt = self.values['receipt.json']
        validate_spell.Draft202012Validator(adapter.base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
        self.assertEqual('CANDIDATE_SCHEMA_VALID', receipt['status'])
        self.assertFalse(receipt['native_execution_qualified'])
        self.assertTrue(receipt['remaining_mechanics'])

    def test_chronology_and_target_presentation_are_distinct(self):
        effects = self.values['dependencies.json']['effects']
        presentation = effects[0]['presentation']
        self.assertEqual('before_combat', presentation['caster_effect_timing'])
        self.assertEqual('crystal.appearance:effect/magic_blue', presentation['caster_effect_asset_binding'])
        self.assertEqual('crystal.appearance:effect/magic_blue', presentation['impact_asset_binding'])
        self.assertEqual('remove_condition', effects[1]['operation'])
        self.assertEqual('paralyze', effects[1]['removed_condition'])
        self.assertNotIn('presentation', effects[1])
        modified = copy.deepcopy(self.values['dependencies.json'])
        del modified['effects'][0]['presentation']['caster_effect_timing']
        self.assertTrue(validate_spell.validate(self.values['spell.json'], modified, self.values['catalog.json']))

    def test_source_literal_formula_arithmetic_and_no_helper_override(self):
        formula = self.values['dependencies.json']['formulas'][0]
        for level, magic in [(1, 0), (100, 50), (1500, 130)]:
            env = {'level': level, 'magic_level': magic}
            self.assertAlmostEqual(level * 0.3 + magic * 7 + 3, validate_spell.evaluate(formula['minimum'], env))
            self.assertAlmostEqual(level * 0.3 + magic * 11 + 5, validate_spell.evaluate(formula['maximum'], env))
        self.assertNotIn('fn', json.dumps(formula))
        self.assertEqual(425, self.values['spell.json']['spell']['base_power'])
        self.assertFalse(self.values['source-qualification-proof.json']['base_power_used_by_source_formula'])

    def test_changed_source_and_wrong_registration_fail_closed(self):
        with self.assertRaisesRegex(ValueError, 'source/capture identity'):
            adapter.qualify(self.source.replace(b'magicLevel * 7', b'magicLevel * 8'), self.fact)
        modified = copy.deepcopy(self.fact); modified['registration_key'] += '#2'
        with self.assertRaisesRegex(ValueError, 'source/capture identity'):
            adapter.qualify(self.source, modified)

    def test_unrepresented_capture_operations_and_formula_substitutions_refused(self):
        for mutation in ['cast', 'combat', 'formula', 'base_power']:
            changed = copy.deepcopy(self.fact)
            raw = changed['source_callback_facts']
            if mutation == 'cast':
                raw['cast']['body'] += ' creature:addItem(1)'
            elif mutation == 'combat':
                raw['combats'][0]['parameters']['COMBAT_PARAM_USECHARGES'] = True
            elif mutation == 'base_power':
                raw['registrar']['basePower'] = 999
            else:
                raw['combats'][0]['callbacks'][0]['formula']['minimum']['args'][1]['const'] = '99'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                adapter.qualify(self.source, changed)

    def test_complete_actual_bounded_wrapper_cohort_is_accounted(self):
        proof = self.values['source-qualification-proof.json']
        self.assertEqual([adapter.REGISTRATION], proof['selected_r44_registrations'])
        self.assertEqual(['canary-main-current/data/scripts/spells/healing/heal_friend.lua#1'], proof['prior_r42_overlay_excluded'])
        self.assertEqual(2, len(proof['exact_before_combat_blocked_source_cohort']))
        self.assertEqual(['player', 'level', 'magicLevel', 'basePower'], proof['source_formula_parameters'])

    def test_header_preserved_and_candidate_revision_is_additive(self):
        header = self.values['source-header.json']['spell']
        spell = self.values['spell.json']['spell']
        self.assertEqual('player_name', spell['targeting']['parameter'])
        self.assertIs(False, spell['targeting']['allow_on_self'])
        self.assertEqual(header['requirements']['vocations'], spell['requirements']['vocations'])
        self.assertEqual(header['presentation'], spell['presentation'])
        self.assertEqual(adapter.CANDIDATE_REVISION, spell['identity']['revision'])
        proof = self.values['source-qualification-proof.json']
        for name in ['native_identity_allocation', 'canonical_selection_changed', 'runtime_activation', 'native_execution_qualified', 'external_sources_used', 'legacy_matcher_modified']:
            self.assertIs(False, proof[name])


if __name__ == '__main__':
    unittest.main()
