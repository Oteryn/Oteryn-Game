import copy
import json
from pathlib import Path
import unittest
import tempfile
import io
from contextlib import redirect_stdout, redirect_stderr
from unittest.mock import patch

import bundle_authoring as b
import bundle_semantics as s
import source_texts
import source_text_authoring

ROOT=Path(__file__).resolve().parent


def fixture():
    quest={'identity':{'key':'canary:quest/example','revision':'source-r1'},'display_name':'Example',
           'kind':'script_only','shown_in_quest_log':False,'claims':[],'wiki':{'title':'Example','pageid':1,'revid':1}}
    progress={'key':'canary:quest-progress/quest/u1_0/example/stage','missions':[],'start_of':[],
              'read_by_gates':[],'auxiliary_of':[quest['identity']['key']],'owner_basis':'track_owners.json',
              'writes':{'canary':1,'crystalserver':0},'transitions':[{'key':'action_1','script':'scripts/quests/example/action.lua',
                'sources':{'canary':{'path':'data-otservbr-global/scripts/quests/example/action.lua','line':2,'registrations':['id(1)']}},
                'write':{'key':'action_1','owner':'action','callback':'onUse','from':None,'computed':'expression','servers':['canary']}}]}
    transition=progress['transitions'][0]
    transition['source_occurrences']=[{**transition['sources']['canary'], 'source':'canary','occurrence':1,
        'repository':'opentibiabr/canary','revision':'0'*40,'target':'Storage.Quest.U1_0.Example.Stage',
        'blob_sha256':'0'*64,'line_sha256':'0'*64,'write':copy.deepcopy(transition['write'])}]
    wiki={'schema':'OTERYN_QUEST_CATALOGUE/v1','target_cut':'2026-09-27',
          'scope':'complete_source_inventory; candidate coverage only; no runtime promotion',
          'input_provenance':[{'path':'tools/content-schema/quest-authoring/test'+str(i),'sha256':'0'*64} for i in range(4)],
          'historical_coverage_ots_sources':{},'summary':{'source_titles':0,'authored_records':1,'coverage_states':{},'quest_log_conflicts':0,'duplicate_title_checks':0},
          'quests':[],'fresh_acquisition_summary':{},'source_access_checks':[],'unavailable_source_pages':[]}
    data={'quests':[quest],'progress':[progress],'interactions':[],'gates':[],'claims':[],
          'wiki_catalogue':wiki,'interaction_source_conflicts':[]}
    data['source_texts']=source_texts.registry(data,{'texts':[]})
    data['source_texts']['capture_provenance']={'path':'tools/content-schema/quest-authoring/source_text_capture.json','sha256':b.digest(ROOT/'source_text_capture.json')}
    data['source_texts']['input_provenance']=[{'role':role,'path':rel,'sha256':'0'*64} for role,rel in sorted(source_text_authoring.INPUTS.items())]
    evidence={'reported_readiness':[],'coverage_holds':[],'deferred_owners':[]}
    value={'schema':'OTERYN_QUEST_SOURCE_BUNDLE/v1','classification':'OTS_HYPOTHESIS_ONLY',
           'scope':'source_inventory_only; no native or runtime authority','canonical_admission':'NOT_ASSESSED','runtime_readiness':'UNKNOWN',
           'sources':[{'repository':'opentibiabr/canary','revision':'0'*40}],
           'input_provenance':[{'role':role,'path':path,'sha256':'0'*64} for role,path in sorted(b.INPUTS.items())],
           'schema_provenance':b.schema_provenance(ROOT,ROOT),'tool_provenance':b.tool_provenance(),
           'gap_evidence':evidence,**data}
    value.update(s.derive(data,evidence))
    return value


class BundleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.validator=b.offline_validator(ROOT,ROOT)
    def setUp(self):self.value=fixture()
    def validate(self):b.validate(self.value,self.validator,b.schema_provenance(ROOT,ROOT))
    def test_computed_source_only_passes_with_explicit_gaps(self):
        self.validate();self.assertEqual(self.value['runtime_readiness'],'UNKNOWN')
        self.assertTrue(self.value['quest_gaps'][0]['gaps'])
    def test_missing_write_rejected(self):
        self.value['progress'][0]['transitions'][0].pop('write')
        with self.assertRaisesRegex(ValueError,'write'):self.validate()
    def test_computed_cannot_be_relabelled_as_constant(self):
        self.value['progress'][0]['transitions'][0]['write']['to']=1
        with self.assertRaises(ValueError):self.validate()
    def test_extra_runtime_authority_rejected(self):
        self.value['progress'][0]['runtime_ready']=True
        with self.assertRaisesRegex(ValueError,'runtime_ready'):self.validate()
    def test_source_write_descriptor_mismatch_rejected(self):
        self.value['progress'][0]['transitions'][0]['write']['servers']=['crystalserver']
        with self.assertRaisesRegex(ValueError,'descriptor mismatch'):self.validate()
    def test_alias_cannot_conflate_paths(self):
        data=copy.deepcopy(self.value)
        original=data['progress'][0]
        alias=copy.deepcopy(original);alias['key']='crystalserver:quest-progress/quest/u1_0/example/stage_other'
        alias['alias_of']=original['key'];data['progress'].append(alias)
        with self.assertRaisesRegex(ValueError,'distinct source paths'):s.validate_relations(data)
    def test_alias_cycle_rejected(self):
        data=copy.deepcopy(self.value);a=data['progress'][0];c=copy.deepcopy(a)
        c['key']=a['key'].replace('canary:','crystalserver:');a['alias_of']=c['key'];c['alias_of']=a['key'];data['progress'].append(c)
        with self.assertRaisesRegex(ValueError,'alias cycle'):s.validate_relations(data)
    def test_duplicate_identities_rejected(self):
        self.value['quests'].append(copy.deepcopy(self.value['quests'][0]))
        with self.assertRaisesRegex(ValueError,'duplicate identity'):self.validate()
    def test_source_line_zero_rejected(self):
        self.value['progress'][0]['transitions'][0]['sources']['canary']['line']=0
        with self.assertRaises(ValueError):self.validate()
    def test_missing_gap_inventory_rejected(self):
        self.value['quest_gaps'][0]['gaps']=[];self.value['summary']['quests_with_reported_gaps']=0
        with self.assertRaisesRegex(ValueError,'derived source gaps'):self.validate()
    def test_exact_unresolved_reference_inventory_required(self):
        self.value['progress'][0]['auxiliary_of'].append('canary:quest/missing')
        with self.assertRaisesRegex(ValueError,'reference-gap inventory'):self.validate()
    def test_forged_schema_provenance_rejected(self):
        self.value['schema_provenance'][0]['sha256']='f'*64
        with self.assertRaisesRegex(ValueError,'schema provenance'):self.validate()
    def test_stale_generator_provenance_rejected(self):
        self.value['tool_provenance'][0]['sha256']='f'*64
        with self.assertRaisesRegex(ValueError,'generator provenance'):self.validate()


class SourceBackedTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.samples=Path(self.tmp.name);value=fixture()
        docs={role:({role:value[role]} if role in s.COLLECTIONS else {}) for role in b.INPUTS}
        docs['wiki_catalogue']=value['wiki_catalogue'];docs['readiness']={'quests':[]}
        docs['questlog_manifest']={'source_checks':{},'sources':[{'kind':'git',**value['sources'][0]}]}
        docs['interactions_manifest']={'entries':[]}
        for role,rel in b.INPUTS.items():
            path=self.samples/rel;path.parent.mkdir(parents=True,exist_ok=True)
            path.write_text(json.dumps(docs[role])+'\n')
        text_registry=source_text_authoring.build(self.samples,ROOT/'source_text_capture.json',ROOT)
        (self.samples/b.INPUTS['source_texts']).parent.mkdir(parents=True,exist_ok=True)
        (self.samples/b.INPUTS['source_texts']).write_text(source_text_authoring.encoded(text_registry))
        self.value=b.build(ROOT,self.samples,ROOT)
    def install_conflict(self, missing_progress=None):
        key='canary:interaction/example/action'
        track=self.value['progress'][0]['key']
        primary={'identity':{'key':key,'revision':'source-r1'},
                 'source':{'edge':'USE','callback':'onUse','target_registrations':['id(1)']},
                 'rules':[{'owner':'Presentation','effect':'magic_effect','authoritative':False}],
                 'anchors':[],'unresolved':[]}
        alternative=copy.deepcopy(primary)
        children=[{'owner':'Quest','request':'set_progress','progress':track,'to':2}]
        if missing_progress:
            children.append({'owner':'Quest','request':'set_progress',
                             'progress':missing_progress,'to':3})
        alternative['rules']=[{'branch':[{'when':{'quest_stage':{
            'progress':track,'op':'==','value':1},'negate':False},'then':children}]}]
        alternatives=[{'source':'canary','interaction':primary},
                      {'source':'crystalserver','interaction':alternative}]
        witnesses=[{'source':source,'path':prefix+'/scripts/quests/example/action.lua',
                    'callback_line':2,'blob_sha1':str(ordinal)*40}
                   for ordinal,(source,prefix) in enumerate([
                       ('canary','data-otservbr-global'),('crystalserver','data-global')],1)]
        docs={'interactions':{'interactions':[primary]},'interactions_manifest':{'entries':[
            {'destination':key,'status':'conflict','sources':witnesses,
             'conflict_alternatives':alternatives}]}}
        for role,doc in docs.items():
            (self.samples/b.INPUTS[role]).write_text(json.dumps(doc)+'\n')
        (self.samples/b.INPUTS['source_texts']).write_text(source_text_authoring.encoded(
            source_text_authoring.build(self.samples,ROOT/'source_text_capture.json',ROOT)))
        self.value=b.build(ROOT,self.samples,ROOT)
        return alternatives
    def test_full_typed_conflict_alternatives_preserved_and_strictly_validated(self):
        alternatives=self.install_conflict()
        conflict=self.value['interaction_source_conflicts'][0]
        self.assertEqual(conflict['classification'],'CONFLICT')
        self.assertEqual(conflict['alternatives'],alternatives)
        self.assertNotEqual(alternatives[0]['interaction']['rules'],
                            alternatives[1]['interaction']['rules'])
        self.assertEqual(json.loads(b.encoded(self.value))['interaction_source_conflicts'][0],conflict)
        b.validate_source_backed(self.value,ROOT,self.samples,ROOT)
        self.value['interaction_source_conflicts'][0]['alternatives'][1]['interaction'][
            'rules'][0]['branch'][0]['then'][0]['to']='dynamic value'
        with self.assertRaisesRegex(ValueError,'interaction_source_conflicts'):
            b.validate(self.value,b.offline_validator(ROOT,ROOT))
    def test_alternative_undeclared_progress_is_exact_reference_and_quest_gap(self):
        missing='crystalserver:quest-progress/quest/u1_0/example/undeclared'
        self.install_conflict(missing)
        key=self.value['interactions'][0]['identity']['key']
        field='/conflict_alternatives/crystalserver/rules/0/branch/0/then/1/progress'
        self.assertEqual(self.value['reference_gaps'],[{
            'record_type':'Interaction','record_key':key,'field':field,
            'target_type':'Progress','target_key':missing,
            'reason':'reference not declared in this source bundle'}])
        gaps=self.value['quest_gaps'][0]['gaps']
        self.assertIn({'kind':'source_reference_gap','record':key+field,'reason':missing},gaps)
        self.assertTrue(any(g['kind']=='source_coverage_hold' and g['record']==key for g in gaps))
        b.validate_source_backed(self.value,ROOT,self.samples,ROOT)
        validator=b.offline_validator(ROOT,ROOT)
        stale=copy.deepcopy(self.value);stale['reference_gaps']=[]
        with self.assertRaisesRegex(ValueError,'reference-gap inventory'):
            b.validate(stale,validator)
        stale=copy.deepcopy(self.value)
        stale['quest_gaps'][0]['gaps']=[g for g in gaps if g['kind']!='source_reference_gap']
        with self.assertRaisesRegex(ValueError,'derived source gaps'):
            b.validate(stale,validator)
    def test_original_packet_matches_exact_inputs(self):
        b.validate_source_backed(self.value,ROOT,self.samples,ROOT)
    def test_mutated_display_name_rejected_against_sources(self):
        self.value['quests'][0]['display_name']='Mutated title'
        b.validate(self.value,b.offline_validator(ROOT,ROOT))
        with self.assertRaisesRegex(ValueError,'source-backed regeneration'):
            b.validate_source_backed(self.value,ROOT,self.samples,ROOT)
    def test_zero_input_sha_rejected_against_sources(self):
        self.value['input_provenance'][0]['sha256']='0'*64
        b.validate(self.value,b.offline_validator(ROOT,ROOT))
        with self.assertRaisesRegex(ValueError,'source-backed regeneration'):
            b.validate_source_backed(self.value,ROOT,self.samples,ROOT)
    def test_standalone_output_explicitly_states_no_input_proof(self):
        path=self.samples/'packet.json';path.write_text(json.dumps(self.value))
        output=io.StringIO()
        with patch('sys.argv',['bundle_authoring.py','--validate',str(path)]),redirect_stdout(output):
            self.assertEqual(b.main(),0)
        result=json.loads(output.getvalue())
        self.assertEqual(result['validation_mode'],'STRUCTURAL_ONLY')
        self.assertEqual(result['input_provenance_verification'],'NOT_VERIFIED')
    def test_source_backed_requires_validate(self):
        with patch('sys.argv',['bundle_authoring.py','--source-backed']),redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as caught:b.main()
        self.assertEqual(caught.exception.code,2)


if __name__=='__main__':unittest.main()
