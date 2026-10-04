import os
import json,pathlib,hashlib,collections
R=pathlib.Path(os.environ.get('QUEST_BINDING_ROOT', pathlib.Path(__file__).resolve().parents[4]));O=pathlib.Path(os.environ.get('QUEST_BINDING_OUT', pathlib.Path(__file__).resolve().parents[1]/'samples/binding_packets'))/'rewards'
def sha(p):return hashlib.sha256((R/p).read_bytes()).hexdigest()
def witness(p,ptr):return {'path':str(p),'sha256':sha(p),'json_pointer':ptr,'access':'existing_repository_file'}
def records(folder):
 for p in sorted((R/folder).glob('*.json')):
  for n,row in enumerate(json.loads(p.read_text()).get('records',[])):
   yield p.relative_to(R),n,row['definition']
items={};names=collections.defaultdict(list)
for p,n,d in records('content/items/definitions'):
 items[d['identity']['key']]=(p,n,d)
 name=d.get('semantics',{}).get('presentation',{}).get('value',{}).get('name',{}).get('value')
 if name:names[name.casefold()].append((p,n,d))
claims={d['identity']['key']:(p,n,d) for p,n,d in records('content/interactions/reward_claims')}
requirements=[];rewardonly=[];holds=[];kind=[];chosen_rewards=[]
for p,n,d in records('content/quests/definitions'):
 ident=d['identity'];ptr=f'/records/{n}/definition';recipe=d.get('oteryn_recipe',{}).get('payload',{}).get('recipe',d.get('recipe'));recipe_ptr=ptr+'/recipe' if 'recipe' in d else ptr+'/oteryn_recipe/payload/recipe'
 reqg=[x for x in d['missing_data'] if x['code']=='requirement_unknown']
 if reqg:
  requirements.append({'quest':ident,'name':d['display_name'],'unknown_original_fields':[x['field'] for x in reqg],'original_requirements':d['requirements'],'raw_wiki_fields':d['requirements_from_wiki'],'chosen_requirements':recipe['requirements'] if recipe else None,'disposition':'CHOSEN_RECIPE_ALREADY_HAS_EXPLICIT_REQUIREMENTS_SOURCE_UNKNOWN_PRESERVED','witness':witness(p,recipe_ptr+'/requirements'),'application':'Use existing recipe requirements as chosen programme admission; do not replace original Source requirements or clear historical Source holds.'})
 if any(x['code']=='source_kind_log_flag_conflict' for x in d['missing_data']):
  kind.append({'quest':ident,'name':d['display_name'],'kind':d['kind'],'shown_in_quest_log':d['shown_in_quest_log'],'disposition':'AUTOMATED_KIND_LOG_CLASSIFICATION_CONFLICT_NOT_MISSING_QUEST_CONTENT','witness':witness(p,ptr),'chosen_programme':recipe,'application':'Retain original Source kind and donor log fact; chosen authored stages supply story programme. Do not relabel original source quest solely to remove warning.'})
 if recipe:
  for j,reward in enumerate(recipe['reward_intents']):
   row={'quest':ident,'name':d['display_name'],'reward_index':j,'intent':reward,'basis':reward['basis'],'witness':witness(p,recipe_ptr+f'/reward_intents/{j}')}
   match=names.get(reward.get('name','').casefold(),[]) if reward.get('kind')=='item' else []
   if len(match)==1:
    ip,ix,it=match[0];row.update(disposition='EXACT_UNIQUE_ITEM_NAME_MATCH_REQUIRES_DELIVERY_PROFILE',proposed_ref=it['identity'],materializable=it['materializable'],stack_class=it['stack_class'],item_witness=witness(ip,f'/records/{ix}/definition'))
   else:row.update(disposition='UNRESOLVED_OR_NON_ITEM_INTENT',candidate_count=len(match))
   chosen_rewards.append(row)
 qclaims=[]
 for ref in d.get('claims',[]):
  cp,cn,cl=claims[ref['key']];chs=[]
  for placement_index,placement in enumerate(cl['placements']):
   for reward_index,reward in enumerate(placement['reward'].get('items',[])):
    ip,ix,it=items[reward['item']['key']];chs.append({'placement_index':placement_index,'reward_index':reward_index,'reward':reward,'item_materializable':it['materializable'],'item_stack_class':it['stack_class'],'item_witness':witness(ip,f'/records/{ix}/definition')})
  qclaims.append({'identity':cl['identity'],'readiness':cl['readiness'],'definition_profile':cl.get('definition_profile','plain_quantity_claim'),'data_holds':cl.get('data_holds',[]),'item_rewards':chs,'claim_witness':witness(cp,f'/records/{cn}/definition')})
  for gap in [x for x in d['missing_data'] if x.get('source_key')==cl['provenance']['pilot_key'] and x['code'] in ['claim_item_semantics_missing','claim_source_data_missing']]:
   holds.append({'quest':ident,'name':d['display_name'],'gap':gap,'claim':qclaims[-1],'disposition':'ORIGINAL_SOURCE_OR_INSTANCE_DELIVERY_HOLD_PRESERVED','chosen_recipe_exists':bool(recipe)})
 if d.get('kind')=='reward_only':rewardonly.append({'quest':ident,'name':d['display_name'],'readiness':d['readiness'],'missing_data':d['missing_data'],'requirements':d['requirements'],'chosen_requirements':recipe['requirements'] if recipe else None,'claims':qclaims,'definition_witness':witness(p,ptr),'chosen_recipe_present':bool(recipe)})
for name,data in [('requirement-choices.json',requirements),('reward-only-105.json',rewardonly),('reward-holds.json',holds),('kind-log-conflicts.json',kind),('chosen-reward-item-candidates.json',chosen_rewards)]:
 (O/name).write_text(json.dumps({'schema':'OTERYN_QUEST_COMPLETION_REWARD_AUDIT/v1','scope':'DATA_ONLY_EXISTING_REPOSITORY_EVIDENCE_NO_SOURCE_HOLD_REMOVAL','runtime_ready':False,'records':data},ensure_ascii=False,indent=2)+'\n')
summary={'requirements_unknown_quests':len(requirements),'requirements_explicit_chosen':sum(bool(r['chosen_requirements']) for r in requirements),'reward_only':len(rewardonly),'reward_only_definition_ready':sum(r['readiness']=='definition_ready' for r in rewardonly),'reward_only_chosen_recipes':sum(r['chosen_recipe_present'] for r in rewardonly),'reward_hold_occurrences':len(holds),'reward_hold_quests':len({r['quest']['key'] for r in holds}),'kind_log_conflicts':len(kind),'all_chosen_reward_intents':len(chosen_rewards),'exact_unique_item_matches':sum('proposed_ref'in r for r in chosen_rewards),'unique_item_matches_materializable':sum(r.get('materializable',False) for r in chosen_rewards),'no_new_downloads':True}
(O/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(summary)
perquest=[]
for p,n,d in records('content/quests/definitions'):
 recipe=d.get('oteryn_recipe',{}).get('payload',{}).get('recipe',d.get('recipe'));ptr=f'/records/{n}/definition';recipe_ptr=ptr+'/recipe' if 'recipe' in d else ptr+'/oteryn_recipe/payload/recipe'
 row={'quest':d['identity'],'name':d['display_name'],'witness':witness(p,ptr),'chosen_constraints':None,'claim_refs':d.get('claims',[]),'chosen_reward_bindings':[r for r in chosen_rewards if r['quest']==d['identity']],'runtime_enabled':False,'original_source_holds_preserved':True}
 if recipe:
  row['chosen_constraints']={'requirements':recipe['requirements'],'repeat':recipe['repeat'],'witness':witness(p,recipe_ptr)}
 else:
  row['source_constraints']={'requirements':d['requirements'],'repeat':'EXACT_PER_CLAIM_POLICY','witness':witness(p,ptr+'/requirements')}
 row['data_action']='USE_PREEXISTING_CHOSEN_CONSTRAINTS_AND_QUALIFIED_REWARD_ID_CANDIDATES' if recipe else 'USE_EXACT_CANONICAL_REWARD_CLAIMS'
 perquest.append(row)
(O/'perquest-352.json').write_text(json.dumps({'schema':'OTERYN_QUEST_REWARD_CONSTRAINT_SUPPLEMENT/v1','records':perquest,'qualification_scope':'Reference binding candidates and constraints only; matching name is not successful delivery proof.'},ensure_ascii=False,indent=2)+'\n')
# Qualify every witness against exact Git-tree bytes and JSON pointers.
count=0
witness_json_cache={}
witness_sha_cache={}
for obj in [requirements,rewardonly,holds,kind,chosen_rewards,perquest]:
 def walk(x):
  global count
  if isinstance(x,dict):
   if set(['path','sha256','json_pointer']).issubset(x):
    if x['path'] not in witness_sha_cache:witness_sha_cache[x['path']]=sha(pathlib.Path(x['path']));witness_json_cache[x['path']]=json.loads((R/x['path']).read_text())
    assert witness_sha_cache[x['path']]==x['sha256'];target=witness_json_cache[x['path']]
    for part in x['json_pointer'].split('/')[1:]:target=target[int(part)] if isinstance(target,list) else target[part.replace('~1','/').replace('~0','~')]
    count+=1
   for v in x.values():walk(v)
  elif isinstance(x,list):
   for v in x:walk(v)
 walk(obj)
assert len(perquest)==352
receipt={'valid':True,'quest_count':352,'qualified_witness_occurrences':count,'resolved_source_holds':0,'new_finite_binding_candidates':sum('proposed_ref' in r for r in chosen_rewards),'new_downloads':0,'source_classification':'Original SOURCE unknown/charge/fluid/carrier/classification holds preserved','new_work':'Deterministic per-quest reward and constraint supplement with exact canonical Item identities where unique names match; no claim of new source discovery or runtime admission'}
(O/'qualification.json').write_text(json.dumps(receipt,indent=2)+'\n');print(receipt)
