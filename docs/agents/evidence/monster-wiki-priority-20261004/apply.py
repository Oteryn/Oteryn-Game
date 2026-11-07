from pathlib import Path
import json,hashlib,copy
R=Path('/workspace/monster-main-reconciliation-20261004');B=Path('/tmp/monster-wiki-priority-20261004');C=B/'candidate';sealed=B/'sealed-world';sha=lambda x:hashlib.sha256(x).hexdigest();blob=lambda x:hashlib.sha1(b'blob '+str(len(x)).encode()+b'\0'+x).hexdigest()
receipt=json.loads((B/'integration-prepared.json').read_text());out={};pre={}
for p,g in receipt['files'].items():
 original=(R/p).read_bytes();assert sha(original)==g['preimage'];pre[p]=original;out[p]=(C/p).read_bytes()
for f in sealed.rglob('*.json'):
 p='content/world/'+str(f.relative_to(sealed));original=(R/p).read_bytes()
 if original!=f.read_bytes():pre.setdefault(p,original);out[p]=f.read_bytes()
native='content/creatures/definitions/spell-native-profiles.json';oldsha=sha(pre[native]);newsha=sha(out[native]);decl='content/world/definitions/declarations.json';oldblob=blob(pre[decl]);newblob=blob(out[decl]);updated=[]
# Update existing profile pins and legacy declaration provenance without changing other controls.
paths={'content/spells.manifest.json','content/manifest.json','content/content.lock.json','content/creatures/definitions/index.json'}|{str(p.relative_to(R)) for p in (R/'content').rglob('index.json')}
for p in sorted(paths):
 original=(R/p).read_bytes();b=original.replace(oldsha.encode(),newsha.encode()).replace(oldblob.encode(),newblob.encode())
 if b!=original:pre[p]=original;out[p]=b;updated.append(p)
# Only authorized canonical profiles changed, not definitions/Items/other families.
a=json.loads(pre[decl]);b=json.loads(out[decl]);assert len(a['authoring_profiles'])==len(b['authoring_profiles'])==27272
allowed={c['target']['key'] for c in receipt['changes'] if c['lane'] in ['canonical-correction','healing']}
a_profiles={tuple(x['target'].values()):x for x in a['authoring_profiles']};b_profiles={tuple(x['target'].values()):x for x in b['authoring_profiles']};assert a_profiles.keys()==b_profiles.keys()
changed=[]
for k,v in a_profiles.items():
 if v!=b_profiles[k]:assert v['target']['key'] in allowed;changed.append(v['target']['key'])
for key in a:
 if key!='authoring_profiles':assert a[key]==b[key]
# Validate family mirrors agree with actual canonical writer.
w={x['target']['key']:x['data']['profile'] for x in b['authoring_profiles'] if x['target']['family']=='Creature'}
for p,bs in out.items():
 if p.startswith('content/creatures/definitions/creatures-'):
  for v in json.loads(bs)['records']:assert v['authoring']['profile']==w[v['definition']['identity']['key']]
for p,original in pre.items():assert (R/p).read_bytes()==original
for p,original in pre.items():dest=B/'preimages'/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(original)
for p,bs in out.items():(R/p).write_bytes(bs)
receipt['applied_files']={p:{'before':sha(pre[p]),'after':sha(bs)} for p,bs in out.items()};receipt['canonical_changed_actors']=changed;receipt['metadata_updated']=updated;receipt['state']='LOCAL_DATA_APPLIED_PENDING_NATIVE_VALIDATION';(B/'integration-applied.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'files':len(out),'canonical_actors':changed,'native_sha':newsha,'metadata':updated}))
