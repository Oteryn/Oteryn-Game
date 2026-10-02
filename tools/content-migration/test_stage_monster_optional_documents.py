import copy
import json
import unittest
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).parent))
import creature_admission_stage as original
import stage_monster_optional_documents as successor

DOC={'identity':{'key':'oteryn:document/monster-encyclopedia/rat','revision':'wiki-r1'},'document_type':'Other','title':'Rat','language':'pt-BR','content':['Original {{markup}}.']}

def packet():
    mapper=successor.DocumentMapper({})
    declaration=successor.declaration(DOC,mapper)
    carrier=dict(declaration,identity=dict(declaration['identity'],family='Document'))
    return {'records':[carrier], 'declarations':[], 'authoring_profiles':[{'data':{'kind':'Creature','profile':{'details':{'encyclopedia_document':mapper.ref({'family':'Document',**DOC['identity']})}}}}], 'counts':{'records':1,'encounters':0},'source':{},'source_identity_bindings':[{'retained':'byte exact'}]}

class DocumentAdmissionTests(unittest.TestCase):
    def test_declared_document_not_playable_preserves_text_and_bindings(self):
        before=packet();after=successor.finalize(before)
        self.assertEqual(after['records'],[])
        self.assertEqual(after['declarations'][0]['content'],DOC['content'])
        self.assertEqual(after['source_identity_bindings'],before['source_identity_bindings'])
        self.assertNotIn('family',after['declarations'][0]['identity'])
        self.assertEqual(after['counts']['encounters'],0)
        self.assertEqual(after['counts']['documents'],1)
        self.assertEqual(before['counts']['records'],1)

    def test_missing_document_reference_rejected(self):
        value=successor.finalize(packet());value['declarations']=[];value['counts']['documents']=0
        with self.assertRaisesRegex(original.StageError,'exact declared Document'):successor.validate_document_closure(value)

    def test_wrong_family_document_reference_rejected(self):
        value=packet();value['authoring_profiles'][0]['data']['profile']['details']['encyclopedia_document']['family']='Item'
        with self.assertRaisesRegex(original.StageError,'exact declared Document'):successor.finalize(value)

    def test_document_source_text_and_enum_errors_rejected(self):
        for field,bad in [('title',''),('content',[]),('content',['\x00']),('language','pt_BR'),('document_type','Encyclopedia')]:
            value=copy.deepcopy(DOC);value[field]=bad
            with self.subTest(field=field,bad=bad),self.assertRaises(original.StageError):successor.declaration(value,successor.DocumentMapper({}))

    def test_document_only_namespace_and_existing_item_fences(self):
        mapper=successor.DocumentMapper({1:'oteryn:item.allowed'})
        self.assertEqual(mapper.key('Item','canary:item/1'),'oteryn:item.allowed')
        for family,key in [('Item','canary:item/2'),('Document','outside:document/name'),('Document','oteryn:document/test-fixture')]:
            with self.subTest(key=key),self.assertRaises(original.StageError):mapper.key(family,key)

    def test_no_documents_is_exact_old_packet(self):
        value={'records':[],'declarations':[],'counts':{'records':0,'encounters':0},'source':{}}
        self.assertEqual(successor.finalize(value),value)

    def test_nested_loot_still_rejected(self):
        stage=successor.DocumentStage(successor.DocumentMapper({}))
        with self.assertRaisesRegex(original.StageError,'outside wave A'):
            stage.stage_dependencies({'formulas':[],'effects':[],'abilities':[],'items':[],'documents':[],'loot_tables':[{}]},'owner')

    def test_duplicate_declaration_rejected(self):
        value=packet();value['declarations']=[successor.declaration(DOC,successor.DocumentMapper({}))]
        with self.assertRaisesRegex(original.StageError,'Duplicate'):successor.finalize(value)

if __name__=='__main__':unittest.main()
