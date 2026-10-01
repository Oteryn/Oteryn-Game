import copy
import unittest
from jsonschema.exceptions import ValidationError
from wheel_authoring import ROOT, VOCATIONS, build, read, validate, validate_allocation, validate_gem, effective_gem_grades
class WheelAuthoringTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.candidate=read(ROOT/'samples/wheel-candidate.json')
    def reject(self, mutate, code=None):
        c=copy.deepcopy(self.candidate);mutate(c)
        if code:
            with self.assertRaisesRegex(ValueError,code):validate(c)
        else:
            with self.assertRaises((ValueError,ValidationError)):validate(c)
    def test_rebuild_matches_committed_candidate(self):self.assertEqual(build(),self.candidate);validate(self.candidate)
    def test_all_180_planner_generated_allocations(self):
        snapshots=read(ROOT/'samples/planner-allocation-snapshots.json');self.assertEqual(len(snapshots),180)
        for s in snapshots:
            with self.subTest(vocation=s['vocation'],points=s['spent']):validate_allocation(self.candidate,s['vocation'],s['points'],s['spent'])
    def test_signed_gem_penalty_is_preserved(self):
        gem=next(m for m in self.candidate['gems']['basic_mods']if m['source_id']==7)
        self.assertEqual(gem['effects'][1]['values_by_vocation']['knight'],[-1.0]*4)
    def test_exact_mitigation_not_rounded_increment(self):
        for voc in VOCATIONS:
            values=[e['value_per_point']for s in self.candidate['vocations'][voc]['slots']for e in s['dedication']if e['stat']=='mitigation_multiplier']
            self.assertEqual(len(values),8);self.assertEqual(set(values),{0.075})
    def test_unknown_field_rejected(self):self.reject(lambda c:c.update(extra=True))
    def test_runtime_admission_rejected(self):self.reject(lambda c:c.update(runtime_admitted=True))
    def test_duplicate_slot_rejected(self):self.reject(lambda c:c['topology'][1].update(state_slot=1),'SLOT_IDENTITIES')
    def test_wrong_domain_rejected(self):self.reject(lambda c:c['topology'][0].update(domain='red'),'DOMAIN_CROSSWALK')
    def test_self_dependency_rejected(self):self.reject(lambda c:c['topology'][0].update(unlock_from_any_full_slot=[1]),'SELF_DEPENDENCY')
    def test_isolated_cycle_in_graph_rejected(self):
        def mutate(c):c['topology'][0]['unlock_from_any_full_slot']=[2];c['topology'][1]['unlock_from_any_full_slot']=[1]
        self.reject(mutate,'UNREACHABLE_TOPOLOGY')
    def test_bad_domain_total_rejected(self):self.reject(lambda c:c['topology'][0].update(capacity=150),'DOMAIN_CAPACITY')
    def test_duplicate_vocation_slot_rejected(self):self.reject(lambda c:c['vocations']['knight']['slots'][1].update(state_slot=1),'VOCATION_SLOT_IDENTITIES')
    def test_wrong_dedication_unit_rejected(self):self.reject(lambda c:c['vocations']['knight']['slots'][0]['dedication'][0].update(unit='percent_points'),'DEDICATION_UNIT')
    def test_unknown_mod_reference_rejected(self):self.reject(lambda c:c['vocations']['knight']['supreme_mods'].append(250),'UNKNOWN_MOD_REFERENCE')
    def test_duplicate_mod_rejected(self):self.reject(lambda c:c['gems']['basic_mods'].append(copy.deepcopy(c['gems']['basic_mods'][0])),'DUPLICATE_MOD_ID')
    def test_wrong_gem_shape_rejected(self):self.reject(lambda c:c['gems']['qualities'][2].update(basic_mod_count=1),'GEM_QUALITY_SHAPE')
    def test_wrong_icon_mapping_rejected(self):self.reject(lambda c:c['vocations']['knight']['slots'][0]['conviction']['icon'].update(source_index=99),'CONVICTION_ICON')
    def test_bad_revelation_threshold_rejected(self):self.reject(lambda c:c['vocations']['knight']['revelations'][0]['stages'][1].update(minimum_domain_points=499),'REVELATION_THRESHOLDS')
    def test_duplicate_mod_grade_rejected(self):self.reject(lambda c:c['gems']['supreme_mods'][0]['grades'][1].update(grade=0),'MOD_GRADES')
    def test_input_digest_rejected(self):self.reject(lambda c:c['input_digests'].update({'source-graph.json':'0'*64}),'INPUT_DIGEST_MISMATCH')
    def test_zero_allocation(self):validate_allocation(self.candidate,'knight',[0]*36,0)
    def test_budget_rejection(self):
        p=[0]*36;p[14]=50
        with self.assertRaisesRegex(ValueError,'POINT_BUDGET_EXCEEDED'):validate_allocation(self.candidate,'knight',p,49)
    def test_slot_capacity_rejection(self):
        p=[0]*36;p[14]=51
        with self.assertRaisesRegex(ValueError,'SLOT_CAPACITY_EXCEEDED'):validate_allocation(self.candidate,'knight',p,100)
    def test_minimum_point_rejection(self):
        p=[0]*36;p[0]=1
        with self.assertRaisesRegex(ValueError,'MINIMUM_POINTS_NOT_MET'):validate_allocation(self.candidate,'knight',p,50)
    def test_disconnected_allocation_rejection(self):
        p=[0]*36;p[0]=200
        with self.assertRaisesRegex(ValueError,'DISCONNECTED_ALLOCATION'):validate_allocation(self.candidate,'knight',p,4000)
    def test_bool_is_not_points(self):
        p=[0]*36;p[14]=True
        with self.assertRaisesRegex(ValueError,'INVALID_POINT_VECTOR'):validate_allocation(self.candidate,'knight',p,100)
    def test_value_only_numeric_successor(self):
        c=copy.deepcopy(self.candidate);c.update(revision='r2',release={'kind':'value_only','predecessor':self.candidate['revision']});c['vocations']['knight']['slots'][0]['dedication'][0]['value_per_point']=4;validate(c,self.candidate)
    def test_value_only_rejects_structure_change(self):
        c=copy.deepcopy(self.candidate);c.update(revision='r2',release={'kind':'value_only','predecessor':self.candidate['revision']});c['vocations']['knight']['slots'][0]['conviction']['key']='other_perk'
        with self.assertRaisesRegex(ValueError,'VALUE_ONLY_STRUCTURE_CHANGED'):validate(c,self.candidate)
    def test_value_only_rejects_augment_kind_change(self):
        c=copy.deepcopy(self.candidate);c.update(revision='r2',release={'kind':'value_only','predecessor':self.candidate['revision']});c['vocations']['knight']['slots'][5]['conviction']['augment_stages'][0]['numeric_effects'][0]['kind']='critical_extra_damage'
        with self.assertRaisesRegex(ValueError,'VALUE_ONLY_STRUCTURE_CHANGED'):validate(c,self.candidate)
    def test_successor_requires_matching_previous(self):
        c=copy.deepcopy(self.candidate);c.update(revision='r2',release={'kind':'wheel_reset','predecessor':'wrong'})
        with self.assertRaisesRegex(ValueError,'REVISION_CHAIN'):validate(c,self.candidate)
    def test_successor_requires_previous(self):
        c=copy.deepcopy(self.candidate);c.update(revision='r2',release={'kind':'value_only','predecessor':self.candidate['revision']})
        with self.assertRaisesRegex(ValueError,'PREDECESSOR_REQUIRED'):validate(c)
    def test_augment_unit_mismatch_rejected(self):self.reject(lambda c:c['vocations']['knight']['slots'][5]['conviction']['augment_stages'][0]['numeric_effects'][0].update(unit='mana'),'AUGMENT_EFFECT_UNIT')
    def test_dedication_wrong_sprite_rejected(self):self.reject(lambda c:c['vocations']['knight']['slots'][0]['dedication_icon'].update(sprite='basic_mod'),'DEDICATION_ICON_SPRITE')
    def test_revelation_parameter_is_required(self):
        self.reject(lambda c:c['vocations']['knight']['revelations'][0]['stages'][0]['numeric_effects'].pop(),'REVELATION_PARAMETERS')
    def test_revelation_unit_is_typed(self):
        c=copy.deepcopy(self.candidate)
        c['vocations']['knight']['revelations'][0]['stages'][0]['numeric_effects'][0]['unit']='seconds'
        with self.assertRaises(ValidationError):validate(c)
    def test_revelation_correction(self):
        rev=next(r for r in self.candidate['vocations']['sorcerer']['revelations'] if r['key']=='lord_of_destruction')
        effect=next(e for e in rev['stages'][1]['numeric_effects'] if e['kind']=='mastery_decay_critical_extra_damage')
        self.assertEqual(effect['value'],22.5)
        self.assertIn('25.50',rev['stages'][1]['reference_description'])
    def test_avatar_cooldown_minutes_converted_to_seconds(self):
        rev=next(r for r in self.candidate['vocations']['monk']['revelations'] if r['key']=='avatar_of_balance')
        values=[next(e['value'] for e in s['numeric_effects'] if e['kind']=='cooldown') for s in rev['stages']]
        self.assertEqual(values,[7200,5400,3600])
    def test_resonance_mapping_rejected(self):
        self.reject(lambda c:c['vocations']['knight']['resonance_slots']['green'].__setitem__(0,1),'RESONANCE_SLOTS')
    def test_gem_family_mismatch_rejected(self):
        self.reject(lambda c:c['vocations']['monk']['gem_family'].update(family='guardian'),'GEM_FAMILY_BINDING')
    def test_fragment_yield_inversion_rejected(self):
        self.reject(lambda c:c['gems']['atelier']['fragment_yields']['lesser'].update(revealed=[4,3]),'FRAGMENT_YIELD_RANGE')
    def test_clockwise_order_rejected(self):
        self.reject(lambda c:c['gems']['atelier'].update(clockwise_domains=['green','blue','purple','red']),'CLOCKWISE_DOMAINS')
    def test_cooldown_mod_grade_must_use_momentum(self):
        def mutate(c):
            mod=next(m for m in c['gems']['supreme_mods'] if m['source_id']==6)
            mod['grades'][1]['numeric_effects'][0]['value']=1000
        self.reject(mutate,'COOLDOWN_GRADES_REQUIRE_MOMENTUM')
    def test_gem_catalogue_pairs(self):
        for voc,data in self.candidate['vocations'].items():
            for first in data['basic_mods_position_1']:
                second=next(x for x in data['basic_mods_position_2'] if x!=first)
                validate_gem(self.candidate,voc,'lesser',first)
                validate_gem(self.candidate,voc,'regular',first,second)
                validate_gem(self.candidate,voc,'greater',first,second,data['supreme_mods'][0])
    def test_duplicate_basic_mod_refused(self):
        with self.assertRaisesRegex(ValueError,'DUPLICATE_BASIC_MOD'):validate_gem(self.candidate,'knight','regular',3,3)
    def test_supreme_wrong_vocation_refused(self):
        with self.assertRaisesRegex(ValueError,'SUPREME_NOT_ALLOWED'):validate_gem(self.candidate,'knight','greater',3,0,91)
    def test_lesser_extra_mod_refused(self):
        with self.assertRaisesRegex(ValueError,'GEM_MOD_SHAPE'):validate_gem(self.candidate,'monk','lesser',3,0)
    def test_grade_chain_caps_preceding_mods(self):
        self.assertEqual(effective_gem_grades([3,1,2]),[3,1,1])
        self.assertEqual(effective_gem_grades([0,3,3]),[0,0,0])
    def test_grade_chain_rejects_invalid_grade(self):
        for grades in ([],[4],[True],[1,2,3,0]):
            with self.assertRaisesRegex(ValueError,'INVALID_GEM_GRADES'):effective_gem_grades(grades)
    def test_icon_asset_mismatch_rejected(self):
        self.reject(lambda c:c['vocations']['knight']['slots'][0]['dedication_icon'].update(asset_url='https://static.tibia.com/images/wrong.png'),'ICON_ASSET_BINDING')
    def test_unique_parameters_are_required(self):
        self.reject(lambda c:c['vocations']['knight']['slots'][0]['conviction'].update(unique_parameters=None),'UNIQUE_PARAMETER_PRESENCE')
    def test_unique_parameters_have_typed_units(self):
        c=copy.deepcopy(self.candidate)
        parameter=c['vocations']['knight']['slots'][0]['conviction']['unique_parameters']['numeric_effects'][0]
        parameter['unit']='seconds'
        with self.assertRaises(ValidationError):validate(c)
    def test_unique_parameter_omission_rejected(self):
        self.reject(lambda c:c['vocations']['knight']['slots'][0]['conviction']['unique_parameters']['numeric_effects'].pop(),'UNIQUE_PARAMETERS')
    def test_unique_behavior_binding_rejected(self):
        self.reject(lambda c:c['vocations']['knight']['slots'][0]['conviction']['unique_parameters'].update(behaviors=['shield_doubles_bonus']),'UNIQUE_BEHAVIOR_BINDING')
    def test_no_unique_payload_on_stat_perk(self):
        def mutate(c):
            data=c['vocations']['knight']['slots']
            stat=next(s['conviction'] for s in data if s['conviction']['category']=='skill_bonus')
            stat['unique_parameters']=copy.deepcopy(data[0]['conviction']['unique_parameters'])
        self.reject(mutate,'UNIQUE_PARAMETER_PRESENCE')
    def test_unique_coverage_all_vocations(self):
        for data in self.candidate['vocations'].values():
            uniques={s['conviction']['key'] for s in data['slots'] if s['conviction']['category']=='unique'}
            self.assertEqual(len(uniques),2)
    def test_revelation_sprite_cells_cover_ids(self):
        evidence=self.candidate['icon_evidence']['revelation_asset']
        self.assertEqual(evidence['width'],evidence['height']*evidence['cell_count'])
        self.assertTrue(evidence['visual_inspection'])
        for data in self.candidate['vocations'].values():
            for rev in data['revelations']:self.assertLess(rev['source_info_id'],evidence['cell_count'])
    def test_official_8944_cooldown_corrections(self):
        aug={s['conviction']['key']:s['conviction'] for s in self.candidate['vocations']['monk']['slots']}
        for key,stage in [('augmented_mystic_repulse',1),('augmented_thousand_fist_blows',2)]:
            effect=next(e for e in aug[key]['augment_stages'][stage-1]['numeric_effects'] if e['kind']=='cooldown_reduction')
            self.assertEqual(effect['value'],4)
            self.assertIn('-6s',aug[key]['augment_stages'][stage-1]['reference_text'])
    def test_great_fire_wave_conflict_selection(self):
        aug=next(s['conviction'] for s in self.candidate['vocations']['sorcerer']['slots'] if s['conviction']['key']=='augmented_great_fire_wave')
        effect=aug['augment_stages'][0]['numeric_effects'][0]
        self.assertEqual((effect['kind'],effect['value']),('critical_hit_chance',10))
    def test_flurry_area_reference(self):
        aug=next(s['conviction'] for s in self.candidate['vocations']['monk']['slots'] if s['conviction']['key']=='augmented_flurry_of_blows')
        self.assertEqual(aug['augment_stages'][0]['area_reference'],'AREA_GREATER_FLURRY_OF_BLOWS')
    def test_verification_evidence_binds_candidate(self):
        import hashlib
        from jsonschema import Draft202012Validator
        evidence=read(ROOT/'samples/verification-evidence.json')
        Draft202012Validator(read(ROOT/'verification.schema.json')).validate(evidence)
        self.assertEqual(evidence['candidate_sha256'],hashlib.sha256((ROOT/'samples/wheel-candidate.json').read_bytes()).hexdigest())
        self.assertEqual(evidence['counts']['slots'],sum(len(v['slots']) for v in self.candidate['vocations'].values()))
        self.assertEqual(evidence['source_conflicts'],self.candidate['gems']['reference_corrections'])
        self.assertFalse(evidence['live_verification']['live_global_parity_confirmed'])
if __name__=='__main__':unittest.main()
