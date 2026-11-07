"""Finite Serpentine component ownership preserves Source holds and exact graph approval."""
import copy,json,os,unittest
from pathlib import Path
import ots_readiness as r
import source_fix_guard as guard
ROOT=Path(os.environ.get('WHITE_PEARL_FIXTURE',str(Path(__file__).resolve().parents[3])))
HERE=ROOT/'tools/content-schema/quest-authoring'
PROOF=HERE/'samples/source-proofs/serpentine-white-pearl.json'
PAIRS=HERE/'samples/source-proofs/serpentine-white-pearl-cores.json'
class WhitePearlCurationTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.entries=json.loads((HERE/'script_quests.json').read_text())['quests']
  cls.quests=json.loads((HERE/'samples/questlog/quests.json').read_text())['quests']
  cls.progress=json.loads((HERE/'samples/questlog/progress.json').read_text())['progress']
  cls.graphs=json.loads((HERE/'samples/interactions/interactions.json').read_text())['interactions']
  cls.manifest=json.loads((HERE/'samples/interactions/manifest.json').read_text())['entries']
  cls.proof=json.loads(PROOF.read_text());cls.pair=json.loads(PAIRS.read_text())['records'][0]
  cls.key=cls.proof['interaction_key'];cls.owner='canary:quest/serpentine_tower_quest'
 def test_exact_component_and_no_directory_leak(self):
  new=r.join_interactions(self.quests,self.progress,self.graphs,self.entries,self.manifest)
  entries=copy.deepcopy(self.entries);entry=next(e for e in entries if e['title']=='Serpentine Tower Quest');entry['interaction_keys'].remove(self.key)
  old=r.join_interactions(self.quests,self.progress,self.graphs,entries,self.manifest)
  self.assertEqual(new[self.key],{self.owner});self.assertFalse(old[self.key])
  self.assertEqual([k for k in old if old[k]!=new[k]],[self.key])
 def test_exact_registered_blob_proof_and_partial_hold(self):
  r.curated_interaction_owners(self.quests,self.graphs,self.entries,self.manifest,strict=True)
  entry=next(e for e in self.entries if e['title']=='Serpentine Tower Quest')
  self.assertEqual(entry['coverage_gap'],self.proof['coverage_gap'])
  graph=next(g for g in self.graphs if g['identity']['key']==self.key)
  self.assertEqual(guard.digest(graph),self.proof['graph_sha256'])
  self.assertFalse(self.proof['source_holds_resolved']);self.assertFalse(self.proof['runtime_enabled'])
 def proposal(self):
  old,new=copy.deepcopy(self.pair['before']),copy.deepcopy(self.pair['after'])
  change={'key':new['identity']['key'],'from_digest':guard.digest(old),'to_digest':guard.digest(new),'old_core':old,'new_core':new}
  approval={'approved_core_digests':[{'key':change['key'],'from_digest':change['from_digest'],'to_digest':change['to_digest']}],'approved_graphs':[{'key':k,'from_digest':a,'to_digest':b} for k,a,b in guard.graph_changes(old,new)]}
  return change,approval
 def test_existing_guard_admits_only_exact_approved_join(self):
  change,approval=self.proposal();self.assertEqual(approval['approved_graphs'][0]['from_digest'],None)
  guard.validate_change(change,{change['key']:change['from_digest']},change['new_core'],approval)
  self.assertEqual(guard.normalize_core(change['old_core']),guard.normalize_core(change['new_core']))
  self.assertEqual(change['new_core']['readiness'],'waiting_data')
 def test_missing_or_wrong_graph_approval_rejected(self):
  for graphs in ([],[{'key':self.key,'from_digest':None,'to_digest':'0'*64}]):
   change,approval=self.proposal();approval['approved_graphs']=graphs
   with self.assertRaisesRegex(ValueError,'unreviewed graph'):guard.validate_change(change,{change['key']:change['from_digest']},change['new_core'],approval)
 def test_unrelated_core_mutation_rejected_after_resigning(self):
  change,approval=self.proposal();change['new_core']['requirements']['min_level']=999999
  change['to_digest']=guard.digest(change['new_core']);approval['approved_core_digests'][0]['to_digest']=change['to_digest']
  with self.assertRaisesRegex(ValueError,'unrelated'):guard.validate_change(change,{change['key']:change['from_digest']},change['new_core'],approval)
 def test_native_hold_cannot_clear_after_resigning(self):
  change,approval=self.proposal();change['new_core']['native_lowering']['state']='READY'
  change['to_digest']=guard.digest(change['new_core']);approval['approved_core_digests'][0]['to_digest']=change['to_digest']
  with self.assertRaises(ValueError):guard.validate_change(change,{change['key']:change['from_digest']},change['new_core'],approval)
if __name__=='__main__':unittest.main()
