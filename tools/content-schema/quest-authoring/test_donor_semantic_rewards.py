import os,sys,pathlib,tempfile,unittest,copy,json
path=os.environ.get('QUEST_AUTHORING_DIR')
if path:sys.path.insert(0,path)
else:
 for p in pathlib.Path(__file__).resolve().parents:
  if (p/'ots_interactions.py').exists():sys.path.insert(0,str(p));break
import ots_interactions as b
from donor_semantic_rewards import apply

class RewardRecordTests(unittest.TestCase):
 def convert(self,extra='',prefix=None,args='reward.itemid, reward.count'):
  prefix=prefix or 'local rewards = {[1002]={itemid=3389,count=1},[1003]={itemid=8077,count=2}}'
  text=prefix+'\nlocal action=Action()\nfunction action.onUse(player,item,fromPosition,target,toPosition)\nlocal reward = rewards[item.uid]\n'+extra+'\nplayer:addItem('+args+')\nend\naction:uid(1002,1003)\naction:register()\n'
  with tempfile.TemporaryDirectory()as tmp:
   repo=pathlib.Path(tmp);(repo/'fixture.lua').write_text(text);s=b.Script('canary',repo,'fixture.lua',{},'fixture');g=s.interactions()[0];return g,apply(s,g)
 def test_two_exact_record_choices_preserve_fallback(self):
  old,(new,changes)=self.convert();self.assertEqual(len(changes),1);child=new['rules'][0];self.assertEqual([r['then'][0]['count']for r in child['branch']],[1,2]);self.assertEqual(child['otherwise'],[old['rules'][0]])
 def test_duplicate_uid_and_record_field_fail_closed(self):
  for prefix in ['local rewards={[1002]={itemid=3389,count=1},[1002]={itemid=8077,count=2}}','local rewards={[1002]={itemid=3389,itemid=8077,count=1}}']:
   self.assertEqual(self.convert(prefix=prefix)[1][1],[])
 def test_alias_root_escape_mutation_shadow_environment_reject(self):
  for extra in ['evil(reward)','reward.itemid=8077','reward.itemid, other = 8077, 1','rewards[1002].itemid=8077','rewards[1002].itemid, other = 8077,1','evil(rewards)','local other=reward','local reward=other','reward=other','player=item','item=target','evil(item)','item:transform(1003)','item:unknownMutation()','setmetatable(rewards,{})','rawset(rewards,1002,{})']:
   with self.subTest(extra=extra):self.assertEqual(self.convert(extra)[1][1],[])
 def test_dynamic_invalid_quantity_or_item_never_guessed(self):
  for prefix in ['local rewards={[1002]={itemid=pick(),count=1}}','local rewards={[1002]={itemid=3389,count=false}}','local rewards={[1002]={itemid=3389,count=0}}','local rewards={[1002]={itemid=3389,count=1.5}}','local rewards={[1002]={itemid="helmet",count=1}}']:
   with self.subTest(prefix=prefix):self.assertEqual(self.convert(prefix=prefix)[1][1],[])
 def test_non_two_argument_and_random_quantity_reject(self):
  for args in ['reward.itemid','reward.itemid,1,true','reward.itemid,math.random(5)','reward.itemid,reward.count or 1']:
   self.assertEqual(self.convert(args=args)[1][1],[])
 def test_fixed_literal_count_preserves_record_ids(self):
  old,(new,changes)=self.convert(args='reward.itemid, 3');self.assertEqual([x['then'][0]['count']for x in new['rules'][0]['branch']],[3,3])
 def test_every_other_outer_guard_and_effect_is_preserved(self):
  old,(new,changes)=self.convert('if player:getLevel() < 10 then\nreturn true\nend');c=changes[0];ptr=c['baseline_child_pointer'].split('/')[1:];target=new
  for p in ptr[:-1]:target=target[int(p)]if isinstance(target,list)else target[p]
  target[int(ptr[-1])]=copy.deepcopy(target[int(ptr[-1])]['otherwise'][0]);self.assertEqual(new,old)


class RewardPacketQualificationTests(unittest.TestCase):
 def test_portable_qualifier_rejects_wrong_item_count_and_selector(self):
  import hashlib,subprocess
  from donor_semantic_rewards import apply
  sha=lambda raw:hashlib.sha256(raw).hexdigest()
  with tempfile.TemporaryDirectory()as tmp:
   root=pathlib.Path(tmp);repo=root/'repo';repo.mkdir();text='local rewards = {[1002]={itemid=3389,count=1}}\nlocal action=Action()\nfunction action.onUse(player,item,fromPosition,target,toPosition)\nlocal reward = rewards[item.uid]\nplayer:addItem(reward.itemid, reward.count)\nend\naction:uid(1002)\naction:register()\n';blob=text.encode();(repo/'fixture.lua').write_bytes(blob)
   script=b.Script('canary',repo,'fixture.lua',{},'fixture');old={k:v for k,v in script.interactions()[0].items()if k not in ('script','callback_line')};new,changes=apply(script,old);c=changes[0];lines=text.splitlines(keepends=True)
   for field,n,raw in [('source_table_span',1,lines[0].rstrip('\n')),('source_alias_span',4,lines[3]),('source_call_span',5,lines[4])]:
    start=len(''.join(lines[:n-1]).encode());c[field]={'byte_start':start,'byte_end':start+len(raw.encode()),'raw_source':raw,'sha256':sha(raw.encode())}
   c['baseline_json_pointer']='/interactions/0'+c['baseline_child_pointer']
   authoring=root/'authoring';authoring.mkdir()
   for py in pathlib.Path(b.__file__).parent.glob('*.py'):(authoring/py.name).symlink_to(py)
   samples=authoring/'samples/interactions';samples.mkdir(parents=True);baseline=samples/'interactions.json';baseline.write_text(json.dumps({'interactions':[old]}));(authoring/'interaction.schema.json').symlink_to(pathlib.Path(b.__file__).parent/'interaction.schema.json')
   row={'source':'canary','repository':'fixture','revision':'1'*40,'path':'fixture.lua','git_blob_sha1':hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest(),'sha256':sha(blob),'cache_path':'repo/fixture.lua'};manifest=root/'corpus.json';manifest.write_text(json.dumps({'files':[row]}))
   record={'source':{k:row[k]for k in ('source','repository','revision','path','git_blob_sha1','sha256')},'chosen_source':'canary','interaction':old['identity'],'baseline_graph_sha256':sha(json.dumps(old,sort_keys=True,separators=(',',':')).encode()),'baseline_graph':old,'candidate_graph':new,'changes':changes,'native_admission':False,'full_source_complete':False}
   packet={'schema':'OTERYN_DONOR_REWARD_RECORD_SEMANTIC_SUPPLEMENT/v1','scope':'fixture','source_manifest_sha256':sha(manifest.read_bytes()),'baseline_interactions_sha256':sha(baseline.read_bytes()),'native_semantic_admission':False,'full_source_complete':False,'records':[record],'summary':{}}
   here=pathlib.Path(__file__).parent;qualifier=next((here/n for n in ('qualify.py','quest_donor_reward_qualify.py')if(here/n).exists()))
   def check(p):
    f=root/'packet.json';f.write_text(json.dumps(p));return subprocess.run([sys.executable,str(qualifier),'--corpus-manifest',str(manifest),'--authoring',str(authoring),'--packet',str(f),'--out',str(root/'receipt.json')],capture_output=True)
   result=check(packet);self.assertEqual(result.returncode,0,result.stderr.decode())
   for kind in ('item','count','selector'):
    mutant=copy.deepcopy(packet);branch=mutant['records'][0]['candidate_graph']['rules'][0]['branch'][0]
    if kind=='item':branch['then'][0]['item']['key']='canary:item/8077'
    elif kind=='count':branch['then'][0]['count']=2
    else:branch['when']['object']['value']=1003
    with self.subTest(kind=kind):self.assertNotEqual(check(mutant).returncode,0)

if __name__=='__main__':unittest.main()
