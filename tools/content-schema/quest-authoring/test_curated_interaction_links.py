"""Finite callback links preserve partial SOURCE scope and independent track owners."""
import copy
import unittest
import ots_readiness as r

class CuratedInteractionLinksTests(unittest.TestCase):
    def setUp(self):
        self.key='canary:interaction/others/one'
        self.q={'identity':{'key':'canary:quest/documented'},'display_name':'Documented Quest','missions':[]}
        self.i={'identity':{'key':self.key},'source':{'edge':'USE'},'rules':[],'unresolved':[]}
        self.source={'source':'canary','path':'data-otservbr-global/scripts/quests/others/one.lua','callback_line':7,'blob_sha1':'a'*40}
        self.manifest=[{'destination':self.key,'sources':[self.source]}]
        self.entry={'title':'Documented Quest','interaction_keys':[self.key],'coverage_gap':'Only this component; rest UNKNOWN.','coverage_scope':'partial_executable_component','source_evidence':[{'source':'canary','repository':r.SOURCES['canary']['repository'],'revision':r.SOURCES['canary']['revision'],'path':self.source['path'],'line':7,'blob_sha1':'a'*40,'blob_sha256':'b'*64,'role':'executable','classification':'OTS_HYPOTHESIS_ONLY','interaction_key':self.key}]}
    def join(self,qs=None,ins=None,entry=None):
        return r.join_interactions(qs or [self.q],[],ins or [self.i],[entry or self.entry],self.manifest)
    def strict(self,entry=None,qs=None,ins=None):
        return r.curated_interaction_owners(qs or [self.q],ins or [self.i],[entry or self.entry],self.manifest,strict=True)
    def test_exact_component_only_no_others_directory_leak(self):
        other=copy.deepcopy(self.i);other['identity']['key']='canary:interaction/others/unrelated'
        got=self.join(ins=[self.i,other]);self.assertEqual(got[self.key],{'canary:quest/documented'});self.assertEqual(got[other['identity']['key']],set())
    def test_independent_character_track_owner_retained(self):
        second={'identity':{'key':'canary:quest/other'},'display_name':'Other Quest','missions':[{'progress':'canary:quest-progress/test'}]}
        i=copy.deepcopy(self.i);i['rules']=[{'owner':'Quest','progress':'canary:quest-progress/test','status':'known'}]
        self.assertEqual(self.join(qs=[self.q,second],ins=[i])[self.key],{'canary:quest/documented','canary:quest/other'})
    def test_stale_key_rejected_full_slice_but_projection_allowed(self):
        e=copy.deepcopy(self.entry);e['interaction_keys'].append('canary:interaction/others/missing')
        with self.assertRaisesRegex(ValueError,'stale curated interaction'):self.strict(e)
        self.assertEqual(self.join(entry=e)[self.key],{'canary:quest/documented'})
    def test_unknown_title_rejected_full_slice(self):
        e=copy.deepcopy(self.entry);e['title']='Absent Quest'
        with self.assertRaisesRegex(ValueError,'missing or ambiguous'):self.strict(e)
        self.assertEqual(self.join(entry=e)[self.key],set())
    def test_ambiguous_quest_title_rejected(self):
        second=copy.deepcopy(self.q);second['identity']['key']='crystalserver:quest/documented'
        with self.assertRaisesRegex(ValueError,'ambiguous'):self.strict(qs=[self.q,second])
    def test_path_line_and_blob_exactly_bound(self):
        for field,value in [('path','data-otservbr-global/wrong.lua'),('line',8),('blob_sha1','c'*40),('blob_sha256','BAD'),('source','crystalserver'),('role','comment'),('classification','READY'),('repository','wrong/repo'),('revision','0'*40)]:
            e=copy.deepcopy(self.entry);e['source_evidence'][0][field]=value
            with self.subTest(field=field),self.assertRaisesRegex(ValueError,'provenance'):self.strict(e)
    def test_witness_cannot_cover_different_callback(self):
        e=copy.deepcopy(self.entry);e['source_evidence'][0]['interaction_key']='canary:interaction/others/not_this'
        with self.assertRaisesRegex(ValueError,'provenance'):self.strict(e)
    def test_partial_hold_and_unique_keys_required(self):
        for field,value in [('coverage_gap',''),('coverage_scope','complete'),('interaction_keys',[self.key,self.key])]:
            e=copy.deepcopy(self.entry);e[field]=value
            with self.subTest(field=field),self.assertRaisesRegex(ValueError,'invalid finite'):self.strict(e)
    def test_no_world_character_numeric_alias_created(self):
        i=copy.deepcopy(self.i);i['rules']=[{'owner':'Quest','progress':'canary:quest-progress/storage/100','status':'blocked'}]
        q=copy.deepcopy(self.q);q['missions']=[{'progress':'canary:quest-progress/world/storage/100'}]
        self.assertEqual(r.join_interactions([q],[],[i],[],[])[self.key],set())
    def test_projection_without_title_keeps_old_track_join(self):
        q=copy.deepcopy(self.q);q.pop('display_name')
        self.assertEqual(self.join(qs=[q])[self.key],set())
    def test_computed_unknown_and_original_graph_untouched(self):
        i=copy.deepcopy(self.i);i['unresolved']=[{'reason':'computed source expression'}];before=copy.deepcopy(i)
        self.assertEqual(self.join(ins=[i])[self.key],{'canary:quest/documented'})
        self.assertEqual(i,before);self.assertEqual(r.interaction_facts(i)[2],1)

if __name__=='__main__':unittest.main()
