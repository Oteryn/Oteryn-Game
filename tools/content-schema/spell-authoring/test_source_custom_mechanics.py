import copy
import json
import pathlib
import unittest
import jsonschema
from source_custom_mechanics import descriptor, delay_ms, build
from unittest.mock import patch


class CustomMechanicsTests(unittest.TestCase):
    def capture(self, text):
        return descriptor(text.encode(), 'canary', 'a'*40, 'scripts/spells/monster/test.lua', 'b'*40, 'ground_transform')

    def test_literal_positions_restore_binding_and_comments(self):
        d = self.capture('''-- Position(1,2,3)
local items = {{itemid = 1867, position = Position(33588,32691,14)}}
ground:transform(21494)
addEvent(revertLava, 5 * 1000)
''')
        self.assertEqual(1, len(d['literal_positions']))
        self.assertEqual(1867, d['item_position_bindings'][0]['source_item_id'])
        self.assertEqual(5000, d['candidate_schedules'][0]['literal_delay_ms'])
        self.assertEqual('transform_item', d['candidate_actions'][0]['operation'])

    def test_spawn_random_targets_and_opaque_callback_delay(self):
        d = self.capture('''local p = Position(33708,32042,15)
local rand = math.random(1,4)
Game.createMonster("glooth-generator", generator[rand].pos, true, true)
addEvent(function() local a,b=1,2; ground:transform(a) end, dynamicDelay)
''')
        self.assertEqual([1,4], d['random_calls'][0]['literal_bounds'])
        self.assertEqual('spawn_monster', d['candidate_actions'][0]['operation'])
        self.assertIsNone(d['candidate_schedules'][0]['literal_delay_ms'])
        self.assertEqual('anonymous_function_body_not_exported', d['candidate_schedules'][0]['callback']['reason'])
        self.assertFalse(d['runtime_activation'])

    def test_gaz_helper_mutable_state_and_unavailable_binding_are_preserved(self):
        script = b"""local t = Game.getSpectators(creature:getPosition(),false,false,25,25,25,25)
if spectator:getName() == "Minion of Gaz'haragoth" then check=check+1 end
if check >= GazVariables.MaxSummons then return false end
if check < GazVariables.MinionsNow then
for i=1,(GazVariables.MinionsNow-check) do
monster=Game.createMonster("Minion of Gaz'haragoth",creature:getPosition(),true,false)
monster:setSummon(sum)
end
end
if math.random(0,100) < 25 then GazVariables.MinionsNow=GazVariables.MinionsNow+1 end
"""
        helper = b'GazVariables = {MinionsNow=2, MaxSummons=7}'
        def read(args, **kwargs):
            if 'rev-parse' in args:
                return 'b'*40+'\n'
            return helper if args[-1].endswith('gaz_functions.lua') else script
        with patch('source_custom_mechanics.SOURCES', {'canary':'a'*40}), patch('source_custom_mechanics.SCRIPTS', {"gaz'haragoth_summon":'area_bound_wild_minion_counting'}), patch('source_custom_mechanics.subprocess.check_output', side_effect=read):
            record = build('/unused-local')['records'][0]
        self.assertEqual({'MinionsNow':2,'MaxSummons':7}, record['gaz_state_contract']['initial_literal_state'])
        self.assertEqual('sum',record['unavailable_bindings'][0]['argument_symbol'])
        self.assertEqual(1,len(record['helper_sources']))
        self.assertFalse(record['gaz_state_contract']['owned_summon_filter_present'])
        schema=json.loads(pathlib.Path(__file__).with_name('source-custom-mechanics.schema.json').read_text())
        jsonschema.Draft202012Validator(schema).validate({'schema':'OTERYN_SOURCE_CUSTOM_MECHANICS/v1','runtime_activation':False,'record_count':1,'records':[record]})

    def test_unsupported_delay_arithmetic_is_not_evaluated(self):
        self.assertIsNone(delay_ms({'kind': 'expression_tokens', 'tokens': ['worldTime', '*', '1000']}))

    def test_strict_schema_refuses_runtime_and_unknown_properties(self):
        s = json.loads(pathlib.Path(__file__).with_name('source-custom-mechanics.schema.json').read_text())
        d = {'schema':'OTERYN_SOURCE_CUSTOM_MECHANICS/v1','runtime_activation':False,'record_count':1,'records':[self.capture('ground:transform(23049)')]}
        jsonschema.Draft202012Validator(s).validate(d)
        bad = copy.deepcopy(d);bad['records'][0]['runtime_activation']=True
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(s).validate(bad)
        bad = copy.deepcopy(d);bad['records'][0]['native_allocated_id']=23049
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(s).validate(bad)


if __name__ == '__main__':
    unittest.main()
