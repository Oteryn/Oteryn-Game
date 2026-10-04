import copy
import unittest
from jsonschema import Draft202012Validator
import project_monster_combat_controllers as m
import source_complete_monster_common as common

class ControllerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.rows=m.build()

    def family(self,stem):return [r for r in self.rows if r['controller']['parameters']['source_model']=='r65-'+stem]

    def test_exact_cohort_and_strict_controller_schema(self):
        self.assertEqual(len(self.rows),51)
        self.assertEqual(len({r['controller']['parameters']['source_model'] for r in self.rows}),21)
        common.verify_rows(65,self.rows)
        schema=m.schema(self.rows);Draft202012Validator.check_schema(schema)
        check=Draft202012Validator(schema)
        for row in self.rows:check.validate(row['controller'])
        broken=copy.deepcopy(self.rows[0]['controller']);broken['parameters']['opaque_lua']='runanything'
        self.assertTrue(list(check.iter_errors(broken)))

    def test_normalized_schedule_and_input_identity(self):
        for r in self.rows:
            p=r['source_parameters'];s=r['target_schedule']
            self.assertEqual(s['interval_ms'],p.get('interval',2000)%65536)
            self.assertEqual(s['chance_percent'],min(p.get('chance',100)%256,100))
            self.assertEqual(s['range_tiles'],min(p.get('range',0)%256,22))
            self.assertEqual(s['effective_sorted_signed_slot_damage'],sorted([p.get('minDamage',0),p.get('maxDamage',0)]))
            self.assertFalse(r['runtime_activation']);self.assertFalse(r['input_provider_equivalence'])
            self.assertFalse(r['source_consumer_implemented']);self.assertTrue(r['authoring_contract_extension_pending'])

    def test_skill_reducer_complete_random_and_closures(self):
        for stem,bounds in [('lisa_skill_reducer',[60,75]),('walker_skill_reducer',[45,60])]:
            for r in self.family(stem):
                p=r['controller']['parameters']
                self.assertEqual(len(p['combats']),16)
                self.assertEqual(p['selection']['uniform_integer'],bounds)
                self.assertEqual(p['attribute_conditions']['monk'],[])
                self.assertEqual(p['tile_creature_scan']['source_self_comparison'],'numeric_uid_vs_caster_userdata')
                self.assertEqual(p['callback_binding'],'each_registered_combat_closes_over_its_own_conditions')

    def test_signed_formula_a_terms_and_pairs_order(self):
        for stem in ('rotthing_wave','rootkraken_deathholy','rootkraken_rootearth'):
            for r in self.family(stem):
                p=r['controller']['parameters']
                self.assertEqual(p['iteration'],'lua_pairs');self.assertFalse(p['strict_execution_order'])
                for c in p['combats']:
                    f=c['fixed_formula']
                    if f:
                        self.assertEqual(f['effective_signed_minmax'],[f['min_a'],f['max_a']])
                        self.assertFalse(f['b_terms_used']);self.assertEqual(f['distribution'],'normal_random')
                        self.assertEqual(f['min_a'],0);self.assertGreater(f['max_a'],0)

    def test_guarded_heal_timing_marker_numeric_conversion(self):
        for stem,delay,duration in [('lisa_heal',6000,6000),('glooth_fairy_healing',10000,30000),('tyrn_heal',0,900000)]:
            for r in self.family(stem):
                p=r['controller']['parameters'];self.assertEqual(p['delay_ms'],delay)
                self.assertEqual(p['marker']['duration_ms'],duration)
                self.assertEqual(p['marker']['declared_health_gain'],.01)
                self.assertEqual(p['marker']['effective_health_gain'],0)
                if delay:self.assertEqual(p['actor_binding'],'captured_userdata');self.assertFalse(p['scheduled_presence_guard'])
        for r in self.family('minotaur_cult_prophet_mass_healing'):
            self.assertEqual(r['controller']['parameters']['roll_scope'],'script_initialization_once')

    def test_timed_stages_current_position_and_return_rules(self):
        for stem,delays in [('foam_splash',[1000,2000,3000]),('candy_horror_wave',[1,1]),('smelly_cheese_berserk',[500,700]),('angry_orc_ancestor_spirit_rooted',[2000]),('angry_orc_ancestor_spirit_fear',[2000])]:
            for r in self.family(stem):
                p=r['controller']['parameters'];self.assertEqual([x['requested_delay_ms'] for x in p['stages']],delays)
                self.assertEqual(p['scheduler_delay_floor_ms'],100);self.assertEqual(p['actor_binding'],'caster_reacquired_by_id')
                self.assertTrue(p['ignore_scheduled_combat_returns']);self.assertTrue(p['enqueue_result_ignored'])
        for r in self.family('plagirath_bog'):
            p=r['controller']['parameters'];self.assertTrue(p['scheduled_before_initial_combat'])
            self.assertEqual(p['delayed_guards']['distance_lt'],20);self.assertTrue(p['value_callback_requires_player'])
            self.assertEqual(p['declared_player_callback_signed_minmax'],[-1500,-1500])

    def test_world_mutations_and_order(self):
        for r in self.family('spider_queen_wrap'):
            p=r['controller']['parameters'];self.assertEqual(p['storage']['key'],43361)
            self.assertEqual(p['teleport']['delay_ms'],4500);self.assertEqual(p['teleport']['position'],{'x':32013,'y':32087,'z':10})
            self.assertEqual(p['outfit']['duration_ms'],30000)
        for r in self.family('ratmiral_fire_wave'):
            p=r['controller']['parameters'];self.assertEqual(p['wetness_storage']['key'],47519)
            self.assertTrue(p['wetness_icon']['positive_count']['write_icon_even_when_zero'])
            self.assertEqual(p['reduced_if_post_decrement_count_gt'],0)
        for r in self.family('the_welter_heal'):
            p=r['controller']['parameters'];self.assertEqual(p['heal'],25000);self.assertTrue(p['first_match_only'])
            self.assertEqual(p['commit_order'][:3],['victim_effect','remove_victim','caster_voice'])
        for r in self.family('omrafir_healing_2'):
            self.assertIsNone(r['controller']['parameters']['return_on_all_paths'])

    def test_ignite_missing_capture_variants_are_repaired(self):
        for r in self.family('ignite'):
            p=r['controller']['parameters'];self.assertEqual(len(p['combats']),3)
            self.assertEqual([x['second_target_effect'] for x in p['stance_routes']],[333,336])
            for c in p['combats']:
                self.assertEqual(c['conditions'][0]['damage_ticks'],[{'rounds':25,'interval_ms':3000,'signed_delta':-45}])
            self.assertEqual([c['conditions'][0]['type'] for c in p['combats']],['CONDITION_FIRE','CONDITION_ENERGY','CONDITION_CURSED'])

if __name__=='__main__':unittest.main()
