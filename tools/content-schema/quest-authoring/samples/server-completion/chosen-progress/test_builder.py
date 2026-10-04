import copy,unittest,json,pathlib,os,hashlib
import builder as b

def quest(count=2,repeat='once'):
 return {'identity':{'key':'oteryn:quest.authored.example','revision':'authored-r1'},'readiness':'waiting_native_bindings','definition_profile':'oteryn_authored_v1','recipe':{'requirements':{'min_level':8},'repeat':{'kind':repeat},'reward_intents':[{'kind':'xp','count':50}],'stages':[{'key':'s1','kind':'talk','count':count,'targets':['NPC'],'next':['s2']},{'key':'s2','kind':'complete','count':1,'targets':['NPC'],'next':[]}]}}

def apply(record,stage,values):
 t=next(t for t in record['transitions']if t['source']['chosen_stage']['key']==stage)
 nextvalues=dict(values)
 for e in t['effects']:
  value=values[e['track']];f=e['from'];ok=value==f['value']if f['op']=='EQ'else value<f['value']
  if not ok:return None
  effect=e['effect'];nextvalues[e['track']]=effect['value']if effect['kind']=='SET'else value+effect['value']
 return nextvalues,t['completes']

class Tests(unittest.TestCase):
 def test_order_count_and_single_completion(self):
  r=b.project(quest(),{'path':'a','json_pointer':'/records/0/definition','packet_sha256':'a'*64});v={t['key']:0 for t in r['tracks']}
  self.assertIsNone(apply(r,'s2',v));v,done=apply(r,'s1',v);self.assertFalse(done);self.assertIsNone(apply(r,'s2',v));v,done=apply(r,'s1',v);self.assertIsNone(apply(r,'s1',v));v,done=apply(r,'s2',v);self.assertTrue(done);self.assertIsNone(apply(r,'s2',v))
 def test_repeat_is_preserved_held_not_reset(self):
  r=b.project(quest(repeat='daily'),{});self.assertEqual(r['completion']['repeat_lowering'],'HELD_NO_CYCLE_RESET_BINDING');self.assertEqual(r['completion']['recipe_metadata']['repeat'],{'kind':'daily'})
 def test_reject_unsupported_graph_counts_and_keys(self):
  for mutate in [lambda q:q['recipe']['stages'][0].update(next=['s2','s1']),lambda q:q['recipe']['stages'][1].update(next=['s1']),lambda q:q['recipe']['stages'][1].update(count=2),lambda q:q['recipe']['stages'][0].update(count=True),lambda q:q['recipe']['stages'][0].update(count=0),lambda q:q['recipe']['stages'][1].update(key='s1')]:
   q=quest();mutate(q)
   with self.assertRaises(ValueError):b.project(q,{})
 def test_closed_shape_and_native_promotion_rejected(self):
  r=b.project(quest(),{});b.validate_record(r)
  for where in [r,r['tracks'][0],r['transitions'][0],r['transitions'][0]['effects'][0]]:
   where['extra']=True
   with self.assertRaises(ValueError):b.validate_record(r)
   del where['extra']
  r['completion']['native_admission']=True
  with self.assertRaises(ValueError):b.validate_record(r)
 def test_metadata_hash_preserved(self):
  q=quest();r=b.project(q,{'path':'x','packet_sha256':'0'*64,'json_pointer':'/records/2/definition'});self.assertEqual(r['completion']['canonical_ref']['definition_sha256'],b.digest(q));self.assertEqual(r['completion']['recipe_metadata'],{k:v for k,v in q['recipe'].items()if k!='stages'})
 def test_wrong_family_owner_rejected(self):
  for owner in ['oteryn:item.foo', 'oteryn:quest.other', 'oteryn:quest.authored.']:
   q=quest();q['identity']['key']=owner
   with self.assertRaises(ValueError):b.project(q,{})
class All68Tests(unittest.TestCase):
 def test_all68_projection_and_pointer_hash(self):
  root=pathlib.Path(os.environ.get('QUEST_PRODUCT_ROOT','/workspace/quest-data-completion-80-worktree'));p=b.build(root);b.validate_packet(p);self.assertEqual(p['summary']['quests'],68);self.assertEqual(p['summary']['repeat_cycles_held'],9)
  cache={}
  for q in p['quests']:
   ref=q['completion']['canonical_ref']
   if ref['path']not in cache:
    raw=(root/ref['path']).read_bytes();cache[ref['path']]=(hashlib.sha256(raw).hexdigest(),json.loads(raw))
   digest,value=cache[ref['path']];self.assertEqual(digest,ref['packet_sha256'])
   for k in ref['json_pointer'].split('/')[1:]:value=value[int(k)]if isinstance(value,list)else value[k]
   self.assertEqual(b.digest(value),ref['definition_sha256']);self.assertEqual(value['identity']['key'],q['quest'])
   values={t['key']:0 for t in q['tracks']}
   for i,t in enumerate(q['transitions']):
    stage=t['source']['chosen_stage'];goal=stage['count'];track=q['tracks'][i]['key']
    if i:self.assertIsNone(apply(q,stage['key'],{**values,q['tracks'][i-1]['key']:q['tracks'][i-1]['max']-1}))
    # Check the actual final required occurrence and overflow refusal without repeating thousands of events.
    values[track]=goal-1;values,done=apply(q,stage['key'],values);self.assertEqual(values[track],goal);self.assertEqual(done,i==len(q['transitions'])-1);self.assertIsNone(apply(q,stage['key'],values))
 def test_pinned_main_merge_and_byte_drift(self):
  import tempfile
  root=pathlib.Path(os.environ.get('QUEST_PRODUCT_ROOT','/workspace/quest-data-completion-80-worktree'));source=pathlib.Path(os.environ.get('QUEST_MAIN_STATE','/workspace/quest-next5/transitions387/main-state.json'));candidate=b.merged_candidate(root,source);self.assertEqual(candidate['counts']['tracks'],1746);self.assertEqual(candidate['counts']['transitions'],3679)
  with tempfile.TemporaryDirectory()as d:
   altered=pathlib.Path(d)/'source.json';altered.write_bytes(source.read_bytes()+b' ')
   with self.assertRaises(ValueError):b.merged_candidate(root,altered)
 def test_merge_preserves_existing_records_and_rejects_duplicate(self):
  root=pathlib.Path(os.environ.get('QUEST_PRODUCT_ROOT','/workspace/quest-data-completion-80-worktree'));packet=b.build(root);source=json.loads((root/'content/quests/missions/quest-state.json').read_text());merged=b.merge(source,packet);existing={q['quest']:q for q in merged['quests']};self.assertTrue(all(existing[q['quest']]==q for q in source['quests']));self.assertEqual(merged['counts']['quests'],len(source['quests'])+68)
  with self.assertRaises(ValueError):b.merge(merged,packet)
  packet['native_admission']=True
  with self.assertRaises(ValueError):b.merge(source,packet)
if __name__=='__main__':unittest.main()
