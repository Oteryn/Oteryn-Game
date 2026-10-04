import copy
from pathlib import Path
import unittest

import project_player_control_candidates as adapter
import validate_spell

REPO = Path(__file__).resolve().parents[3]
SOURCE_ROOT = '/workspace/spell-sources'


class TargetControlProjectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.values = adapter.build_candidate(REPO, SOURCE_ROOT)

    def test_real_target_bundle_schema_and_existing_reader_shape(self):
        values = self.values
        self.assertEqual(validate_spell.validate(values['spell.json'], values['dependencies.json'], values['catalog.json']), [])
        self.assertTrue(adapter.validate_reader_shape(REPO, values['spell.json'], values['dependencies.json']))
        spell = values['spell.json']['spell']
        self.assertEqual(spell['execution']['ability']['family'], 'Ability')
        self.assertEqual(spell['targeting']['allowed_targets'], 'not_self')
        self.assertFalse(spell['targeting']['self_target'])
        self.assertTrue(spell['targeting']['needs_target'])
        self.assertTrue(spell['targeting']['check_floor'])
        self.assertNotIn('vocation_display_flags', spell['requirements'])
        self.assertIn('vocation_display_flags', values['source-header.json']['spell']['requirements'])

    def test_source_heal_formula_and_paralysis_dispel(self):
        deps = self.values['dependencies.json']; formula = deps['formulas'][0]
        self.assertEqual([effect['operation'] for effect in deps['effects']], ['heal', 'remove_condition'])
        self.assertEqual(deps['effects'][1]['removed_condition'], 'paralyze')
        self.assertEqual(deps['effects'][0]['presentation']['impact_asset_binding'], 'canary.appearance:effect/magic_blue')
        for level, magic in [(300, 0), (600, 90), (1500, 150)]:
            values = {'level': level, 'magic_level': magic}
            self.assertEqual(validate_spell.evaluate(formula['minimum'], values), level / 2.5 + magic * 20)
            self.assertEqual(validate_spell.evaluate(formula['maximum'], values), level / 2.5 + magic * 28)
        self.assertEqual(self.values['spell.json']['spell']['requirements']['level'], 300)

    def test_failure_presentation_difference_is_explicit_and_no_runtime_claim(self):
        proof = self.values['projection-receipt.json']
        self.assertFalse(proof['source_full_mechanics_1_to_1_complete'])
        self.assertFalse(proof['existing_reader_executed_on_this_bundle'])
        self.assertFalse(proof['runtime_activation'])
        self.assertFalse(proof['native_execution_qualified'])
        failure = proof['source_differences_and_provider_limits'][0]
        self.assertEqual(failure['source']['effect'], 'poff')
        self.assertEqual(failure['source']['message'], "You can't cast this spell to yourself.")
        self.assertIn('generic', failure['current_reader'])

    def test_reader_rejects_custom_guard_field_and_metadata(self):
        for section, key in [('targeting', 'refusal_presentation'), ('requirements', 'vocation_display_flags')]:
            changed = copy.deepcopy(self.values['spell.json']); changed['spell'][section][key] = []
            with self.assertRaisesRegex(ValueError, 'deny_unknown_fields'):
                adapter.validate_reader_shape(REPO, changed, self.values['dependencies.json'])

    def test_wrong_source_identity_fails_closed(self):
        with self.assertRaisesRegex(ValueError, 'source identity'):
            adapter.qualify(b'changed source', {'registration_key': adapter.REGISTRATION,
                                              'source_revision': adapter.SOURCE_REVISION, 'source_sha256': adapter.SOURCE_SHA})

    def test_21_specific_endpoint_proposals_no_placeholder_bundles(self):
        rows = adapter.build_matrix(REPO)
        self.assertEqual(len(rows), 21)
        self.assertEqual(sum(row['target_bundle_emitted'] for row in rows), 7)
        for row in rows:
            self.assertTrue(row['specific_unrepresented_behavior'])
            self.assertTrue(row['existing_runtime_endpoints'])
            self.assertTrue(row['proposed_owned_data_fields'])
            self.assertFalse(row['full_source_equivalent'])
        locate = next(row for row in rows if row['registration_key'].endswith('find_person.lua#1'))
        self.assertTrue(any('275' in detail and '251' in detail for detail in locate['specific_unrepresented_behavior']))
        cancel = next(row for row in rows if row['registration_key'].endswith('cancel_magic_shield.lua#1'))
        self.assertTrue(any('before Combat' in detail for detail in cancel['specific_unrepresented_behavior']))

    def test_current_forked_chain_source_values_differ_by_spell(self):
        rows = adapter.build_matrix(REPO)
        glacier = next(row for row in rows if row['registration_key'].endswith('forked_glacier.lua#1'))
        thorns = next(row for row in rows if row['registration_key'].endswith('forked_thorns.lua#1'))
        self.assertIn('seven', glacier['specific_unrepresented_behavior'][0]); self.assertIn('324', glacier['specific_unrepresented_behavior'][0])
        self.assertIn('six', thorns['specific_unrepresented_behavior'][0]); self.assertIn('325', thorns['specific_unrepresented_behavior'][0])

    def test_actual_canonical_base_chain_and_separate_wheel_augment(self):
        rows = adapter.build_chain_candidates(REPO, SOURCE_ROOT)
        self.assertEqual(len(rows), 2)
        for key, row_id, values in rows:
            self.assertEqual(validate_spell.validate(values['spell.json'], values['dependencies.json'], values['catalog.json']), [])
            self.assertTrue(adapter.validate_reader_shape(REPO, values['spell.json'], values['dependencies.json']))
            chain = values['dependencies.json']['abilities'][0]['chain']
            self.assertEqual(chain['shape'], 'fork'); self.assertFalse(chain['backtracking'])
            self.assertEqual(chain['range_tiles'], 4); self.assertEqual(chain['initial_range_tiles'], 7)
            self.assertNotIn('target_filter', chain)
            self.assertEqual(chain['max_targets'], 6 if 'glacier' in key else 5)
            proof = values['projection-receipt.json']
            self.assertEqual(proof['accepted_normalization']['source_chain_jump'], 5)
            self.assertEqual(proof['separate_augment_dependency']['binding_type'], 'ProjectV2AugmentBinding')
            self.assertFalse(proof['separate_augment_dependency']['binding_complete'])
            self.assertIn('level_base_damage_healing', str(values['dependencies.json']['formulas'][0]))
            self.assertEqual(values['receipt.json']['status'], 'CANDIDATE_SCHEMA_VALID')

    def test_native_templates_exact_equality_is_not_header_policy_acceptance(self):
        rows = adapter.build_existing_native_bindings(REPO)
        self.assertEqual(len(rows), 3)
        self.assertEqual(sum(len(values['projection-receipt.json']['registrations']) for _, values in rows), 4)
        for name, values in rows:
            proof = values['projection-receipt.json']
            self.assertTrue(proof['complete_profile_equality_verified'])
            self.assertTrue(proof['template_binding_requires_header_policy_review'])
            self.assertTrue(proof['template_values_are_not_claimed_current_source_correct'])
            self.assertFalse(proof['current_source_receipts_promoted'])
            self.assertEqual(values['spell.json']['spell']['identity']['revision'], 'spell-p2-r20')
        blood = next(values for name,values in rows if name == 'blood_rage')
        differences = blood['projection-receipt.json']['registrations'][0]['source_to_existing_template_differences']
        costs = next(row for row in differences if row['field'] == 'costs')
        self.assertEqual(costs['source']['mana'], 290); self.assertEqual(costs['existing_reader_template']['mana'], 20)

    def test_ultimate_caster_guard_real_requirement_mapping_not_silent_narrowing(self):
        row = adapter.build_ultimate_caster_requirements(REPO, SOURCE_ROOT)
        self.assertNotIn('exalted_monk', row['source_derived_requirements']['vocations'])
        self.assertIn('monk', row['source_derived_requirements']['vocations'])
        self.assertIn('none', row['source_derived_requirements']['vocations'])
        self.assertEqual(row['source_vocations_outside_current_reader_domain'], ['none'])
        self.assertNotIn('none', row['existing_reader_domain_requirements']['vocations'])
        self.assertFalse(row['full_spell_emitted'])


if __name__ == '__main__': unittest.main()
