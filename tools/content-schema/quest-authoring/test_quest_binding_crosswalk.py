import copy
import json
from pathlib import Path
import tempfile
import unittest
from binding_packets.source import build, PREFIX

class CrosswalkQualification(unittest.TestCase):
    def fixture(self):
        donor={'identity':{'key':'canary:quest/test'},'display_name':'Test','missions':[]}
        track={'key':'source:track/test','missions':[],'start_of':['canary:quest/test'],'read_by_gates':[],
            'transitions':[{'key':'npc_1','script':'npc/test.lua','source_occurrences':[{'path':'npc/test.lua','line':4,'revision':'pin','blob_sha256':'abc'}]}]}
        graph={'identity':{'key':'source:interaction/test'},'source':{'edge':'USE','target_registrations':['aid(1)']}}
        claim={'identity':{'key':'source:claim/test'},'quest':{'key':'canary:quest/test'}}
        definition={'identity':{'key':'oteryn:quest.test'},'display_name':'Test','claims':[{'key':'native:claim/test'}],
            'source_refs':{'quest':{'key':'canary:quest/test'},'claims':[{'key':'source:claim/test'}]},
            'source_data':{'quest':copy.deepcopy(donor),'progress':[track],'interactions':[graph]}}
        return {'content/quests/definitions/index.json':{'shards':['content/quests/definitions/test.json']},
            'content/quests/definitions/test.json':{'records':[{'definition':definition}]},
            PREFIX+'source_migration/bundle.json':{'quests':[donor],'progress':[track],'interactions':[graph],'claims':[claim],'gates':[], 'gap_evidence':{'coverage_holds':[]}},
            PREFIX+'interactions/manifest.json':{'entries':[{'destination':'source:interaction/test','sources':[{'path':'quest/test.lua','revision':'pin'}]}]}}
    def invoke(self,files):
        with tempfile.TemporaryDirectory() as tmp:
            for path,obj in files.items():
                p=Path(tmp,path);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(obj))
            return build(tmp)
    def test_preserves_exact_registration_and_source_write_without_native_inference(self):
        files=self.fixture();out=self.invoke(files);self.assertEqual(out,self.invoke(files))
        q=out['quests'][0];self.assertEqual(q['trigger_bindings'][0]['target_registrations'],['aid(1)'])
        self.assertEqual(q['progress_bindings'][0]['writes'][0]['source_occurrences'][0]['line'],4)
        self.assertEqual(q['counts'],{'missions':0,'progress':1,'source_write_occurrences':1,'triggers':1,'gates':0,'rewards':1,'native_reward_refs':1})
        self.assertEqual(q['stage_full_coverage'],'NOT_ASSESSED')
    def test_rejects_wrong_source_quest(self):
        files=self.fixture();files['content/quests/definitions/test.json']['records'][0]['definition']['source_data']['quest']['identity']['key']='canary:quest/wrong'
        with self.assertRaisesRegex(ValueError,'wrong quest'):self.invoke(files)
    def test_rejects_reward_owned_by_another_quest(self):
        files=self.fixture();files[PREFIX+'source_migration/bundle.json']['claims'][0]['quest']['key']='canary:quest/wrong'
        with self.assertRaisesRegex(ValueError,'claim belongs'):self.invoke(files)
    def test_rejects_missing_graph_identity(self):
        files=self.fixture();files[PREFIX+'source_migration/bundle.json']['interactions']=[]
        with self.assertRaisesRegex(ValueError,'missing interaction'):self.invoke(files)
    def test_rejects_missing_track_identity(self):
        files=self.fixture();files[PREFIX+'source_migration/bundle.json']['progress']=[]
        with self.assertRaisesRegex(ValueError,'missing progress'):self.invoke(files)

if __name__=='__main__':unittest.main()
