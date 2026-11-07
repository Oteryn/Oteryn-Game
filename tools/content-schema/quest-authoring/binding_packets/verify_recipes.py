import os
import json,pathlib,hashlib
D=pathlib.Path(os.environ.get('QUEST_BINDING_OUT', pathlib.Path(__file__).resolve().parents[1]/'samples/binding_packets'))/'recipes';ROOT=pathlib.Path(os.environ.get('QUEST_BINDING_ROOT', pathlib.Path(__file__).resolve().parents[4]));x=json.loads((D/'qualification.json').read_text());rows=x['records'];assert len(rows)==310 and len({r['canonical_key'] for r in rows})==310
for p in x['inputs']:assert hashlib.sha256((ROOT/p['path']).read_bytes()).hexdigest()==p['sha256']
for r in rows:
 keys=[s['key'] for s in r['stages']];assert len(keys)==len(set(keys))
 for i,s in enumerate(r['stages']):
  assert s['next']==([keys[i+1]] if i+1<len(keys) else [])
  assert s['source_action_equivalence']=='NOT_PROVED'
  assert s['has_entity_anchor']==any(t['status']=='MENTION_ANCHORED' for t in s['target_evidence'])
  for t in s['target_evidence']:
   assert t['behavior_binding']=='NOT_PROVED'
   if t['status']=='MENTION_ANCHORED':assert t['source_fact_matches'] or t['wiki_entity_mentions']
   for f in t['source_fact_matches']:assert f['evidence'] and f['fact_value'] is not None
 assert r['qualification']['source_action_equivalence'] is False
 assert r['qualification']['native_binding_complete'] is False
assert x['summary']['source_fidelity_resolved']==0 and x['summary']['runtime_admitted']==0
print('PASS 310 identities, stage connectivity, source input hashes, honest proof semantics')
compact=json.loads((D/'supplementary-index.json').read_text());cache={};references=0

def walk(v):
 global references
 if isinstance(v,dict):
  if 'json_pointer' in v:
   path=v['path'];payload=cache.setdefault(path,json.loads((ROOT/path).read_text())) if path not in cache else cache[path]
   for part in v['json_pointer'].split('/')[1:]:payload=payload[int(part)] if isinstance(payload,list) else payload[part.replace('~1','/').replace('~0','~')]
   references+=1
   if 'identity' in v:
    ident=payload['declaration']['identity'] if 'declaration' in payload else {'key':payload['target']};assert ident['key']==v['identity']['key'];assert v['native_binding_admitted'] is False
   if 'citation_id' in v:assert v['citation_id'][7:] in compact['citations']
  for z in v.values():walk(z)
 elif isinstance(v,list):
  for z in v:walk(z)
walk(compact['records']);print('PASS',references,'exact JSON pointers and identity candidate referents')
