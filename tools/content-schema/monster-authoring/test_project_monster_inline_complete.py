import copy
import math
import unittest

import jsonschema
import project_monster_inline_complete as adapter


class CompleteInlineControllerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows=adapter.build();cls.schema=adapter.schema(cls.rows)
        cls.validator=jsonschema.Draft202012Validator(cls.schema)

    def test_exact_ten_remaining_partition_and_original_invariants(self):
        source=adapter.cohort()
        self.assertEqual(len(self.rows),10)
        self.assertEqual(len({adapter.canonical(row['slot_identity']) for row in self.rows}),10)
        for row,original in zip(self.rows,source):
            for field in ['slot_identity','source_parameters','original_slot_sha256']:self.assertEqual(row[field],original[field])
            self.assertTrue(row['full_slot_projection_complete'])
            self.assertTrue(row['authoring_contract_extension_pending'])
            for flag in ['runtime_activation','native_execution_qualified','input_provider_equivalence','source_consumer_implemented']:self.assertFalse(row[flag])

    def test_closed_schema_metaschema_and_every_concrete_controller(self):
        jsonschema.Draft202012Validator.check_schema(self.schema)
        for row in self.rows:self.validator.validate(row['controller'])
        jsonschema.Draft202012Validator.check_schema(adapter.fixed([]))
        self.assertNotIn('prefixItems',adapter.fixed([]))

    def test_mixed_signed_grimeleech_preserves_healing_branch_and_offset(self):
        selected=[row for row in self.rows if row['monster']=='Grimeleech']
        self.assertEqual(len(selected),3)
        for row in selected:
            signed=row['controller']['parameters']['signed_health']
            self.assertEqual((signed['minimum'],signed['maximum']),(-565,100))
            self.assertEqual(signed['positive_branch'],'gain_target_health')
            self.assertEqual(signed['negative_branch'],'undefined_type_target_health_damage')
            self.assertEqual(signed['quantization'],'minimum_plus_lround_of_normalized_draw_times_maximum_minus_minimum')
            self.assertEqual(signed['equal_bounds_rule'],'still_draw_then_return_bound')
            for draw,expected in [(0,-565),(0.5,-232),(1,100)]:
                self.assertEqual(signed['minimum']+math.floor(draw*(signed['maximum']-signed['minimum'])+0.5),expected)

    def test_no_mana_or_lifedrain_alias_and_no_heal_aggression_repair(self):
        for row in self.rows:
            params=row['controller']['parameters'];state=params['signed_health']
            self.assertEqual(params['constructor']['combat_type'],'COMBAT_UNDEFINEDDAMAGE')
            self.assertEqual(params['constructor']['donor_combat_type_id'],4)
            self.assertEqual(state['resource_destination'],'health')
            self.assertFalse(state['valid_mana_drain_dispatch']);self.assertFalse(state['builtin_life_drain_type'])
            self.assertEqual(state['mana_shield_rule'],'skip_when_current_health_damage_type_is_undefined_after_hooks')
            self.assertTrue(params['constructor']['aggressive']);self.assertFalse(params['constructor']['blocked_by_armor']);self.assertFalse(params['constructor']['blocked_by_shield'])
            self.assertFalse(params['source_type_repair_performed']);self.assertIsNone(params['canonical_normalization'])
            self.assertEqual(params['donor_intended_damage_type'],'UNKNOWN')

    def test_radius_three_is_exact_donor_stencil_not_three_tile_disk(self):
        selected=[row for row in self.rows if row['source_parameters'].get('radius')==3]
        self.assertEqual(len(selected),3)
        for row in selected:
            geo=row['controller']['parameters']['geometry']
            self.assertEqual(len(geo['active_cells']),9)
            self.assertEqual({(cell['dx'],cell['dy']) for cell in geo['active_cells']},{(x,y) for x in [-1,0,1] for y in [-1,0,1]})
            self.assertEqual(sum(cell['origin'] for cell in geo['active_cells']),1)
            self.assertEqual(geo['base_draw_scope'],'one_per_combat_dispatch');self.assertTrue(geo['per_target_damage_copy'])

    def test_actual_fallback_audio_overwrites_cast_not_impact(self):
        for row in self.rows:
            audio=row['controller']['parameters']['presentation']
            self.assertEqual(audio['impact_sound']['source_symbol'],'SOUND_EFFECT_TYPE_SILENCE')
            expected='SOUND_EFFECT_TYPE_MONSTER_SPELL_SMALL_AREA_HIT' if row['source_parameters'].get('radius')==3 else 'SOUND_EFFECT_TYPE_MONSTER_SPELL_SINGLE_TARGET_HIT'
            self.assertEqual(audio['cast_sound']['source_symbol'],expected)
            self.assertEqual(audio['cast_sound_write_sequence'][-1],expected)
            self.assertFalse(audio['sound_playback_provider_qualified'])

    def test_immutable_full_helpers_and_selected_function_scopes(self):
        for donor in ['canary','crystal']:
            proofs,scopes=adapter.helpers(donor)
            for proof in proofs:self.assertEqual(proof['sha256'],adapter.sha(adapter.inline.read(donor,proof['path'])))
            self.assertEqual(len(scopes),9)
            self.assertTrue(all(scope['qualified_literals'] and scope['revision']==adapter.inline.PINS[donor] for scope in scopes))

    def test_schema_refuses_unsigned_abs_conversion_or_generic_program(self):
        bad=copy.deepcopy(self.rows[0]['controller']);bad['parameters']['signed_health']['minimum']=565
        self.assertTrue(list(self.validator.iter_errors(bad)))
        bad=copy.deepcopy(self.rows[0]['controller']);bad['kind']='source_ast_program'
        self.assertTrue(list(self.validator.iter_errors(bad)))
        bad=copy.deepcopy(self.rows[0]['controller']);bad['parameters']['lua']='anything'
        self.assertTrue(list(self.validator.iter_errors(bad)))


if __name__=='__main__':unittest.main()
