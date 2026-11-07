import copy,json,os,unittest
from pathlib import Path
import source_facts_authoring as a
ROOT=Path(os.environ.get('QUEST_SOURCE_TEST_ROOT',Path(__file__).parent))
P=Path(os.environ.get('QUEST_SOURCE_TEST_SAMPLES',ROOT/'samples/unbound-source-facts'))
class Facts(unittest.TestCase):
    def setUp(self):
        read=lambda p:json.loads(p.read_text())
        self.spec=read(ROOT/'samples/wiki-source-all373/source-specs-373.json');self.sel=read(P/'selection.json')
        self.data=read(P/'authored-facts.json');self.receipt=read(P/'receipt.json');self.schema=read(P/'source-facts.schema.json')
    def test_exact_portable_replay(self):self.assertEqual(a.build(self.spec,self.sel,self.data,self.receipt,self.schema),self.data)
    def reject(self,mutate,reanchor=False):
        data=copy.deepcopy(self.data);receipt=copy.deepcopy(self.receipt);mutate(data)
        if reanchor:receipt.update(authored_sha256=a.digest(data),record_digests=sorted(a.digest(r) for r in data['entries']))
        with self.assertRaises(Exception):a.build(self.spec,self.sel,data,receipt,self.schema)
    def test_runtime_false_closed(self):self.reject(lambda d:d.update(runtime_promotion=True),True)
    def test_missing_duplicate_title(self):self.reject(lambda d:d['entries'].pop(),True)
    def test_parent_projection_immutable(self):self.reject(lambda d:d['entries'][0]['source_reward_references'].clear(),True)
    def test_foreign_witness_pin(self):self.reject(lambda d:d['entries'][0]['source_directive_references'][0].update(revid=1),True)
    def test_unsigned_mutant_not_recorded(self):self.reject(lambda d:d['entries'][0]['source_directive_references'][0]['action_keywords'].append('invented'))
    def test_no_extra_object_fields(self):self.reject(lambda d:d['entries'][0].update(native_enum='SET'),True)
    def test_origin_scopes_preserved(self):
        self.assertEqual({x['origin'] for r in self.data['entries'] for x in r['source_reward_references']},{'source_specification','source_fact_supplements'})
if __name__=='__main__':unittest.main()
