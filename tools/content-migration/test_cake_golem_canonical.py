"""Portable exact-source canonical-equivalence tests; no canonical World writes."""
import copy,json,sys,unittest,tempfile
from unittest.mock import patch
from pathlib import Path
# Own packet directory precedes dependency directory; when mounted, normal import
# resolves the same owning module and checked-in stage file.
sys.path.insert(0,str(Path(__file__).resolve().parent))
import cake_golem as m
class CakeCanonical(unittest.TestCase):
 def setUp(self):
  self.stage=json.loads(m.INPUT.read_text())
  self.source=next(p for p in self.stage['profiles'] if p['target']['key']=='oteryn:creature.cake_golem')
 def test_all_thirteen_profiles_are_idempotent_and_preserve_source(self):
  source=copy.deepcopy(self.stage);rows=[]
  for p in self.stage['profiles']:m.append_profile(rows,p)
  first=copy.deepcopy(rows)
  for p in self.stage['profiles']:m.append_profile(rows,p)
  self.assertEqual(rows,first);self.assertEqual(len(rows),13);self.assertEqual(self.stage,source)
 def test_only_two_serde_defaults_are_elided(self):
  canonical=m.canonical_profile(self.source);expected=copy.deepcopy(self.source)
  del expected['data']['profile']['immunities'];del expected['data']['profile']['details']['reference_level']
  self.assertEqual(canonical,expected)
  self.assertEqual(canonical['data']['profile']['details']['condition_immunities'],['invisible'])
  rows=[canonical];before=copy.deepcopy(rows);m.append_profile(rows,self.source);self.assertEqual(rows,before)
 def test_explicit_null_immunities_is_not_empty_vec(self):
  row=copy.deepcopy(self.source);row['data']['profile']['immunities']=None
  with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([row],self.source)
 def test_real_immunities_cannot_be_removed(self):
  for values in [['fire'],['invisible']]:
   row=copy.deepcopy(self.source);row['data']['profile']['immunities']=values
   with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([row],self.source)
 def test_nonnull_reference_level_and_other_stats_are_strict(self):
  for field,value in [('health',445),('armor',9)]:
   row=copy.deepcopy(self.source);row['data']['profile'][field]=value
   with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([row],self.source)
  row=copy.deepcopy(self.source);row['data']['profile']['details']['reference_level']=1
  with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([row],self.source)
 def test_condition_immunities_remain_exact(self):
  row=copy.deepcopy(self.source);row['data']['profile']['details']['condition_immunities']=[]
  with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([row],self.source)
 def test_other_identity_is_not_normalized_and_duplicates_refused(self):
  row=copy.deepcopy(self.source);row['target']['key']='oteryn:creature.other'
  self.assertEqual(m.canonical_profile(row),row)
  canonical=copy.deepcopy(row);canonical['data']['profile'].pop('immunities')
  with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([canonical],row)
  with self.assertRaisesRegex(ValueError,'IDENTITY_CONFLICT'):m.append_profile([self.source,self.source],self.source)
 def test_wrong_revision_or_kind_is_not_normalized(self):
  for field,value in [('revision','definition-r2'),('family','Behavior')]:
   row=copy.deepcopy(self.source);row['target'][field]=value;self.assertEqual(m.canonical_profile(row),row)
  row=copy.deepcopy(self.source);row['data']['kind']='Behavior';self.assertEqual(m.canonical_profile(row),row)
 def test_source_revision_and_fourteen_thirteen_bounds_remain_strict(self):
  variants=[]
  for field in ['records','profiles']:
   value=copy.deepcopy(self.stage);value[field].pop();variants.append(value)
  value=copy.deepcopy(self.stage);value['source']['revision']='unqualified';variants.append(value)
  with tempfile.TemporaryDirectory() as directory:
   path=Path(directory)/'stage.json'
   for value in variants:
    path.write_text(json.dumps(value))
    with patch.object(m,'INPUT',path):
     with self.assertRaisesRegex(ValueError,'CAKE_GOLEM_STAGE_SOURCE'):m.files()
if __name__=='__main__':unittest.main(verbosity=2)
