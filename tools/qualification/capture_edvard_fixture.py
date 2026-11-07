#!/usr/bin/env python3
"""Seal actual Edvard NPC service frames; never encode protocol or invent a map."""
import argparse,hashlib,json
from pathlib import Path

def materialize(v,root):
 s=v['source']; w=v['viewer']
 keybytes=(root/'tools/content-schema/native-gameplay/item-keys.json').read_bytes()
 keys=json.loads(keybytes)
 if keys['schema']!='OTERYN_NATIVE_ITEM_KEYS/v1':raise ValueError('qualified native Item index schema')
 rows=keys['records']
 if any(rows[i][0].encode()>=rows[i+1][0].encode() for i in range(len(rows)-1)):raise ValueError('strict ordered native Item keys')
 refs={}
 for ident in (2976,23398):
  found=[i+1 for i,row in enumerate(rows) if row[0]==f'oteryn:item.tibia.i{ident}']
  if len(found)!=1:raise ValueError('source official Item absent from qualified native keyset')
  refs[str(ident)]=found[0]
 v['qualified_item_keys_sha256']=hashlib.sha256(keybytes).hexdigest()
 v['qualified_source_item_refs']=refs
 if s['npc']!='oteryn:npc.edvard' or s['source_position']!=[33229,31832,7] or w['position']!=[33228,31832,7]:raise ValueError('actual source NPC/viewer provenance')
 if s['donor_revision']!='201189f871d23597b82caf88fd611454a2ddefb8':raise ValueError('donor pin')
 if not {3,4,6,15}.issubset(w['selected_capabilities']) or not set(w['selected_capabilities']).issubset({3,4,6,11,15,18}):raise ValueError('actual selected capability proof')
 if len(w['actor_identity'])!=16 or not w['actor_generation']:raise ValueError('viewer identity')
 snapshots={r['domain_id'] for r in v['packets'] if r['kind']=='snapshot'}
 deltas={r['domain_id'] for r in v['packets'] if r['kind']=='delta'}
 if snapshots not in ({1,7,8,9},{1,7,8,9,17}) or not {7,8,9}.issubset(deltas):raise ValueError('actual domain coverage')
 if (17 in snapshots)!=(18 in w['selected_capabilities']):raise ValueError('actual map negotiation')
 for r in v['packets']:
  for k in ('envelope_hex','payload_hex','state_delta_hex'):
   if k in r:
    b=bytes.fromhex(r[k])
    if len(b)>1048576:raise ValueError('bounded actual bytes')
    r[k+'_sha256']=hashlib.sha256(b).hexdigest()
 return (json.dumps(v,sort_keys=True,indent=2)+'\n').encode()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('output',type=Path);p.add_argument('--check',action='store_true');p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[2]);a=p.parse_args();data=materialize(json.loads(a.capture.read_bytes()),a.root)
 if a.check:
  if a.output.read_bytes()!=data:raise ValueError('actual Edvard fixture drift')
 else:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_bytes(data)
 print('actual Edvard NPC fixture '+('verified' if a.check else 'generated'))
if __name__=='__main__':main()
