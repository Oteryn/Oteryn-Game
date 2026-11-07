import copy,unittest
import prepare_loot_completion as helper
class CheckedLootCompletionTests(unittest.TestCase):
 def fixture(self,with_loot=True):
  source={'kind':'mediawiki','api':'https://tibia.fandom.com/api.php','title':'Loot Statistics:Example','page_id':1,'revision_id':2,'content_sha256':'a'*64}
  item={'family':'Item','key':'canary:item/3031','revision':'canary-47dfd51f'}
  old={'item':item,'min_count':1,'max_count':10,'probability_percent':1,'skip_later_same_item_after_success':False}
  monster={'creature':{'identity':{'key':'canary:creature/example','revision':'canary-47dfd51f'},'stats':{'max_health':100}},'behavior':{'attacks':[]}}
  catalog={'definitions':[copy.deepcopy(item)]};manifest={'sources':[{'repository':'opentibiabr/canary'}],'entries':[]}
  if with_loot:
   monster['loot']={'identity':{'key':'canary:loot/example','revision':'canary-47dfd51f'},'algorithm':'IndependentBernoulli','entries':[copy.deepcopy(old)]}
   manifest['entries']=[{'source_index':0,'source_file':'example.lua','source_line':1,'source_field':'loot[1]','kind':'field','status':'mapped','destination':'/monster/loot/entries/0','resolution':'Source preserved.'}]
  proposal={'kind':'REPLACE_SUPPORTED_WIKI_ESTIMATE','monster':'example','entry_index':0,'source_field':'loot[1]','before':copy.deepcopy(old),'after':{**copy.deepcopy(old),'probability_percent':62.5},'wiki_source':source,'wiki_item':'gold coin','wiki_line':1,'version':'1.0','times':10,'kills':16}
  return monster,catalog,manifest,[proposal]
 def test_before_entry_guard_rejects_drift(self):
  m,c,f,p=self.fixture();m['loot']['entries'][0]['probability_percent']=99
  with self.assertRaisesRegex(ValueError,'current loot differs'):helper.apply_loot_updates(m,c,f,p)
 def test_rejects_tampered_statistics_and_probability(self):
  m,c,f,p=self.fixture();p[0]['after']['probability_percent']=99
  with self.assertRaisesRegex(ValueError,'exact half-even'):helper.apply_loot_updates(m,c,f,p)
  m,c,f,p=self.fixture();p[0]['kills']=0
  with self.assertRaisesRegex(ValueError,'drop count/kills'):helper.apply_loot_updates(m,c,f,p)
 def test_addition_creates_missing_loot_preserving_other_fields_and_input(self):
  m,c,f,p=self.fixture(False);before=copy.deepcopy(m);p[0].update(kind='ADD_SUPPORTED_WIKI_OBSERVATION',probability_share_ids=[3031],classification='APPROVED_WIKI_ADDITION')
  out,cat,manifest,flags=helper.apply_loot_updates(m,c,f,p)
  self.assertNotIn('loot',before);self.assertEqual(out['creature']['stats'],before['creature']['stats']);self.assertEqual(out['behavior'],before['behavior'])
  self.assertEqual(out['creature']['loot']['key'],'canary:loot/example');self.assertIn('WIKI_LOOT_ESTIMATE',flags);self.assertEqual(m,before)
  self.assertEqual(out['loot']['entries'],[p[0]['after']])
 def test_sort_moves_pairing_and_keeps_unique_gate(self):
  m,c,f,p=self.fixture();second=copy.deepcopy(m['loot']['entries'][0]);second['item']['key']='canary:item/3032';second['probability_percent']=50;second['skip_later_same_item_after_success']=True
  m['loot']['entries'].append(second);f['entries'].append({**copy.deepcopy(f['entries'][0]),'source_field':'loot[2]','destination':'/monster/loot/entries/1'})
  out,cat,manifest,flags=helper.apply_loot_updates(m,c,f,p)
  self.assertEqual([e['probability_percent'] for e in out['loot']['entries']],[50,62.5]);self.assertTrue(out['loot']['entries'][0]['skip_later_same_item_after_success'])
  rows={r['source_field']:r for r in manifest['entries']};self.assertEqual(rows['loot[1]']['destination'],'/monster/loot/entries/1');self.assertEqual(rows['loot[2]']['destination'],'/monster/loot/entries/0')
 def test_exact_item_change_rejected(self):
  m,c,f,p=self.fixture();p[0]['after']['item']['key']='canary:item/1'
  with self.assertRaisesRegex(ValueError,'exact Item reference'):helper.apply_loot_updates(m,c,f,p)
 def test_duplicate_addition_rejected(self):
  m,c,f,p=self.fixture();p[0].update(kind='ADD_SUPPORTED_WIKI_OBSERVATION',probability_share_ids=[3031],classification='LOW_CONFIDENCE_WIKI_ADDITION')
  with self.assertRaisesRegex(ValueError,'duplicates an existing exact'):helper.apply_loot_updates(m,c,f,p)
if __name__=='__main__':unittest.main()
