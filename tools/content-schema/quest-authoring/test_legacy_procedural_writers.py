"""Legacy storage calls must retain their exact argument roles and lexical builtin identity."""
from collections import Counter
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import lua_writers as writers
import ots_questlog as quests


class LegacyProceduralWriterTests(unittest.TestCase):
    def scan(self, body):
        return writers.scan('local function callback(creature, message)\n'+body+'\nend\n',
                            'data-otservbr-global/npc/example.lua')

    def test_scoped_character_alias_cannot_turn_game_world_write_into_progress(self):
        text='function a.onKill(player)\nlocal characterKey=775558\nlocal worldKey=673003\n'
        text+='player:setStorageValue(characterKey,player:getStorageValue(characterKey)+1)\n'
        text+='Game.setStorageValue(worldKey,0)\nend\n'
        rows=writers.scan(text,'scripts/quests/raging_mage_tower/example.lua')
        self.assertEqual([row['target'] for row in rows],['775558'])
        self.assertEqual(rows[0]['increment'],1)
        self.assertNotIn('initial',rows[0])
        self.assertNotIn('bounds',rows[0])

    def test_alias_after_closed_function_is_available_only_at_exact_method_sites(self):
        text='local function revert() end\nlocal State=Storage.Quest.Example.Stage\n'
        text+='function a.onUse(player)\nif player:getStorageValue(State)==3 then\n'
        text+='player:setStorageValue(State,4)\nend\nend\n'
        row=writers.scan(text,'scripts/quests/example.lua')[0]
        self.assertEqual(row['target'],'Storage.Quest.Example.Stage')
        self.assertEqual(row['from'],{'op':'==','value':3,'exact':True})
        self.assertEqual(row['to'],4)

    def test_branch_local_storage_alias_never_leaks_to_sibling_callback(self):
        text='function a.onUse(player)\nif allowed then\nlocal Key=44551\n'
        text+='player:setStorageValue(Key,1)\nend\nend\n'
        text+='function b.onUse(player)\nplayer:setStorageValue(Key,2)\nend\n'
        self.assertEqual([row['to'] for row in writers.scan(text,'scripts/quests/example.lua')],[1])

    def test_custom_inquisition_spelling_is_not_merged_with_global_pack(self):
        custom='local TheInquisitionQuest = Storage.Quest.U8_0.TheInquisitionQuest\n'
        custom+='function a.onStepIn(player)\nplayer:setStorageValue(TheInquisitionQuest.EnterTeleport,teleport.storage)\nend\n'
        global_pack=custom.replace('U8_0','U8_2')
        custom_row=writers.scan(custom,'data-crystal/scripts/quests/inqusition_quest/teleport_boss.lua')[0]
        global_row=writers.scan(global_pack,'data-global/scripts/quests/the_inquisition_quest/movements_teleport_main.lua')[0]
        self.assertEqual(custom_row['computed'],'expression')
        self.assertNotEqual(custom_row['target'],global_row['target'])
        self.assertIn('U8_0',custom_row['target'])
        self.assertIn('U8_2',global_row['target'])

    def test_repeated_nested_npc_keywords_satisfy_existing_unique_observation_contract(self):
        text='local function callback(creature,message)\nif MsgContains(message,"reason") then\n'
        text+='if MsgContains(message,"flatter") or MsgContains(message,"reason") then\n'
        text+='setPlayerStorageValue(creature,44551,1)\nend\nend\nend\n'
        row=writers.scan(text,'data-otservbr-global/npc/example.lua')[0]
        self.assertEqual(row['dialogue']['keywords'],['reason','flatter'])

    def test_storage_is_second_argument_never_receiver(self):
        row = self.scan('setPlayerStorageValue(700, 44551, 2)')[0]
        self.assertEqual((row['target'],row['to'],row['receiver_expression']),('44551',2,'700'))
        self.assertEqual(row['call_form'],'legacy_procedure')

    def test_exact_elementalist_five_calls_keep_four_tracks(self):
        body='\n'.join('setPlayerStorageValue(creature, Storage.Quest.U9_1.ElementalistOutfits.'+key+', '+str(value)+')'
                       for key,value in [('Addon1',1),('Addon2',1),('Outfit',1),('Questline',1),('Outfit',2)])
        rows=self.scan(body)
        self.assertEqual(len(rows),5)
        self.assertEqual(len({row['target'] for row in rows}),4)
        self.assertEqual([row['to'] for row in rows],[1,1,1,1,2])
        self.assertTrue(all(row['receiver_expression']=='creature' for row in rows))

    def test_computed_target_does_not_join_another_numeric_argument(self):
        self.assertEqual(self.scan('setPlayerStorageValue(123, resolveStorage(44551), 2)'),[])
        self.assertEqual(self.scan('setPlayerStorageValue(creature, unknownKey, 44551)'),[])

    def test_strings_comments_and_qualified_methods_are_not_builtin_calls(self):
        for body in ['-- setPlayerStorageValue(creature, 44551, 1)',
                     'local text="setPlayerStorageValue(creature, 44551, 1)"',
                     'other:setPlayerStorageValue(creature, 44551, 1)',
                     'other.setPlayerStorageValue(creature, 44551, 1)']:
            with self.subTest(body=body):self.assertEqual(self.scan(body),[])

    def test_rebound_parameter_alias_and_function_leave_builtin_unknown(self):
        for text in ['local setPlayerStorageValue=other\nsetPlayerStorageValue(creature,44551,1)',
                     'setPlayerStorageValue=other\nsetPlayerStorageValue(creature,44551,1)',
                     'local copy=setPlayerStorageValue\nsetPlayerStorageValue(creature,44551,1)',
                     'function setPlayerStorageValue(cid,key,value) end\nsetPlayerStorageValue(creature,44551,1)',
                     'function f(setPlayerStorageValue)\nsetPlayerStorageValue(creature,44551,1)\nend']:
            with self.subTest(text=text):self.assertEqual(writers.scan(text,'npc/example.lua'),[])

    def test_shadowed_or_escaped_storage_namespace_stays_unknown(self):
        for prefix in ['local Storage={Example=44551}\n', 'mutate(Storage)\n',
                       'Storage.Example=44551\n']:
            with self.subTest(prefix=prefix):
                self.assertEqual(self.scan(prefix+'setPlayerStorageValue(creature,Storage.Example,1)'),[])

    def test_argument_arity_and_incomplete_calls_fail_closed(self):
        for body in ['setPlayerStorageValue(creature,44551)',
                     'setPlayerStorageValue(creature,44551,1,2)',
                     'setPlayerStorageValue(creature,44551,1']:
            with self.subTest(body=body):self.assertEqual(self.scan(body),[])

    def test_multiline_nested_value_preserves_expression(self):
        row=self.scan('setPlayerStorageValue(\n creature,\n Storage.Quest.Example.Track,\n compute(1,2)\n)')[0]
        self.assertEqual(row['target'],'Storage.Quest.Example.Track')
        self.assertEqual(row['computed'],'expression')
        self.assertEqual(row['value'],'compute(1,2)')

    def test_procedural_timestamp_requires_a_real_unshadowed_call(self):
        self.assertEqual(self.scan('setPlayerStorageValue(creature,44551,os.time()+60)')[0]['computed'],'timestamp')
        self.assertEqual(self.scan('setPlayerStorageValue(creature,44551,"os.time()")')[0]['computed'],'expression')
        self.assertEqual(self.scan('local os=custom\nsetPlayerStorageValue(creature,44551,os.time()+60)')[0]['computed'],'expression')

    def test_same_receiver_guard_and_increment(self):
        row=self.scan('if getPlayerStorageValue(7,44551)==1 then\n'
                      ' setPlayerStorageValue(7,44551,getPlayerStorageValue(7,44551)+1)\nend')[0]
        self.assertEqual(row['from'],{'op':'==','value':1,'exact':True})
        self.assertEqual(row['increment'],1)

    def test_repeated_dynamic_receiver_calls_never_prove_same_actor(self):
        row=self.scan('if getPlayerStorageValue(nextPlayer(),44551)==1 then\n'
                      'setPlayerStorageValue(nextPlayer(),44551,2)\nend')[0]
        self.assertEqual(row['to'],2)
        self.assertIsNone(row['from'])
        row=self.scan('setPlayerStorageValue(nextPlayer(),44551,getPlayerStorageValue(nextPlayer(),44551)+1)')[0]
        self.assertEqual(row['computed'],'expression')
        self.assertNotIn('increment',row)

    def test_reassigned_or_unproved_receiver_parameter_keeps_write_but_not_actor_proof(self):
        for reassignment in ['', 'creature=other\n']:
            row=self.scan('if getPlayerStorageValue(creature,44551)==1 then\n'+reassignment+
                          'setPlayerStorageValue(creature,44551,getPlayerStorageValue(creature,44551)+1)\nend')[0]
            self.assertIsNone(row['from'])
            self.assertEqual(row['computed'],'expression')
            self.assertNotIn('increment',row)
            self.assertEqual(row['receiver_expression'],'creature')

    def test_other_receiver_or_rebound_reader_never_proves_guard_or_increment(self):
        for prefix,reader in [('', 'other'),('local getPlayerStorageValue=custom\n','creature')]:
            row=self.scan(prefix+'if getPlayerStorageValue('+reader+',44551)==1 then\n'
                          'setPlayerStorageValue(creature,44551,getPlayerStorageValue('+reader+',44551)+1)\nend')[0]
            self.assertIsNone(row['from'])
            self.assertEqual(row['computed'],'expression')

    def test_receiver_inside_guard_comment_is_not_evidence(self):
        row=self.scan('-- if getPlayerStorageValue(creature,44551)==1 then\n'
                      'setPlayerStorageValue(creature,44551,2)')[0]
        self.assertIsNone(row['from'])

    def test_character_method_results_remain_identical(self):
        body='if player:getStorageValue(Storage.Quest.Example.Track)==1 then\n'
        body+='player:setStorageValue(Storage.Quest.Example.Track,2)\nend'
        rows=self.scan(body)
        self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['to'],2)
        self.assertEqual(rows[0]['from'],{'op':'==','value':1,'exact':True})
        self.assertNotIn('call_form',rows[0])

    def test_procedural_npc_calls_keep_distinct_pinned_source_occurrences(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/quests.SOURCES['canary']['datapack']/'npc/example.lua'
            path.parent.mkdir(parents=True)
            path.write_text('local function callback(creature,message)\n'
                            'if MsgContains(message,"first") then\nsetPlayerStorageValue(creature,44551,1)\nend\n'
                            'if MsgContains(message,"second") then\nsetPlayerStorageValue(creature,44551,1)\nend\nend\n')
            found=quests.transition_index({'canary':root})[quests.norm('storage/44551')]
            self.assertEqual(found['count']['canary'],2)
            transition=quests.progress_transition(quests.mission_transitions(found)[0])
            self.assertEqual([row['line'] for row in transition['source_occurrences']],[3,6])
            self.assertEqual([row['write']['requested_by']['keywords'] for row in transition['source_occurrences']],
                             [['first'],['second']])
            self.assertNotIn('initial',transition)
            self.assertNotIn('bounds',transition)

    def test_exact_curator_precedes_unrelated_broad_prefix(self):
        target='quest/u10_50/dark_trails/mission03';quest='canary:quest/dark_trails_quest'
        found={'count':Counter({'canary':1}),'paths':{('canary',target)},'transitions':{
            'action':{'script':'scripts/quests/dark_trails/action.lua','owner':'action'}}}
        catalogue=[{'identity':{'key':quest},'display_name':'Dark Trails Quest'},
                   {'identity':{'key':'canary:quest/oramond_quest'},'display_name':'Oramond Quest',
                    'missions':[{'progress':'canary:quest-progress/quest/u10_50/oramond/status'}]}]
        with patch.object(quests,'TRACK_OWNERS',{quests.norm(target):{'quest':quest}}), \
             patch.object(quests,'storage_declarations',return_value={}), \
             patch.object(quests,'mission_transitions',return_value=[]):
            row=quests.auxiliary_tracks({quests.norm(target):found},[],catalogue,[],{})[0]
        self.assertEqual(row['auxiliary_of'],[quest])
        self.assertEqual(row['owner_basis'],'track_owners.json')

    def test_unexpected_writer_prevents_curated_npc_owner_false_join(self):
        target='quest/u10_50/dark_trails/mission01';quest='canary:quest/dark_trails_quest';npc='npc/ezebeth.lua'
        found={'count':Counter({'canary':2}),'paths':{('canary',target)},'transitions':{
            'npc':{'script':npc,'owner':'npc'},'other':{'script':'scripts/quests/foreign.lua','owner':'action'}}}
        with patch.object(quests,'TRACK_OWNERS',{quests.norm(target):{'quest':quest,'npc_sources':[npc]}}),\
             patch.object(quests,'storage_declarations',return_value={}),\
             patch.object(quests,'mission_transitions',return_value=[]):
            row=quests.auxiliary_tracks({quests.norm(target):found},[],[{'identity':{'key':quest},'display_name':'Dark Trails Quest'}],[],{})[0]
        self.assertEqual(row['auxiliary_of'],[])
        self.assertEqual(row['owner_basis'],'UNKNOWN')
        self.assertNotIn('initial',row)
        self.assertNotIn('bounds',row)


if __name__=='__main__':unittest.main()
