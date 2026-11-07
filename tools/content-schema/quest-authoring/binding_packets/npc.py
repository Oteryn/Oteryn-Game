import os
import json,pathlib,re,hashlib,collections
ROOT=pathlib.Path(os.environ.get('QUEST_BINDING_ROOT', pathlib.Path(__file__).resolve().parents[4])); OUT=pathlib.Path(os.environ.get('QUEST_BINDING_OUT', pathlib.Path(__file__).resolve().parents[1]/'samples/binding_packets'))/'npc'; inputs={}
def read(p):
 b=(ROOT/p).read_bytes();inputs[p]=hashlib.sha256(b).hexdigest();return json.loads(b)
def records(p):
 idx=read(p);return [r for s in idx['shards'] for r in read(s)['records']]
def slug(s):return re.sub(r'[^a-z0-9]+','_',s.lower()).strip('_')
npcs=records('content/npcs/definitions/index.json'); dialogues=records('content/dialogues/definitions/index.json');quests=records('content/quests/definitions/index.json'); texts=read('tools/content-schema/quest-authoring/samples/source_texts/source_texts.json')['texts']
npckey={r['declaration']['identity']['key']:r for r in npcs};dialoguekey={r['declaration']['identity']['key']:r for r in dialogues}; byexternal=collections.defaultdict(list)
for n in npcs:
 for b in n['source_bindings']:
  if b['identity_namespace'].endswith('/npc-file'):byexternal[(b['identity_namespace'],b['external_id'])].append((n,b))
def npc_ref(n,b=None):
 d=n['declaration'];dr=d.get('dialogue');return {'npc':d['identity'],'dialogue':dr,'dialogue_record_present':bool(dr and dr['key'] in dialoguekey),'source_identity_binding':b,'executable_transition_resolved':False}
def source_link(o):
 p=o.get('path','');stem=pathlib.PurePosixPath(p).stem; ns=('canary' if o.get('source')=='canary' else 'crystalserver')+'/npc-file'; found=byexternal.get((ns,stem),[])
 if len(found)==1:
  n,b=found[0];r=npc_ref(n,b);r.update({'classification':'CANONICAL_NPC_IDENTITY_CANDIDATE_FOR_DONOR_FILE','identity_only':True,'source_revision_equal':b['source_revision']==o.get('revision'),'evidence':o});return r
 candidates=[]
 if stem.endswith('_functions'):
  for n,b in byexternal.get((ns,stem[:-10]),[]):candidates.append(npc_ref(n,b))
 return {'classification':'UNRESOLVED_SOURCE_FILE_IDENTITY','source_file':p,'source_revision':o.get('revision'),'candidate_helper_owner':candidates,'evidence':o,'executable_transition_resolved':False}
out=[]
for rec in quests:
 d=rec['definition'];row={'quest':d['identity'],'display_name':d['display_name'],'definition_profile':d.get('definition_profile','source'),'source_progress_npc_links':[],'recipe_stage_npc_links':[],'remaining_holds':[]}
 for pr in d.get('source_data',{}).get('progress',[]):
  for tr in pr.get('transitions',[]):
   for o in tr.get('source_occurrences',[]):
    if '/npc/' in o.get('path',''):
     link=source_link(o);link.update({'progress_key':pr['key'],'transition_key':tr['key'],'dialogue_guard':o.get('dialogue'),'requested_write':o.get('write')});row['source_progress_npc_links'].append(link)
 recipe=d.get('recipe') or d.get('oteryn_recipe',{}).get('payload',{}).get('recipe',{})
 for st in recipe.get('stages',[]):
  if st.get('kind') not in ['talk','deliver']:continue
  for target in st.get('targets',[]):
   key='oteryn:npc.'+slug(target);n=npckey.get(key)
   link={'stage_key':st.get('key'),'target_name':target,'classification':'CANONICAL_NPC_IDENTITY_CANDIDATE_FOR_AUTHORED_STAGE' if n else 'NO_EXACT_CANONICAL_NAME_IDENTITY','basis':'CHOSEN_OTERYN_RECIPE; name-resolution only, not donor callback proof','stage':st,'executable_transition_resolved':False}
   if n:link.update(npc_ref(n));link['candidate_for_native_binding']=True
   if target=='Pig (NPC)' and 'oteryn:npc.pig' in npckey:
    link['additional_identity_candidates']=[npc_ref(npckey['oteryn:npc.pig'])]
    link['candidate_basis']='Explicit NPC disambiguator stripped; existing donor npc-file pig. Still requires chosen binding admission.'
   if target=='Kesar the Younger':
    link['additional_identity_candidates']=[npc_ref(npckey[k]) for k in ['oteryn:npc.kesar_the_younger_day','oteryn:npc.kesar_the_younger_night'] if k in npckey]
    link['candidate_basis']='Existing Crystal day/night identities; recipe phase not fixed, no single identity selected.'
   link['target_requires_npc_identity']=(st.get('kind')=='talk')
   row['recipe_stage_npc_links'].append(link)
 if any(x['classification']=='UNRESOLVED_SOURCE_FILE_IDENTITY' for x in row['source_progress_npc_links']):row['remaining_holds'].append('source NPC helper/file identity requires owner evidence')
 if any(x['classification']=='NO_EXACT_CANONICAL_NAME_IDENTITY' and x['target_requires_npc_identity'] for x in row['recipe_stage_npc_links']):row['remaining_holds'].append('authored talk/delivery targets absent exact NPC identity or not NPC targets')
 if row['source_progress_npc_links'] or row['recipe_stage_npc_links']:row['remaining_holds'].append('conditional conversation branch and executable quest transition not established by identity presence')
 out.append(row)
summary={'quests':len(out),'quests_with_source_npc_occurrences':sum(bool(r['source_progress_npc_links']) for r in out),'source_npc_occurrences':sum(len(r['source_progress_npc_links']) for r in out),'exact_source_file_identity_occurrences':sum(x['classification']=='CANONICAL_NPC_IDENTITY_CANDIDATE_FOR_DONOR_FILE' for r in out for x in r['source_progress_npc_links']),'recipe_npc_target_mentions':sum(len(r['recipe_stage_npc_links']) for r in out),'exact_authored_name_matches':sum(x['classification']=='CANONICAL_NPC_IDENTITY_CANDIDATE_FOR_AUTHORED_STAGE' for r in out for x in r['recipe_stage_npc_links']),'authored68':sum(r['definition_profile']=='oteryn_authored_v1' for r in out),'runtime_bindings_created':0,'downloaded_sources':0}
keyword_count=0
for row in out:
 for link in row['source_progress_npc_links']:
  ref=link.get('dialogue'); d=dialoguekey.get(ref['key'],{}).get('declaration') if ref else None
  kw=(link.get('dialogue_guard') or {}).get('keywords',[])
  matches=[]
  if d:
   for k in d.get('keywords',[]):
    if set(kw)&set(k.get('triggers',[])):matches.append({'keyword_key':k['key'],'triggers':k['triggers'],'reply':k['reply']})
  link['existing_canonical_keyword_entries']=matches;link['keyword_match_certifies_topic_or_branch']=False
  keyword_count+=bool(matches)
 for link in row['recipe_stage_npc_links']:
  ref=link.get('dialogue'); d=dialoguekey.get(ref['key'],{}).get('declaration') if ref else None
  if d:link['existing_dialogue_summary']={'greet_lines':len(d.get('greet',[])),'keyword_entries':len(d.get('keywords',[])),'stage_branch_verified':False}
authored=[r for r in out if r['definition_profile']=='oteryn_authored_v1']
summary.update({'source_occurrences_existing_keyword_reply':keyword_count,'authored68_npc_identity_candidates':sum(x['classification']=='CANONICAL_NPC_IDENTITY_CANDIDATE_FOR_AUTHORED_STAGE' for r in authored for x in r['recipe_stage_npc_links']),'authored68_talk_targets_without_exact_npc':sum(x['target_requires_npc_identity'] and x['classification']=='NO_EXACT_CANONICAL_NAME_IDENTITY' for r in authored for x in r['recipe_stage_npc_links']),'donor_identity_candidates_same_revision':sum(x.get('source_revision_equal',False) for r in out for x in r['source_progress_npc_links'])})
result={'schema':'OTERYN_QUEST_NPC_DIALOGUE_LINKAGE_AUDIT/v1','scope':'read-only identity candidates, no Native admission or runtime certification','input_sha256':inputs,'summary':summary,'quests':out}
(OUT/'quest-npc-dialogue-links.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
(OUT/'authored68-npc-links.json').write_text(json.dumps({'summary':summary,'quests':authored},ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,indent=2))
