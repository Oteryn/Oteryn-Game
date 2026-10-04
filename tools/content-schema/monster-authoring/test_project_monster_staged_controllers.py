"""R66 exact membership, typed branch contracts and independent Lua event oracle."""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
from lupa import LuaRuntime
import project_monster_staged_controllers as project
import source_complete_monster_common as common


class StagedMonsterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = project.build()
        cls.schema = json.loads(project.SCHEMA.read_text())

    def model(self, stem, donor='canary'):
        return next(row for row in self.rows if row['source']==donor and
                    Path(row['source_proofs'][0]['path']).stem==stem)

    def test_exact_cohort_and_source_identity(self):
        self.assertEqual(len(self.rows),83)
        self.assertEqual(len(common.verify_rows(66,self.rows)),83)
        self.assertEqual(len({row['source_proofs'][0]['path'].split('/')[-1] for row in self.rows}),25)
        for row in self.rows:
            self.assertTrue(row['full_slot_projection_complete'])
            self.assertFalse(row['runtime_activation'])
            self.assertFalse(row['native_provider_qualified'])
            self.assertFalse(row['source_consumer_implemented'])
            self.assertEqual(row['required_operations_unrepresented'],[])
            proof=row['source_proofs'][0]
            self.assertEqual(project.sha(project.read(row['source'],proof['path'])),proof['sha256'])

    def test_strict_controller_rejects_missing_or_invented_stage(self):
        Draft202012Validator.check_schema(self.schema)
        validate=Draft202012Validator(self.schema)
        for row in self.rows:validate.validate(row['controller'])
        changed=copy.deepcopy(self.model('doctor_marrow_explosion')['controller'])
        changed['parameters'].pop('blast')
        self.assertFalse(validate.is_valid(changed))
        changed=copy.deepcopy(self.model('doctor_marrow_explosion')['controller'])
        changed['parameters']['generic_lua_program']='return true'
        self.assertFalse(validate.is_valid(changed))

    def test_source_return_values_and_taunt_helpers_are_exact(self):
        energy=self.model('energy_pulse_explosion')['controller']['parameters']
        self.assertEqual(energy['return_values'],['combat_result','remove_result'])
        self.assertEqual(energy['core_consumes_return_value_index'],0)
        self.assertTrue(energy['remove_even_if_combat_false'])
        for donor in ('canary','crystal'):
            params=self.model('summonchallenge',donor)['controller']['parameters']['target_callback']
            self.assertEqual(params['cooldown_ms'],8000)
            self.assertTrue(params['helper_contract']['monster_summon_target_rejected'])
            self.assertEqual(params['helper_contract']['live_player_battle_healing_call'],donor=='canary')
        self.assertEqual(self.model('summon_challenge')['controller']['parameters']['target_callback']['cooldown_ms'],6000)

    def test_conditional_global_bug_and_effective_scheduler_floor(self):
        blue=next(row for row in self.rows if row['controller']['parameters'].get('zone_positions')=='external_global_zonePositions_default_nil')
        self.assertTrue(blue['source_runtime_error_known'])
        self.assertIn('Conditional',blue['source_runtime_error'])
        self.assertFalse(blue['controller']['parameters']['external_global_writers_qualified'])
        sapling=self.model('sapling_explode')['controller']['parameters']['removal']
        self.assertEqual(sapling['delay_ms'],1)
        self.assertEqual(sapling['effective_delay_ms'],100)

    def test_independent_original_lua_doctor_requested_event_timeline(self):
        lua=LuaRuntime(unpack_returned_tuples=True)
        lua.execute('''
            function stub() return setmetatable({}, {__index=function(t,k) return function(...) end end}) end
            Combat=stub; Condition=stub; createCombatArea=function(a) return a end
            function Spell() savedSpell=stub(); return savedSpell end
            requestedEvents={}
            function addEvent(fn,delay,...) requestedEvents[#requestedEvents+1]=delay end
            pos=stub(); caster=stub(); target=stub()
            caster.getId=function() return 1 end; caster.getPosition=function() return pos end
            target.getPosition=function() return pos end
            Creature=function(id) if id==1 then return caster elseif id==2 then return target end end
            variant={getNumber=function() return 2 end}
        ''')
        row=self.model('doctor_marrow_explosion');proof=row['source_proofs'][0]
        lua.execute(project.read(row['source'],proof['path']).decode())
        self.assertTrue(lua.eval('savedSpell.onCastSpell(caster,variant)'))
        actual=list(lua.globals().requestedEvents.values())
        params=row['controller']['parameters']
        pulses=params['paralyze_pulses']
        expected=list(range(pulses['first_ms'],pulses['last_ms']+1,pulses['interval_ms']))
        expected += [params['warning_repeat']['delay_ms'],params['blast']['delay_ms']]
        self.assertEqual(actual,expected)
        effective=[max(params['event_scheduler_minimum_delay_ms'],delay) for delay in actual]
        self.assertEqual(effective[:2],[100,100])

    def test_zone_helper_and_raw_health_direction(self):
        beam=self.model('spell-fire_beam_cruelty')['controller']['parameters']
        self.assertEqual(beam['delay_ms'],7000)
        self.assertEqual(beam['event_position'],'fresh_target_position')
        self.assertEqual(beam['tile_callback']['health_uniform_positive'],[2300,3000])
        self.assertEqual(beam['event_order'],['combat','target_remove_outfit_condition'])
        storage=self.model('outburst_explode')['controller']['parameters']['global_storage']
        self.assertEqual(storage['charging_killed']['key'],60183)
        self.assertEqual(storage['health']['key'],60182)
        self.assertEqual(storage['health_read_policy'],'read_positive_guard_then_fresh_second_read_else_zero')
        rot=self.model('rotthing_shaper')['controller']['parameters']
        self.assertEqual(rot['combat_iteration'],'source_pairs_unspecified_order')
        self.assertEqual([(c['damage']['minimum'],c['damage']['maximum']) for c in rot['combat_models']],[(0,900),(0,1000)])


if __name__=='__main__':unittest.main()
