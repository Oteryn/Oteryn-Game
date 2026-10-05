import os
import sys,json,pathlib,hashlib,re,collections
ROOT=pathlib.Path(os.environ.get('QUEST_BINDING_ROOT', pathlib.Path(__file__).resolve().parents[4])); HERE=ROOT/'tools/content-schema/quest-authoring';OUT=pathlib.Path(os.environ.get('QUEST_BINDING_OUT', pathlib.Path(__file__).resolve().parents[1]/'samples/binding_packets'))/'recipes';sys.path.insert(0,str(HERE))
from quest_recipe_refinements import apply_refinements
from quest_recipe_followup import apply as apply_followup
from quest_completion_authoring import validate_journey
from jsonschema import Draft202012Validator

def read(p):return json.loads((ROOT/p).read_text())
def norm(t):return ' '.join(re.findall(r'[a-z0-9]+',str(t).lower()))
def sha(p):return hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
def leaves(v):
 if isinstance(v,str):yield v
 elif isinstance(v,dict):
  for z in v.values():yield from leaves(z)
 elif isinstance(v,list):
  for z in v:yield from leaves(z)

def hits(target,value):
 n=norm(target)
 return n and any(n==norm(s) or (len(n.split())>=2 and (' '+n+' ') in (' '+norm(s)+' ')) for s in leaves(value))

def wikientities(v,path='',ev=None):
 if isinstance(v,dict):
  ev=v.get('evidence',ev)
  if isinstance(v.get('entity'),str):yield {'entity':v['entity'],'path':path,'evidence':ev,'kind':v.get('kind'),'quantity':v.get('quantity')}
  for k,z in v.items():yield from wikientities(z,path+'/'+k,ev)
 elif isinstance(v,list):
  for i,z in enumerate(v):yield from wikientities(z,path+'/'+str(i),ev)

comp=read('tools/content-schema/quest-authoring/samples/completion242/recipes.json'); refined,_=apply_refinements(ROOT,comp)
refined,followup=apply_followup(ROOT,refined)
for row in refined:
 if row['identity']['key']=='oteryn:quest.falconer_outfits_quest':row['recipe']['source_notes'].append('Chosen stage s1 classification corrected from talk to use for Task Board; local evidence: '+followup['path']+' SHA256='+followup['sha256']+'. Original SOURCE holds and Native non-admission are preserved.')
auth=read('tools/content-schema/quest-authoring/samples/authored68/recipes.json')['recipes']
wiki={x['wiki_title']:x for x in read('tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json')['entries']}
enrich={x['canonical_key']:x for x in read('tools/content-schema/quest-authoring/samples/enrichment242/enrichment.json')['records']}
idx=read('content/quests/definitions/index.json');definitions=[r['definition'] for p in idx['shards'] for r in read(p)['records']]; bytitle={r['display_name']:r for r in definitions if r.get('definition_profile')=='authored_recipe_v1' or 'authored.' in r['identity']['key']}
# Actual declared identities are candidates, never execute-event bindings.
alias={};
for x in read('content/items/aliases.json')['entries']:
 if x['state']=='ALIAS':alias.setdefault(norm(x['key'].split('.')[-1]),[]).append(x)
npcs={}
for f in (ROOT/'content/npcs/definitions').glob('npcs-*.json'):
 for x in json.loads(f.read_text())['records']:
  ident=x['declaration']['identity'];npcs.setdefault(norm(ident['key'].split('.')[-1]),[]).append({'identity':ident,'path':str(f.relative_to(ROOT)),'source_bindings':x.get('source_bindings',[])})

def identity_candidates(t):
 n=norm(t);out=[]
 for x in alias.get(n,[]):out.append({'family':'Item','key':x['target'],'alias_key':x['key'],'status':'EXACT_ALIAS_SUFFIX_NAME_CANDIDATE_ONLY','evidence':x['evidence'],'path':'content/items/aliases.json'})
 for x in npcs.get(n,[]):out.append(dict(family='NPC',status='EXACT_NPC_IDENTITY_SUFFIX_NAME_CANDIDATE_ONLY',**x))
 return out
rows=[]
for isauth,row in [(False,r) for r in refined]+[(True,r) for r in auth]:
 recipe=row if isauth else row['recipe'];title=recipe['wiki_title']; key=bytitle[title]['identity']['key'] if isauth else row['identity']['key']; validate_journey(recipe)
 if not isauth:Draft202012Validator(read('tools/content-schema/quest-authoring/quest_completion.schema.json')).validate(row)
 wr=wiki.get(title,{});we=list(wikientities(wr.get('source_specification',[])));facts=enrich.get(key,{}).get('facts',[])
 def refs(t):
  ef=[{'kind':f['kind'],'fact_index':i,'fact_value':f['value'],'evidence':f['evidence'],'scope':f['scope']} for i,f in enumerate(facts) if hits(t,f['value'])]
  wf=[x for x in we if norm(x['entity'])==norm(t)]
  return {'target':t,'source_fact_matches':ef,'wiki_entity_mentions':wf,'identity_candidates':identity_candidates(t),'behavior_binding':'NOT_PROVED','evidence_semantics':'ENTITY_OR_TEXT_MENTION_ONLY_NOT_ACTION_EQUIVALENCE','status':'MENTION_ANCHORED' if ef or wf else 'NO_EXACT_ENTITY_MENTION_IN_STRUCTURED_CACHE'}
 stages=[]
 for s in recipe['stages']:
  targets=[refs(t) for t in s['targets']];stages.append({'key':s['key'],'kind':s['kind'],'objective':s['objective'],'chosen_count':s['count'],'next':s['next'],'basis':s['basis'],'target_evidence':targets,'has_entity_anchor':any(t['status']=='MENTION_ANCHORED' for t in targets),'source_action_equivalence':'NOT_PROVED'})
 rewards=[dict(kind=r['kind'],chosen_count=r['count'],basis=r['basis'],**refs(r['name'])) for r in recipe['reward_intents']]
 req=recipe['requirements'];prereqs=[refs(t) for t in req['prerequisites']]
 generic=[s['key'] for s in recipe['stages'] if re.search(r'\b(complete the quest|finish the quest|follow the quest|perform the quest|collect the reward)\b',s['objective'],re.I)]
 rows.append({'canonical_key':key,'wiki_title':title,'profile':'authored68' if isauth else 'completion242','recipe_sha256':hashlib.sha256(json.dumps(recipe,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest(),'requirements':dict(req,prerequisite_evidence=prereqs),'stages':stages,'rewards':rewards,'source_refs':recipe['source_refs'],'chosen_adaptations':recipe['adaptations'],'qualification':{'finite_connected_journey':True,'explicit_requirements_repeat_rewards':True,'entity_mentions_all_nonterminal_stages':all(s['has_entity_anchor'] for s in stages[:-1]),'generic_objective_stage_keys':generic,'source_action_equivalence':False,'native_binding_complete':False,'original_source_holds_preserved':True},'new_supplement':'PER_STAGE_PREREQUISITE_REWARD_CACHE_EVIDENCE_INDEX_NOT_NEW_SOURCE_LOGIC'})
inputs=['tools/content-schema/quest-authoring/samples/completion242/recipes.json','tools/content-schema/quest-authoring/samples/refinements242/refinements.json','tools/content-schema/quest-authoring/samples/authored68/recipes.json','tools/content-schema/quest-authoring/samples/enrichment242/enrichment.json','tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json','content/items/aliases.json']
inputs += idx['shards'] + [str(f.relative_to(ROOT)) for f in sorted((ROOT/'content/npcs/definitions').glob('npcs-*.json'))]
summary={'recipes':len(rows),'stage_count':sum(len(r['stages']) for r in rows),'entity_anchored_stages':sum(s['has_entity_anchor'] for r in rows for s in r['stages']),'all_nonterminal_stage_entity_anchored':sum(r['qualification']['entity_mentions_all_nonterminal_stages'] for r in rows),'generic_objective_quests':sum(bool(r['qualification']['generic_objective_stage_keys']) for r in rows),'reward_count':sum(len(r['rewards']) for r in rows),'reward_entity_anchored':sum(x['status']=='MENTION_ANCHORED' for r in rows for x in r['rewards']),'stage_identity_candidates':sum(bool(t['identity_candidates']) for r in rows for s in r['stages'] for t in s['target_evidence']),'runtime_admitted':0,'source_fidelity_resolved':0}
packet={'schema':'OTERYN_QUEST_RECIPE_CACHE_EVIDENCE_AUDIT/v1','classification':'RESEARCH_HANDOFF_NOT_CANONICAL_ADMISSION','inputs':[{'path':p,'sha256':sha(p)} for p in inputs],'summary':summary,'records':rows}
(OUT/'qualification.json').write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n');print(json.dumps(summary,indent=2))
