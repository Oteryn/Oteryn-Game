import copy, gzip, json, unittest
from pathlib import Path
try:
    from . import dialogue_supplement as d
except ImportError:
    import dialogue_supplement as d
from jsonschema import Draft202012Validator
HERE=Path(__file__).parent

def n(t,**f):return {'node_type':t,'span':None,'fields':f}
def nam(v):return n('Name',id=v)
def row():return {'source':'canary','revision':d.PINS['canary'],'path':'data-otservbr-global/npc/tereban.lua','sha256':'a'*64,'git_blob_sha1':'b'*40}
class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet=json.loads(gzip.decompress((HERE/'dialogue.json.gz').read_bytes()));cls.schema=Draft202012Validator(json.loads((HERE/'dialogue_supplement.schema.json').read_text()))
    def test_schema(self):
        errors=list(self.schema.iter_errors(self.packet));self.assertEqual([list(e.path) for e in errors],[])
    def test_native_promotion_rejected(self):
        packet=copy.deepcopy(self.packet);packet['native_admission']=True
        with self.assertRaises(Exception):self.schema.validate(packet)
    def test_open_sea_factory_never_aliases_display_name(self):
        for donor in d.PINS:
            normal=next(r for r in self.packet['records'] if r['source']==donor and r['path'].endswith('/captain_haba.lua'))
            sea=next(r for r in self.packet['records'] if r['source']==donor and r['path'].endswith('/captain_haba_open_sea.lua'))
            self.assertEqual(normal['factory_name_declarations'][0]['literal'],'Captain Haba')
            self.assertEqual(sea['factory_name_declarations'][0]['literal'],'Captain Haba (Open Sea)')
            self.assertEqual(sea['display_name_declarations'][0]['literal'],'Captain Haba')
    def test_handler_actual_formal_binding(self):
        links=[v for v in self.packet['helper_links'] if v['symbol']=='ParseTerebanSay'];self.assertEqual(len(links),2)
        for link in links:
            handler=next(v for v in link['parameter_bindings'] if v['formal']=='npcHandler')
            self.assertEqual(d.name(handler['actual']),'npcHandler');self.assertFalse(link['native_actor_binding'])
    def test_alesar_unused_helper_not_promoted_to_caller_link(self):
        self.assertFalse(any(l['symbol']=='ParseAlesarSay' for l in self.packet['helper_links']))
    def test_local_shadow_blocks_global(self):
        r=row();ast=n('Chunk',body=n('Block',body=[n('LocalAssign',targets=[nam('ParseTerebanSay')],values=[nam('other')]),n('Call',func=nam('ParseTerebanSay'),args=[])]));proj=d.project(r,ast)
        helper=n('Function',name=nam('ParseTerebanSay'),args=[],body=n('Block',body=[]));r2=r|{'path':'data-otservbr-global/npc/tereban_functions.lua'}
        links=d.helper_links([(r,ast,proj),(r2,helper,d.project(r2,helper))]);self.assertEqual(links[0]['status'],'LOCAL_SHADOW_PRESERVED');self.assertEqual(links[0]['parameter_bindings'],[])
    def test_other_datapack_global_is_not_linked(self):
        r=row();ast=n('Call',func=nam('ParseTerebanSay'),args=[]);r2=r|{'path':'data-canary/npc/tereban_functions.lua'};helper=n('Function',name=nam('ParseTerebanSay'),args=[],body=n('Block',body=[]))
        links=d.helper_links([(r,ast,d.project(r,ast)),(r2,helper,d.project(r2,helper))]);self.assertEqual(links[0]['status'],'AMBIGUOUS_OR_MISSING_GLOBAL_DECLARATION')
    def test_tereban_failed_remove_not_success_branch(self):
        r=next(r for r in self.packet['records'] if r['source']=='canary' and r['path'].endswith('/tereban_functions.lua'))
        removal=next(e for e in r['events'] if e['type']=='ItemRemoval');self.assertEqual(removal['evaluation_role'],'CONDITION_EXPRESSION')
        failure=next(e for e in r['events'] if e['type']=='Utterance' and 'failure' in json.dumps(e.get('arguments')))
        success=next(e for e in r['events'] if e['type']=='Utterance' and 'success' in json.dumps(e.get('arguments')))
        writes=[e for e in r['events'] if e['type']=='StorageWrite' and removal['preorder']<e['preorder']<success['preorder']]
        self.assertTrue(writes);self.assertLess(removal['preorder'],failure['preorder']);self.assertLess(failure['preorder'],writes[0]['preorder'])
        self.assertEqual(failure['guards'][-1]['branch'],'TRUE')
        # Success after a failure-return retains that return; no fabricated negated reachability guard.
        self.assertTrue(any(e['type']=='ControlTransfer' and failure['preorder']<e['preorder']<writes[0]['preorder'] for e in r['events']))
    def test_source_event_order_stable(self):
        for r in self.packet['records']:
            self.assertEqual([e['preorder'] for e in r['events']],list(range(len(r['events']))))
            offsets=[e['witness']['span']['start_char'] for e in r['events'] if e['witness']['span']]
            self.assertEqual(offsets,sorted(offsets))
if __name__=='__main__':unittest.main()
