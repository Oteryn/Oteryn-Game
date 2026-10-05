import copy,json,unittest,tempfile
from pathlib import Path
import jsonschema
from builder import PIN,RECIPE,build,norm,resolve
ROOT=Path(__file__).parent
class Packet(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.p=json.loads((ROOT/'packet.json').read_text());cls.s=json.loads((ROOT/'packet.schema.json').read_text())
 def test_closed_schema(self):
  jsonschema.validate(self.p,self.s);bad=copy.deepcopy(self.p);bad['records'][0]['guessed_owner']='bad'
  with self.assertRaises(jsonschema.ValidationError):jsonschema.validate(bad,self.s)
 def test_changed_recipe_payload_rejected(self):
  source=Path('/workspace/quest-data-completion-80-worktree')/RECIPE
  document=json.loads(source.read_text());document['recipes'][0]['stages'][0]['count']+=1
  with tempfile.TemporaryDirectory(dir=ROOT) as folder:
   file=Path(folder)/RECIPE;file.parent.mkdir(parents=True);file.write_text(json.dumps(document))
   with self.assertRaisesRegex(AssertionError,'canonical/local recipe payload differs'):
    build(folder,'/workspace/quest-next5/current-main','/workspace/quest-npc-68-audit/qualified-world-base/definitions/declarations.json')
 def test_no_readiness_promotion(self):
  for location in ['top','stage','reward','unchanged']:
   bad=copy.deepcopy(self.p)
   if location=='top':bad['runtime_admitted']=True
   elif location=='stage':bad['records'][0]['stages'][0]['runtime_admitted']=True
   elif location=='reward':bad['records'][0]['reward_intents'][0]['runtime_admitted']=True
   else:bad['records'][0]['native_readiness_unchanged']=False
   with self.assertRaises(jsonschema.ValidationError):jsonschema.validate(bad,self.s)
 def test_canonical_recipe_identity_payload(self):
  for q in self.p['records']:
   self.assertEqual(q['recipe_sha256'],q['canonical_recipe_sha256'])
   self.assertEqual(q['canonical_definition_evidence']['epoch'],PIN)
   self.assertEqual(q['identity_binding_basis'],'EXACT_AUTHORING_IDENTITY_AND_CANONICAL_RECIPE_PAYLOAD')
  self.assertEqual(len({(q['quest_ref']['key'],q['quest_ref']['revision']) for q in self.p['records']}),68)
 def test_full_coverage(self):
  self.assertEqual(len(self.p['records']),68);self.assertEqual(self.p['counts']['non_dialogue_stages'],337)
  self.assertEqual(sum(len(q['reward_intents']) for q in self.p['records']),148)
  self.assertFalse(self.p['runtime_admitted']);self.assertTrue(all(q['native_readiness_unchanged'] for q in self.p['records']))
 def test_no_fuzzy_matching(self):
  t={('Item',norm('Gold Coin')):[{'ref':{'family':'Item','key':'accepted','revision':'1'}}]}
  self.assertEqual(len(resolve(t,['Item'],'GOLD COIN')),1);self.assertEqual(resolve(t,['Item'],'Gold Coins'),[])
 def test_ambiguous_rejected(self):
  t={('Item','ham'):[{'ref':{'family':'Item','key':'a','revision':'1'}},{'ref':{'family':'Item','key':'b','revision':'1'}}]}
  self.assertEqual(len(resolve(t,['Item'],'ham')),2)
  ambiguous=[t for q in self.p['records'] for s in q['stages'] for t in s['targets'] if t['status']=='AMBIGUOUS_CANONICAL_NAME']
  self.assertTrue(ambiguous);self.assertTrue(all(not t['refs'] for t in ambiguous))
 def test_exact_epoch_and_evidence(self):
  self.assertEqual(self.p['epoch'],PIN);self.assertTrue(all(x['epoch']==PIN for x in self.p['canonical_inputs']))
  rows=[t for q in self.p['records'] for s in q['stages'] for t in s['targets']]+[r['mapping'] for q in self.p['records'] for r in q['reward_intents']]
  for row in rows:
   if row['status']=='EXACT_CANONICAL_IDENTITY_ASSOCIATION':
    self.assertEqual(len(row['refs']),1);self.assertTrue(row['evidence']);self.assertTrue(all(x['epoch']==PIN for x in row['evidence']))
 def test_addon_explicit_only(self):
  rr=[r for q in self.p['records'] for r in q['reward_intents'] if r['kind']=='outfit'];self.assertEqual(len(rr),22)
  self.assertTrue(all(r['mapping']['refs'] for r in rr))
  for r in rr:
   if 'requested_addon_index'in r['mapping']:
    self.assertTrue(r['mapping']['chosen_interpretation_only']);self.assertIn(r['mapping']['requested_addon_index'],[None,1,2])
   self.assertIn('BASE_OUTFIT_OR_ADDON_DELIVERY_REQUIRES_EXPLICIT_NATIVE_POLICY',r['unresolved'])
 def test_seams_not_invented(self):
  self.assertEqual(self.p['counts']['encounter_outcome_seams'],0)
  for q in self.p['records']:
   for s in q['stages']:
    self.assertFalse(s['runtime_admitted']);self.assertTrue(s['unresolved'])
if __name__=='__main__':unittest.main()
