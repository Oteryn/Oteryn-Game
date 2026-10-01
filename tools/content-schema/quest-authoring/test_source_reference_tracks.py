import copy
from collections import Counter
import tempfile
from pathlib import Path
import unittest
import json
import jsonschema

import source_reference_tracks as helper
import bundle_semantics
import ots_doors
import ots_readiness


def key_for(expression):
    if isinstance(expression,int):return 'storage/'+str(expression)
    return '/'.join(expression.split('.')[1:]).lower()


class ReferenceTrackTests(unittest.TestCase):
    def setUp(self):
        self.key='canary:quest-progress/quest/test/flag'
        self.expression='Storage.Quest.Test.Flag'
        self.readers={self.key:[{'expression':self.expression}]}
        self.declarations={self.key:{'expression':self.expression,'storage_id':7}}

    def retain(self,index=None,progress=None):
        return helper.retain_reader_tracks(progress or [],self.readers,self.declarations,index or {},key_for,
            lambda found:list(found['transitions'].values()),lambda value:copy.deepcopy(value))

    def test_declared_reader_retained_with_unknown_owner_not_invented_initial_bounds(self):
        added,checks=self.retain();record=added[0]
        self.assertEqual(record['writes'],{'canary':0,'crystalserver':0})
        self.assertEqual(record['transitions'],[])
        self.assertEqual(record['owner_basis'],'UNKNOWN')
        self.assertEqual(record['auxiliary_of'],[])
        self.assertTrue(record['source_checks']['owner'].startswith('UNKNOWN:'))
        self.assertFalse({'initial','bounds','stages','read_by_interactions'} & record.keys())
        self.assertEqual(checks[0]['readers'],self.readers[self.key])

    def test_literal_reader_is_source_qualified_unknown_and_never_named_numeric_alias(self):
        literal='canary:quest-progress/storage/14330'
        self.readers={literal:[{'expression':'14330'}]}
        self.declarations={self.key:{'storage_id':14330,'expression':self.expression}}
        record=self.retain()[0][0]
        self.assertEqual(record['key'],literal)
        self.assertNotIn('source_storage',record)
        self.assertNotIn('alias_of',record)
        self.assertIn('numeric writer inventory',record['source_checks']['owner'])
        self.assertIn('never joined',record['source_checks']['storage_declaration'])
        self.assertEqual(ots_readiness.uncertain_progress_keys([record]),{literal})
        jsonschema.validate({'progress':[record]},json.loads(Path(__file__).with_name('quest_progress.schema.json').read_text()))
        self.readers[literal][0]['expression']='Storage.Quest.Test.Flag'
        self.assertEqual(self.retain()[0],[])

    def test_existing_track_is_never_duplicated_or_changed(self):
        original={'key':self.key,'writes':{'canary':3}}
        snapshot=copy.deepcopy(original)
        self.assertEqual(self.retain(progress=[original])[0],[])
        self.assertEqual(original,snapshot)

    def test_existing_writer_evidence_is_preserved_never_fabricated_zero(self):
        found={'paths':[('canary','quest/test/flag')],'count':Counter(canary=1),
            'transitions':{'write':{'source_occurrences':[{'source':'canary','target':self.expression}]}}}
        index={'identity':found};snapshot=copy.deepcopy(index)
        record=self.retain(index)[0][0]
        self.assertEqual(record['writes']['canary'],1)
        self.assertEqual(record['transitions'],list(found['transitions'].values()))
        self.assertEqual(index,snapshot)
        found['count']['canary']=2
        with self.assertRaisesRegex(ValueError,'does not cover'):self.retain(index)

    def test_symbolic_mismatch_conflict_and_numeric_equality_never_join(self):
        self.declarations[self.key]={'storage_id':7,'expression':'Storage.Quest.Other.Flag'}
        self.assertEqual(self.retain()[0],[])
        self.declarations[self.key]={'state':'CONFLICT','candidates':[{'storage_id':7}]}
        self.assertEqual(self.retain()[0],[])
        self.declarations={self.key.replace('/quest/test/flag','/world/flag'):{'storage_id':7}}
        self.assertEqual(self.retain()[0],[])

    def test_normalized_writer_name_collision_remains_unknown(self):
        found={'paths':[('canary','quest/test/flag')],'count':Counter(canary=1),
            'transitions':{'write':{'source_occurrences':[{'source':'canary','target':'Storage.Quest.Test.FLAG'}]}}}
        self.assertEqual(self.retain({'identity':found})[0],[])

    def test_both_source_namespaces_and_prior_retention_receive_exact_alias(self):
        other=self.key.replace('canary:','crystalserver:')
        self.readers[other]=[{'expression':self.expression}]
        self.declarations[other]=copy.deepcopy(self.declarations[self.key])
        added,_=self.retain()
        self.assertEqual(len(added),2)
        self.assertEqual(added[1]['alias_of'],self.key)
        self.assertEqual(added[1]['writes'],added[0]['writes'])
        self.assertEqual(added[1]['transitions'],added[0]['transitions'])
        alias,_=self.retain(progress=[added[0]])
        self.assertEqual([p['key'] for p in alias],[other])
        del self.declarations[self.key]
        self.assertEqual(self.retain(progress=[added[0]])[0],[])

    def test_alias_schema_requires_known_declaration_when_no_gate_reader(self):
        schema=json.loads(Path(__file__).with_name('quest_progress.schema.json').read_text())
        declaration={'repository':'opentibiabr/canary','revision':'a'*40,
            'path':'data-otservbr-global/lib/core/storages.lua','line':1,'storage_id':7,'expression':self.expression}
        self.declarations[self.key]=declaration
        other=self.key.replace('canary:','crystalserver:')
        self.readers[other]=[{'expression':self.expression}]
        self.declarations[other]={**declaration,'repository':'zimbadev/crystalserver','path':'data-global/lib/core/storages.lua'}
        added,_=self.retain()
        jsonschema.validate({'progress':added},schema)
        del added[1]['source_storage']
        added[1]['source_checks']['storage_declaration']='UNKNOWN: absent declaration'
        with self.assertRaises(jsonschema.ValidationError):jsonschema.validate({'progress':added},schema)

    def test_orphan_changed_path_or_changed_occurrences_never_validate_as_alias(self):
        original=self.retain()[0][0]
        alias={**copy.deepcopy(original),'key':self.key.replace('canary:','crystalserver:'),
               'alias_of':self.key,'owner_basis':'exact source-path alias'}
        data={'quests':[],'progress':[alias],'gates':[],'interactions':[],'claims':[]}
        self.assertTrue(any(json.loads(g)['target_key']==self.key for g in bundle_semantics.reference_gaps(data)))
        data['progress']=[original,alias];bundle_semantics.validate_relations(data)
        alias['key']+='other'
        with self.assertRaisesRegex(ValueError,'conflates'):bundle_semantics.validate_relations(data)
        alias['key']=self.key.replace('canary:','crystalserver:');alias['writes']['canary']=1
        with self.assertRaisesRegex(ValueError,'differs'):bundle_semantics.validate_relations(data)
        alias['writes']=copy.deepcopy(original['writes']);alias['transitions']=[{'changed':True}]
        with self.assertRaisesRegex(ValueError,'differs'):bundle_semantics.validate_relations(data)

    def test_curated_gate_owner_precedes_label_and_shared_owner_stays_unknown(self):
        gate={'condition':{'progress':self.key},'label':'Wrong quest label','quest':None}
        owners={'quest/test/flag':{'quest':'canary:quest/actual'}}
        ots_doors.link_gate_quest(gate,lambda label:None,owners)
        self.assertEqual(gate['quest']['key'],'canary:quest/actual')
        self.assertEqual(gate['quest_link_basis'],'storage_key')
        owners['quest/test/flag'].update({'shared_npc_gate':True,'note':'two quest writers'})
        ots_doors.link_gate_quest(gate,lambda label:None,owners)
        self.assertIsNone(gate['quest'])

    def test_real_gate_build_reads_track_owners_json_not_basis_value_as_path(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temp:
            root=Path(temp)
            (root/'claims.json').write_text(json.dumps({'claims':[]}))
            (root/'track_owners.json').write_text(json.dumps({'tracks':{'quest/test/flag':{'quest':'canary:quest/actual'}}}))
            row={'kind':'quest','key':{'expr':self.expression},'position':(1,2,3),
                 'line':1,'server':'canary','label':'Wrong quest label','appearance':None}
            with patch.object(ots_doors,'ROOT',root),                 patch.object(ots_doors,'read_server',side_effect=lambda source,repo:[row] if source=='canary' else []),                 patch.object(ots_doors,'read_vocation_map',return_value={}),                 patch.object(ots_doors,'read_katana_lever',return_value=(99,(1,2,3))),                 patch.object(ots_doors,'read_secret_service_uid',return_value=98),                 patch.object(ots_doors,'wiki_matcher',return_value=lambda label:None),                 patch.object(ots_doors,'git_blob',return_value='0'*40),                 patch.object(ots_doors,'unused_decisions'):
                output=ots_doors.build({'canary':root,'crystalserver':root},root,root/'coverage.json')
            gate=output['gates.json']['gates'][0]
            self.assertEqual(gate['quest']['key'],'canary:quest/actual')
            self.assertEqual(gate['quest_link_basis'],'storage_key')

    def test_declared_unknown_track_keeps_consumer_gap_and_alias_uncertainty(self):
        record=self.retain()[0][0]
        alias={'key':self.key.replace('canary:','crystalserver:'),'alias_of':self.key}
        uncertain=ots_readiness.uncertain_progress_keys([record,alias])
        self.assertEqual(uncertain,{self.key,alias['key']})
        inter={'source':{'edge':'USE'},'rules':[{'branch':[{'when':{'quest_stage':{'progress':self.key,'op':'==','value':1}},'then':[]}]}],'unresolved':[]}
        self.assertEqual(ots_readiness.interaction_facts(inter,set(),set())[2],1)
        self.assertEqual(ots_readiness.interaction_facts(inter,{self.key},uncertain)[2],1)
        self.assertEqual(ots_readiness.interaction_facts(inter,{self.key},set())[2],0)

    def test_consuming_quest_interaction_and_gate_keep_declared_semantic_hold(self):
        import contextlib
        import io
        from unittest.mock import patch
        quest={'identity':{'key':'canary:quest/example'},'kind':'script_only','claims':[]}
        inter={'identity':{'key':'canary:interaction/example/action'},'source':{'edge':'USE'},
               'rules':[{'branch':[{'when':{'quest_stage':{'progress':self.key,'op':'==','value':1}},'then':[]}]}],'unresolved':[]}
        record=self.retain()[0][0]
        gate={'quest':{'key':quest['identity']['key']},'condition':{'kind':'quest_progress','progress':self.key}}
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temp:
            root=Path(temp)
            for name,payload in [('questlog',{'source_checks':{}}),('interactions',{'entries':[]})]:
                (root/name).mkdir();(root/name/'manifest.json').write_text(json.dumps(payload))
            def compile(progress,interactions,gates):
                inputs={'questlog/quests.json':[quest],'questlog/progress.json':progress,
                    'chests/claims.json':[],'doors/gates.json':gates,'interactions/interactions.json':interactions}
                with patch.object(ots_readiness,'SAMPLES',root),patch.object(ots_readiness,'OUT',root/'out.json'),\
                     patch.object(ots_readiness,'load',side_effect=lambda name,key:inputs[name]),contextlib.redirect_stdout(io.StringIO()):
                    ots_readiness.main()
                return json.loads((root/'out.json').read_text())
            before=compile([],[inter],[]);after=compile([record],[inter],[])
            self.assertEqual(before['quests'][0]['data_gaps']['unresolved_items'],1)
            self.assertEqual(after['quests'][0]['data_gaps']['unresolved_items'],1)
            self.assertEqual(after['counts']['quests_without_data_gaps'],0)
            self.assertEqual(compile([record],[],[gate])['quests'][0]['data_gaps']['unresolved_items'],1)
            known={k:v for k,v in record.items() if k!='source_checks'}
            self.assertEqual(compile([known],[inter],[gate])['quests'][0]['data_gaps']['unresolved_items'],0)

    def test_known_integer_table_rhs_is_safe_but_mutation_or_unknown_root_escape_is_not(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temp:
            repo=Path(temp);path=repo/'pack/scripts/quests/test/action.lua';path.parent.mkdir(parents=True)
            sources={'canary':{'datapack':'pack','repository':'example/repo','revision':'a'*40}}
            base='local cfg = { flag = Storage.Quest.Test.Flag, other = 1 }\n'
            use='player:getStorageValue(Storage.Quest.Test.Flag)\n'
            def read(text, declarations):
                path.write_text(text)
                return helper.storage_readers({'canary':repo},sources,key_for,{self.key},declarations)
            self.assertEqual(read(base+use,self.declarations)[self.key][0]['line'],2)
            self.assertEqual(read(base+use,{}),{})
            for change in ['Storage.Quest.Test.Flag, other = 99, 1\n',
                           'Storage.Quest.Test.Flag = 99\n','local Storage = {}\n',
                           'mutate(Storage)\n','local cfg2 = { branch = Storage.Quest.Test, other = 1 }\n']:
                self.assertEqual(read(base+change+use,self.declarations),{})

    def test_reader_comments_strings_shadowing_and_world_access_are_not_evidence(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temp:
            repo=Path(temp);path=repo/'pack/scripts/quests/test/action.lua';path.parent.mkdir(parents=True)
            sources={'canary':{'datapack':'pack','repository':'example/repo','revision':'a'*40}}
            def read(text):
                path.write_text(text)
                return helper.storage_readers({'canary':repo},sources,key_for,{self.key})
            actual=read('function a.onUse(player)\n return player:getStorageValue(Storage.Quest.Test.Flag)\nend\n')
            self.assertEqual(actual[self.key][0]['line'],2)
            self.assertEqual(actual[self.key][0]['receiver_expression'],'player')
            numeric='canary:quest-progress/storage/14330'
            path.write_text('player:getStorageValue(14330)\nplayer:getStorageValue(14330 + 1)\n')
            captured=helper.storage_readers({'canary':repo},sources,key_for,{numeric})
            self.assertEqual(len(captured[numeric]),1)
            self.assertEqual(captured[numeric][0]['expression'],'14330')
            for text in ['-- player:getStorageValue(Storage.Quest.Test.Flag)\n',
                'print("player:getStorageValue(Storage.Quest.Test.Flag)")\n',
                'Game.getStorageValue(Storage.Quest.Test.Flag)\n',
                'local Storage = {}\nplayer:getStorageValue(Storage.Quest.Test.Flag)\n',
                'player:getStorageValue(GlobalStorage.Quest.Test.Flag)\n']:
                self.assertEqual(read(text),{})


if __name__=='__main__':unittest.main()
