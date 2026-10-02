"""Real R10 receipt regressions, with isolated copies and no helper monkeypatch."""
import copy
import json
import os
import shutil
import tempfile
import unittest
from pathlib import Path
import source_fix_guard as guard
FIXTURE=Path(os.environ.get('SOURCE_FIX_GUARD_FIXTURE',str(Path(__file__).resolve().parents[3])))
class SourceFixGuardTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.baseline=guard.read_json(FIXTURE/(guard.DIRECTORY+'baseline-core-digests.json'))
  cls.current={}
  for path in guard.read_json(FIXTURE/'content/quests/definitions/index.json')['shards']:
   for row in guard.read_json(FIXTURE/path)['records']:
    core=copy.deepcopy(row['definition'])
    if core.get('definition_profile')=='oteryn_authored_v1':continue
    core.pop('oteryn_recipe',None);cls.current[core['identity']['key']]=core
  cls.receipt=guard.read_json(FIXTURE/guard.RECEIPT)
  cls.approval=guard.read_json(FIXTURE/guard.APPROVAL)
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
  self.root=Path(self.temp.name);self.baseline=copy.deepcopy(self.baseline);self.current=copy.deepcopy(self.current)
  self.receipt=copy.deepcopy(self.receipt);self.approval=copy.deepcopy(self.approval)
  descriptors=[self.receipt['approval']]+sum((self.receipt[k] for k in ('immutable_inputs','compiler_inputs','proof_inputs')),[])
  for path in {guard.RECEIPT}|{d['path'] for d in descriptors}:
   dest=self.root/path;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(FIXTURE/path,dest)
 def write_receipt(self):
  (self.root/guard.RECEIPT).write_text(json.dumps(self.receipt))
 def reject_effective(self):
  with self.assertRaises(ValueError):guard.effective_digests(self.root,self.baseline,self.current)
 def semantic_mutant(self,mutate):
  change=copy.deepcopy(next(c for c in self.receipt['changes'] if c['new_core'].get('source_data')))
  mutate(change['new_core']);change['to_digest']=guard.digest(change['new_core'])
  # Reseal only the full-core approval signature to reach structural safety guards.
  for row in self.approval['approved_core_digests']:
   if row['key']==change['key']:row['to_digest']=change['to_digest']
  with self.assertRaises(ValueError):guard.validate_change(change,self.baseline,change['new_core'],self.approval)
 def test_actual_ten_reviewed_core_changes_pass(self):
  expected,proof=guard.effective_digests(self.root,self.baseline,self.current)
  self.assertEqual(len(self.receipt['changes']),10);self.assertIsNotNone(proof)
  for change in self.receipt['changes']:
   self.assertEqual(expected[change['key']],change['to_digest'])
 def test_bad_immutable_baseline_sha(self):
  path=self.root/(guard.DIRECTORY+'baseline-core-digests.json');path.write_bytes(path.read_bytes()+b' ');self.reject_effective()
 def test_bad_caller_baseline_digest(self):
  self.baseline[next(iter(self.baseline))]='0'*64;self.reject_effective()
 def test_bad_to_digest(self):
  self.receipt['changes'][0]['to_digest']='0'*64;self.write_receipt();self.reject_effective()
 def test_unlisted_core_mutation(self):
  approved={c['key'] for c in self.receipt['changes']}
  key=next(k for k in self.baseline if k not in approved)
  self.current[key]['display_name']+=' unreviewed';self.reject_effective()
 def test_unrelated_core_field_rejected_after_resealed_signature(self):
  self.semantic_mutant(lambda c:c.update(requirements={'min_level':999999}))
 def test_source_owner_rejected_after_resealed_signature(self):
  self.semantic_mutant(lambda c:c['source_refs']['quest'].update(key='canary:quest/unrelated_owner'))
 def test_source_donor_revision_rejected_after_resealed_signature(self):
  self.semantic_mutant(lambda c:c['source_refs']['quest'].update(revision='unreviewed-cut'))
 def test_native_admission_rejected_after_resealed_signature(self):
  self.semantic_mutant(lambda c:c.update(native_lowering={'state':'READY','canonical_progress_refs':['fake']}))
 def test_recipe_bytes_cannot_change(self):
  path=self.root/(guard.DIRECTORY+'recipes.json');path.write_bytes(path.read_bytes()+b' ');self.reject_effective()
 def test_proof_or_compiler_bytes_cannot_change(self):
  for field in ('proof_inputs','compiler_inputs'):
   with self.subTest(field=field):
    path=self.root/self.receipt[field][0]['path'];old=path.read_bytes();path.write_bytes(old+b' ');self.reject_effective();path.write_bytes(old)
 def test_false_native_receipt_flag_rejects_true(self):
  self.receipt['native_runtime_admission']=True;self.write_receipt();self.reject_effective()
if __name__=='__main__':unittest.main()
