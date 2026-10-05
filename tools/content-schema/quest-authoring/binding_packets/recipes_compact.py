import os
import json,pathlib,copy,hashlib
D=pathlib.Path(os.environ.get('QUEST_BINDING_OUT', pathlib.Path(__file__).resolve().parents[1]/'samples/binding_packets'))/'recipes';ROOT=pathlib.Path(os.environ.get('QUEST_BINDING_ROOT', pathlib.Path(__file__).resolve().parents[4]));S='tools/content-schema/quest-authoring/samples/';x=json.loads((D/'qualification.json').read_text())
e=json.loads((ROOT/(S+'enrichment242/enrichment.json')).read_text());ei={r['canonical_key']:i for i,r in enumerate(e['records'])}
w=json.loads((ROOT/(S+'wiki-source-all373/source-specs-373.json')).read_text());wi={r['wiki_title']:i for i,r in enumerate(w['entries'])}
a=json.loads((ROOT/'content/items/aliases.json').read_text());ai={r['key']:i for i,r in enumerate(a['entries'])}
ni={}
for f in (ROOT/'content/npcs/definitions').glob('npcs-*.json'):
 for i,r in enumerate(json.loads(f.read_text())['records']):ni[r['declaration']['identity']['key']]=(str(f.relative_to(ROOT)),i)

def target(t,key,title):
 refs=[]
 for f in t['source_fact_matches']:
  refs.append({'path':S+'enrichment242/enrichment.json','json_pointer':f"/records/{ei[key]}/facts/{f['fact_index']}",'evidence':f['evidence'],'proof_kind':'ENTITY_OR_TEXT_MENTION_NOT_ACTION_EQUIVALENCE'})
 for q in t['wiki_entity_mentions']:
  refs.append({'path':S+'wiki-source-all373/source-specs-373.json','json_pointer':f"/entries/{wi[title]}/source_specification"+q['path'],'evidence':q['evidence'],'proof_kind':'ENTITY_REFERENCE_NOT_ACTION_EQUIVALENCE'})
 candidates=[]
 for c in t['identity_candidates']:
  if c['family']=='Item':ident={'family':'Item','key':c['key']};ptr=f"/entries/{ai[c['alias_key']]}";path='content/items/aliases.json'
  else:ident=dict(family='NPC',**c['identity']);path,i=ni[ident['key']];ptr=f'/records/{i}'
  candidates.append({'identity':ident,'path':path,'json_pointer':ptr,'qualification':'UNIQUE_NAME_CANDIDATE' if len(t['identity_candidates'])==1 else 'AMBIGUOUS_NAME_CANDIDATE','native_binding_admitted':False})
 return {'target':t['target'],'evidence_refs':refs,'identity_candidates':candidates,'evidence_status':t['status'],'source_action_equivalence':'NOT_PROVED'}
rows=[]
for r in x['records']:
 k=r['canonical_key'];t=r['wiki_title'];requirements=copy.deepcopy(r['requirements']);requirements['prerequisite_evidence']=[target(z,k,t) for z in requirements['prerequisite_evidence']]
 row={'canonical_key':k,'wiki_title':t,'profile':r['profile'],'recipe_sha256':r['recipe_sha256'],'requirements':requirements,'stages':[],'rewards':[],'selected_adaptations':r['chosen_adaptations'],'qualification':r['qualification']}
 for s in r['stages']:row['stages'].append({**{k:v for k,v in s.items() if k!='target_evidence'},'target_evidence':[target(z,k,t) for z in s['target_evidence']]})
 for z in r['rewards']:row['rewards'].append(dict(kind=z['kind'],chosen_count=z['chosen_count'],basis=z['basis'],**target(z,k,t)))
 rows.append(row)
p={'schema':'OTERYN_QUEST_RECIPE_CACHE_EVIDENCE_INDEX/v1','classification':'SUPPLEMENTARY_REFERENCE_DATA_NOT_COMPLETION_CERTIFICATE','inputs':x['inputs'],'summary':x['summary'],'records':rows}
(D/'supplementary-index.json').write_text(json.dumps(p,ensure_ascii=False,indent=2)+'\n');print('index bytes',(D/'supplementary-index.json').stat().st_size)
# Deduplicate exact span citations; every consumer still has a stable pointer into pinned input.
citations={}
def intern(v):
 if isinstance(v,dict):
  if 'evidence' in v:
   ev=v.pop('evidence')
   if ev:
    digest=hashlib.sha256(json.dumps(ev,sort_keys=True,separators=(',',':')).encode()).hexdigest();citations.setdefault(digest,ev);v['citation_id']='sha256:'+digest
  for a in list(v.values()):intern(a)
 elif isinstance(v,list):
  for a in v:intern(a)
intern(p['records']);p['citations']=citations
(D/'supplementary-index.json').write_text(json.dumps(p,ensure_ascii=False,indent=2)+'\n');print('deduplicated index bytes',(D/'supplementary-index.json').stat().st_size,'citations',len(citations))
