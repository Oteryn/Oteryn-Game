import copy
from pathlib import Path
import unittest

import project_state_native_extensions as adapter
import validate_spell

REPO=Path(__file__).resolve().parents[3]


class SourceStateExtensionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows=adapter.build(REPO)
        cls.schema=adapter.extension_schema(cls.rows)
        cls.validator=validate_spell.Draft202012Validator(cls.schema,registry=validate_spell.REGISTRY)

    def named(self,name,donor):
        return next(row for row in self.rows if row['registration_key'].startswith(donor) and row['registration_key'].endswith('/'+name+'.lua#1'))

    def test_item40450_reference_closure_for_both_owned_fields(self):
        for donor in ['canary','crystal']:
            row=self.named('divine_empowerment',donor)
            refs=adapter.base.item_references({'spell':row['spell'],'dependencies':row['dependencies']})
            self.assertEqual(row['catalog']['definitions'],refs)
            self.assertEqual(refs,[{'family':'Item','key':'candidate:item/40450','revision':'spell-p2-r21'}])

    def test_helper_proofs_read_immutable_pin_and_qualify_selected_scopes(self):
        proofs,scopes=adapter.immutable_helper_proofs()
        revision=adapter.previous.source_cast_programs_pins()['crystal']
        self.assertEqual(len(proofs),4)
        for path,digest in proofs.items():
            self.assertEqual(digest,adapter.previous.sha(adapter.base.source_file('/workspace/spell-sources/crystal',revision,path)))
        self.assertEqual(len(scopes),8)
        self.assertTrue(all(scope['revision']==revision and scope['qualified_literals'] for scope in scopes))

    def test_exact43_population_41_real_target_models_and2_removed_refs(self):
        self.assertEqual(len(self.rows),43)
        self.assertEqual(len({row['registration_key'] for row in self.rows}),43)
        self.assertEqual(sum('spell' in row for row in self.rows),41)
        refs=[row for row in self.rows if 'spell' not in row]
        self.assertEqual(len(refs),2)
        self.assertTrue(all(row['status']=='REFERENCE_ONLY_REMOVED_S24' and '/sap_strength.lua#1' in row['registration_key'] for row in refs))
        for row in self.rows:
            self.assertEqual(row['source_header_bytes'],(adapter.previous.base_row(REPO,row['registration_key'])/'source-header.json').read_bytes())
            self.assertFalse(row['runtime_activation']);self.assertFalse(row['native_execution_qualified']);self.assertFalse(row['source_consumer_implemented'])

    def test_every_target_model_validates_closed_private_schema(self):
        for row in self.rows:
            if 'spell' not in row:continue
            self.validator.validate(row['spell'])
            contract=row['spell']['spell']['source_state_contract']
            self.assertTrue(contract['authoring_contract_extension_pending'])
            self.assertFalse(contract['raw_source_equivalence'])
            if row['dependencies']['abilities']:
                ordinary=copy.deepcopy(row['spell']);ordinary['spell'].pop('source_state_contract')
                self.assertEqual(validate_spell.validate(ordinary,row['dependencies'],row['catalog']),[])

    def test_unknown_native_key_and_arbitrary_state_field_refused(self):
        row=copy.deepcopy(self.named('avatar_of_steel','canary')['spell'])
        row['spell']['execution']['native_behavior']['key']='generic_source_program'
        self.assertTrue(list(self.validator.iter_errors(row)))
        row=copy.deepcopy(self.named('master_of_flames','crystal')['spell'])
        row['spell']['execution']['native_behavior']['parameters']['run_lua']='anything'
        self.assertTrue(list(self.validator.iter_errors(row)))

    def test_shared_combat_alias_final_geometry_and_crystal_zero_branch(self):
        canary=self.named('great_death_beam','canary')['spell']['spell'];crystal=self.named('great_death_beam','crystal')['spell']['spell']
        params=canary['execution']['native_behavior']['parameters']
        self.assertTrue(params['shared_combat_alias']);self.assertTrue(canary['requirements']['wheel_unlock'])
        self.assertEqual([len(stage['north']) for stage in params['stages']],[8,8,8])
        self.assertEqual(params['stage_zero'],'refuse_before_costs')
        params=crystal['execution']['native_behavior']['parameters']
        self.assertFalse(params['shared_combat_alias']);self.assertFalse(crystal['requirements']['wheel_unlock'])
        self.assertEqual([len(stage['north']) for stage in params['stages']],[6,7,8])
        self.assertEqual(params['stage_zero'],'select_grade_one_and_cast')
        self.assertEqual(params['source_flank_augments']['source_stage_damage_factors'],[0,0.4,0.6,0.8])
        self.assertTrue(params['source_flank_augments']['runs_after_primary_even_if_primary_false'])

    def test_stance_clear_order_and_current_condition_values(self):
        params=self.named('blood_rage','crystal')['spell']['spell']['execution']['native_behavior']['parameters']
        self.assertEqual(params['condition']['melee_skill_percent'],130)
        self.assertEqual(params['condition']['damage_received_percent'],115)
        self.assertEqual(params['same_stance_order'],['remove_matching_attribute_condition_if_present','clear_slot','poff','return_true'])
        self.assertEqual(params['other_stance_order'],['set_slot_before_combat','execute_primary_combat','return_combat_result'])
        self.assertTrue(params['helper_modifiers']['state_setter']['lua_ignores_setter_result'])
        flame=self.named('master_of_flames','crystal')['spell']['spell']['execution']['native_behavior']['parameters']
        self.assertEqual(flame['slot'],'elemental')
        self.assertEqual(flame['helper_modifiers']['elemental']['base_power_multiplier'],1.04)
        self.assertTrue(flame['helper_modifiers']['elemental']['consume_pending_before_bonus'])

    def test_manashield_capacity_expiry_damage_and_commit_model(self):
        params=self.named('magic_shield','canary')['spell']['spell']['execution']['native_behavior']['parameters']
        self.assertEqual(params['duration_ms'],180000)
        self.assertEqual(params['application_timing'],'before_presentation_combat')
        self.assertEqual(params['damage_destination'],'mana_before_health_until_capacity_depleted')
        self.assertFalse(params['wheel_capacity_multiplier'])
        for level,magic,mana in [(14,5,100),(300,100,10000),(1000,140,12000)]:
            actual=validate_spell.evaluate(params['capacity'],{'level':level,'magic_level':magic,'maximum_mana':mana})
            import math
            self.assertEqual(actual,min(mana,math.ceil(7*magic+7.6*level+max(300,0.4*level))))

    def test_lightning_initial_selector_preserves_direction_branch(self):
        row=self.named('lightning','crystal');contract=row['spell']['spell']['source_state_contract']
        self.assertEqual(contract['chain_initial_selector']['direction_route'],'directional_single_target_without_chain')
        self.assertEqual(contract['chain_initial_selector']['target_route'],'explicit_target_then_sequential_chain')
        self.assertEqual(row['dependencies']['abilities'][0]['chain'],{'max_targets':2,'range_tiles':4,'initial_range_tiles':7,'shape':'sequential','backtracking':False})

    def test_optional_wheel_augments_separate_from_plain_base_native(self):
        for name in ['energy_wave','mass_healing']:
            for donor in ['canary','crystal']:
                spell=self.named(name,donor)['spell']['spell']
                params=spell['execution']['native_behavior']['parameters']
                self.assertNotIn('enhanced_area',params)
                self.assertIn('enhanced_area',spell['source_state_contract']['separate_augment_parameters'])
                self.assertFalse(spell['requirements'].get('wheel_unlock',False))


if __name__=='__main__':unittest.main()
