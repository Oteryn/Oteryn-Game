"""Chosen authored progress into accepted QuestState loader format; no event admission."""
import argparse,copy,hashlib,json,pathlib,re
BASIS='CHOSEN_OTERYN_APPROXIMATION'
OWNER_PREFIX='oteryn:quest.authored.'
PINNED_MAIN_STATE_SHA256='c82b7e6e32456527e2c90d54c28573ebde89e0db73dc926ca3a937be1a0ad649'
def encode(v):return json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def digest(v):return hashlib.sha256(encode(v).encode()).hexdigest()
def require(ok,message):
 if not ok:raise ValueError(message)
def key(k):return isinstance(k,str)and len(k.encode())<=128 and bool(re.fullmatch(r'oteryn:[A-Za-z0-9._:/-]+',k))
def fields(v,expected):require(isinstance(v,dict)and set(v)==set(expected),'Closed loader object shape')
def project(q,ref):
 require(q.get('definition_profile')=='oteryn_authored_v1','Not an authored recipe')
 owner=q['identity']['key'];require(key(owner)and owner.startswith(OWNER_PREFIX)and len(owner)>len(OWNER_PREFIX),'Authored Quest owner');recipe=q['recipe'];stages=recipe['stages'];require(bool(stages),'Empty graph')
 ids=[s['key']for s in stages];require(len(set(ids))==len(ids),'Duplicate stages')
 # Only a single explicit path is admitted. Branch/merge semantics are not guessed.
 for i,s in enumerate(stages):
  require(bool(re.fullmatch(r's[1-9][0-9]*',s['key'])),'Stage key')
  require(type(s['count'])is int and 1<=s['count']<=2**63-1,'Stage count')
  require(s['next']==([ids[i+1]]if i+1<len(stages)else []),'Unsupported graph/order')
  require(s['kind']in {'talk','kill','use','collect','explore','complete'},'Unknown event intent')
  require((s['kind']=='complete')==(i==len(stages)-1),'Completion only terminal')
 require(stages[-1]['count']==1,'Completion counter must be one')
 tail=owner.removeprefix('oteryn:quest.authored.');tracks=[];transitions=[]
 for i,s in enumerate(stages):
  track='oteryn:quest-progress/authored/'+tail+'/'+s['key'];transition='oteryn:quest-transition/authored/'+tail+'/'+s['key'];require(key(track)and key(transition),'Track/transition key budget')
  tracks.append({'key':track,'quest':owner,'initial':0,'min':0,'max':s['count'],'bounds_basis':'EXPLICIT_CHOSEN_STAGE_OCCURRENCE_COUNT','source_key':'oteryn:authored-stage/'+tail+'/'+s['key']})
  effects=[]
  if i:
   prev=tracks[-2];effects.append({'track':prev['key'],'from':{'op':'EQ','value':prev['max']},'from_exact':True,'effect':{'kind':'SET','value':prev['max']}})
  effects.append({'track':track,'from':{'op':'LT','value':s['count']},'from_exact':True,'effect':{'kind':'ADD','value':1}})
  require(len(effects)<=8,'Effect budget')
  transitions.append({'key':transition,'quest':owner,'completes':i==len(stages)-1,'effects':effects,'requested_by':None,'source':{'basis':BASIS,'chosen_stage':copy.deepcopy(s),'canonical_ref':dict(ref,definition_sha256=digest(q)),'native_dispatch_binding':None,'event_intent_only':True,'runtime_admission':False,'source_equivalence':False}})
 repeat=recipe.get('repeat',{}).get('kind');require(repeat in {'once','daily'},'Unsupported repeat')
 r={'quest':owner,'source_quest':owner,'completion':{'basis':BASIS,'state':'CHOSEN_TYPED_PROGRESS_ONLY','native_admission':False,'runtime_enabled':False,'source_equivalence':False,'event_dispatch_binding':None,'NPC_dialogue_binding':None,'reward_delivery_binding':None,'repeat_lowering':'FIRST_CYCLE_ONLY'if repeat=='once'else 'HELD_NO_CYCLE_RESET_BINDING','canonical_ref':dict(ref,definition_sha256=digest(q)),'recipe_metadata':copy.deepcopy({k:v for k,v in recipe.items()if k!='stages'}),'counter_assumption':'Explicit chosen occurrence counters: zero initially; each owning stage occurrence adds one; predecessor completion required. Event eligibility remains unresolved.'},'tracks':tracks,'transitions':transitions}
 validate_record(r);return r

def validate_record(r):
 fields(r,('quest','source_quest','completion','tracks','transitions'));require(key(r['quest'])and r['quest'].startswith(OWNER_PREFIX)and len(r['quest'])>len(OWNER_PREFIX),'Authored Quest owner')
 c=r['completion'];fields(c,('basis','state','native_admission','runtime_enabled','source_equivalence','event_dispatch_binding','NPC_dialogue_binding','reward_delivery_binding','repeat_lowering','canonical_ref','recipe_metadata','counter_assumption'))
 require(c['basis']==BASIS and c['state']=='CHOSEN_TYPED_PROGRESS_ONLY','Chosen basis')
 require(all(c[k]is False for k in ['native_admission','runtime_enabled','source_equivalence']),'No promotion')
 require(all(c[k]is None for k in ['event_dispatch_binding','NPC_dialogue_binding','reward_delivery_binding']),'No unresolved dispatch promotion')
 require(c['repeat_lowering']in {'FIRST_CYCLE_ONLY','HELD_NO_CYCLE_RESET_BINDING'},'Repeat hold')
 tracks={}
 for t in r['tracks']:
  fields(t,('key','quest','initial','min','max','bounds_basis','source_key'));require(key(t['key'])and t['quest']==r['quest']and t['key']not in tracks,'Owned unique track');require(all(type(t[k])is int for k in ['initial','min','max'])and t['min']<=t['initial']<=t['max'],'Track bounds');tracks[t['key']]=t
 names=set()
 for t in r['transitions']:
  fields(t,('key','quest','completes','effects','requested_by','source'));require(key(t['key'])and t['quest']==r['quest']and t['key']not in names,'Owned unique transition');names.add(t['key']);require(type(t['completes'])is bool and t['requested_by']is None,'No guessed NPC request');require(1<=len(t['effects'])<=8,'Effect budget')
  fields(t['source'],('basis','chosen_stage','canonical_ref','native_dispatch_binding','event_intent_only','runtime_admission','source_equivalence'));require(t['source']['native_dispatch_binding']is None and t['source']['event_intent_only']is True and t['source']['runtime_admission']is False and t['source']['source_equivalence']is False,'No dispatch promotion')
  seen=set()
  for e in t['effects']:
   fields(e,('track','from','from_exact','effect'));require(e['track']in tracks and e['track']not in seen and e['from_exact']is True,'Owned exact effect');seen.add(e['track']);fields(e['from'],('op','value'));fields(e['effect'],('kind','value'));require(e['from']['op']in {'EQ','LT'}and type(e['from']['value'])is int,'Closed comparison');require(e['effect']['kind']in {'SET','ADD'}and type(e['effect']['value'])is int,'Closed effect');require(e['effect']['kind']!='SET'or tracks[e['track']]['min']<=e['effect']['value']<=tracks[e['track']]['max'],'SET bounds')

def build(root):
 quests=[];refs=[]
 for p in sorted((pathlib.Path(root)/'content/quests/definitions').glob('quests-*.json')):
  raw=p.read_bytes();packetsha=hashlib.sha256(raw).hexdigest();path=str(p.relative_to(root));refs.append({'path':path,'sha256':packetsha})
  for i,row in enumerate(json.loads(raw)['records']):
   q=row['definition']
   if q.get('readiness')=='waiting_native_bindings':quests.append(project(q,{'path':path,'packet_sha256':packetsha,'json_pointer':f'/records/{i}/definition'}))
 quests.sort(key=lambda r:r['quest']);require(len(quests)==68 and len({q['quest']for q in quests})==68,'All authored68 accounted')
 return {'schema':'OTERYN_CHOSEN_QUEST_PROGRESS_IMPORT/v1','basis':BASIS,'native_admission':False,'runtime_enabled':False,'authoring_sources':refs,'summary':{'quests':len(quests),'tracks':sum(len(q['tracks'])for q in quests),'transitions':sum(len(q['transitions'])for q in quests),'completion_transitions':len(quests),'repeat_cycles_held':sum(q['completion']['repeat_lowering']=='HELD_NO_CYCLE_RESET_BINDING'for q in quests),'NPC_bindings':0,'event_dispatch_bindings':0,'reward_delivery_bindings':0},'quests':quests}

def validate_packet(packet):
 fields(packet,('schema','basis','native_admission','runtime_enabled','authoring_sources','summary','quests'))
 require(packet['schema']=='OTERYN_CHOSEN_QUEST_PROGRESS_IMPORT/v1' and packet['basis']==BASIS and packet['native_admission']is False and packet['runtime_enabled']is False,'Packet admission')
 require(len(packet['quests'])==68 and len({q['quest']for q in packet['quests']})==68,'All68 unique owners')
 for q in packet['quests']:validate_record(q)

def merge(source,packet):
 validate_packet(packet)
 result=copy.deepcopy(source);old={q['quest']for q in source['quests']};require(not old&{q['quest']for q in packet['quests']},'Duplicate existing owner');result['quests']=sorted(result['quests']+copy.deepcopy(packet['quests']),key=lambda q:q['quest'])
 ts=[t for q in result['quests']for t in q['transitions']];effects=[e['effect']['kind']for t in ts for e in t['effects']]
 result['counts']={'quests':len(result['quests']),'tracks':sum(len(q['tracks'])for q in result['quests']),'transitions':len(ts),'effects':{k:effects.count(k)for k in sorted(set(effects))},'inexact_from':sum(any(not e['from_exact']for e in t['effects'])for t in ts),'requested_by':sum(t['requested_by']is not None for t in ts),'completes':sum(t['completes']for t in ts),'completion':{}}
 for q in result['quests']:
  state=q['completion']if isinstance(q['completion'],str)else q['completion']['state'];result['counts']['completion'][state]=result['counts']['completion'].get(state,0)+1
 return result

def merged_candidate(root, source_path):
 source_path=pathlib.Path(source_path);raw=source_path.read_bytes();require(hashlib.sha256(raw).hexdigest()==PINNED_MAIN_STATE_SHA256,'Pinned current-main Source state drift')
 source=json.loads(raw);require(source['schema']=='OTERYN_QUEST_STATE_LOWERING/v1','Accepted loader schema')
 packet=build(root);candidate=merge(source,packet)
 candidate['authoring_sources']=copy.deepcopy(source['authoring_sources'])+copy.deepcopy(packet['authoring_sources'])
 return candidate

def main():
 p=argparse.ArgumentParser();p.add_argument('--repo-root',type=pathlib.Path,required=True);p.add_argument('--out',type=pathlib.Path,required=True);p.add_argument('--source-state',type=pathlib.Path);p.add_argument('--check',action='store_true');a=p.parse_args();v=merged_candidate(a.repo_root,a.source_state)if a.source_state else build(a.repo_root);text=json.dumps(v,ensure_ascii=False,sort_keys=True,indent=2)+'\n'
 if a.check:require(a.out.exists()and a.out.read_text()==text,'Generated candidate differs')
 else:a.out.write_text(text)
 print(v.get('summary',v.get('counts')))
if __name__=='__main__':main()
