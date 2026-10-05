from pathlib import Path
import json,hashlib,copy,os,collections
R=Path('/workspace/monster-main-reconciliation-20261004');B=Path('/tmp/monster-wiki-priority-20261004');S=B/'candidate';S.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest(); load=lambda p:json.loads(p.read_bytes()); compact=lambda x:(json.dumps(x,ensure_ascii=False,separators=(',',':'))+'\n').encode()
docs={}; before={}; changes=[]
def doc(p):
 if p not in docs:before[p]=(R/p).read_bytes();docs[p]=json.loads(before[p])
 return docs[p]
def guarded(v,f,r):
 assert (f in v)==r['old_present'] and v.get(f)==r['old'],(f,r.get('target'),v.get(f),r['old'])
 if r['new_present']:v[f]=copy.deepcopy(r['new'])
 else:v.pop(f,None)
nativepath='content/creatures/definitions/spell-native-profiles.json';native=doc(nativepath);nm={r['profile']['target']['key']:r for r in native['records']};assert len(nm)==1870
for lane in ['mitigation_a','mitigation_b','resistance_a','resistance_b','core','resistance_followup']:
 pack=load(B/lane/'proposal.json')
 for r in pack['rows']:
  v=nm[r['target']['key']]['profile']['data']['profile'];assert (r['field'] in v)==r['old_present'] and v.get(r['field'])==r['old']
  status=r.get('status',r.get('provenance'));apply=r.get('apply',status!='UNKNOWN')
  if apply and (r['old_present']!=r['new_present'] or r['old']!=r['new']):guarded(v,r['field'],r);changes.append({'target':r['target'],'field':r['field'],'lane':lane,'status':status})
  if r.get('canonical_correction'):
   correction=r['canonical_correction'];shard=doc(r['canonical_path']);record=next(x for x in shard['records'] if x['definition']['identity']==r['target']);guarded(record['authoring']['profile'],r['field'],correction)
   world=doc('content/world/definitions/declarations.json');record=next(x for x in world['authoring_profiles'] if x['target']==r['target']);guarded(record['data']['profile'],r['field'],correction);changes.append({'target':r['target'],'field':r['field'],'lane':'canonical-correction','proof':r.get('provenance')})
for r in load(B/'resistance_followup/proposal.json')['canonical_changes']:
 target=r['target'];world=doc('content/world/definitions/declarations.json');wr=next(x for x in world['authoring_profiles'] if x['target']==target)
 shardpath=next(str(p.relative_to(R)) for p in (R/'content/creatures/definitions').glob('creatures-*.json') if any(x['definition']['identity']==target for x in load(p)['records']))
 sr=next(x for x in doc(shardpath)['records'] if x['definition']['identity']==target)
 for profile in [wr['data']['profile'],sr['authoring']['profile']]:
  e=next(x for x in profile['resistances'] if x['damage_type']==r['damage_type']);assert e['percent']==r['canonical_old'];e['percent']=copy.deepcopy(r['new'])
 changes.append({'target':target,'field':'resistances.'+r['damage_type'],'lane':'canonical-correction','proof':r['proof']})
for r in load(B/'healing/proposal.json')['rows']:
 for loc in r['locations']:
  d=doc(loc['path']);assert sha(before[loc['path']])==loc['expected_file_sha256'];parts=loc['pointer'].strip('/').split('/');v=d
  for part in parts[:-1]:v=v[int(part)] if isinstance(v,list) else v[part]
  guarded(v,parts[-1],loc)
 changes.append({'target':r['target'],'field':r['field'],'lane':'healing','proof':r['evidence']})
for path,d in docs.items():
 out=S/path;out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(compact(d));bak=B/'preimages'/path;bak.parent.mkdir(parents=True,exist_ok=True);bak.write_bytes(before[path])
# Preserve all non-stat members and existing fixture rows exactly.
original=json.loads(before[nativepath]);allowed={c['target']['key'] for c in changes if c['lane']!='canonical-correction'}
for old,new in zip(original['records'],native['records']):
 assert old['profile']['target']==new['profile']['target']
 a=copy.deepcopy(old);b=copy.deepcopy(new)
 for f in ['health','experience','speed','armor','mitigation','resistances','immunities']:a['profile']['data']['profile'].pop(f,None);b['profile']['data']['profile'].pop(f,None)
 a['profile']['data']['profile']['details'].pop('healing_from_damage',None);b['profile']['data']['profile']['details'].pop('healing_from_damage',None);assert a==b
 if old['profile']['target']['key'] not in allowed:assert old==new
merged=B/'merged-world';(merged/'definitions').mkdir(parents=True,exist_ok=True);(merged/'provenance').mkdir(exist_ok=True)
(merged/'definitions/declarations.json').write_bytes((S/'content/world/definitions/declarations.json').read_bytes())
for p in ['definitions/reference.json','provenance/imports.json','provenance/sources.json']:
 dest=merged/p
 if not dest.exists():dest.symlink_to(R/'content/world'/p)
receipt={'changes':changes,'native_stat_changes':sum(c['lane'] in ['mitigation_a','mitigation_b','resistance_a','resistance_b','core'] for c in changes),'canonical_corrections':sum(c['lane']=='canonical-correction' for c in changes),'healing_actors':sum(c['lane']=='healing' for c in changes),'files':{p:{'preimage':sha(before[p]),'candidate':sha((S/p).read_bytes())} for p in docs},'runtime_activation':False}
(B/'integration-prepared.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n');print(json.dumps({k:v for k,v in receipt.items() if k not in ['changes','files']}))
