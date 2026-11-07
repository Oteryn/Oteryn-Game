#!/usr/bin/env python3
"""Seal placed House Wrap advisory metadata; this never grants mutation authority."""
import argparse, hashlib, json, sys
from pathlib import Path

def sha(b):return hashlib.sha256(b).hexdigest()
def load(p,limit=32*1024*1024):
 b=p.read_bytes()
 if len(b)>limit:raise ValueError('bounded input')
 return json.loads(b),b

def build(root,donor,capture,artifact):
 sys.path.insert(0,str(root/'tools/content-schema/native-gameplay'))
 import build_source_item_profiles as owner
 rules,rules_raw=load(root/'tools/content-schema/native-gameplay/furniture-rules.json')
 expected={x['path']:x['sha256'] for x in rules['sources']}
 xml=(donor/'data/items/items.xml').read_bytes();dat=(donor/'data/items/appearances.dat').read_bytes()
 if sha(xml)!=expected['data/items/items.xml'] or sha(dat)!=expected['data/items/appearances.dat']:raise ValueError('source pin')
 nodes=owner.xml_index(xml);objects=owner.engine_items.load_appearance_objects(dat)
 wrapped={i for lo,hi,out in rules['wrap'] if out==23398 for i in range(lo,hi+1)}
 roster=sorted(i for i in wrapped if i!=23398 and i in objects and int(owner.attrs(nodes[i]).get('wrapableto','0'))>0 and not(objects[i]['flags'].get('flags.wrap') or objects[i]['flags'].get('flags.unwrap')))
 if len(roster)!=72:raise ValueError('exact official-qualified source72 roster')
 # Preserve the one exact source-known/current-ABSENT identity, never skip arbitrary failures.
 raw_roster=sorted(i for i in wrapped if i!=23398 and i in nodes and int(owner.attrs(nodes[i]).get('wrapableto','0'))>0 and not(objects.get(i,{}).get('flags',{}).get('flags.wrap') or objects.get(i,{}).get('flags',{}).get('flags.unwrap')))
 if len(raw_roster)!=73 or sorted(set(raw_roster)-set(roster))!=[12699]:raise ValueError('exact raw73/official72 boundary')
 exported,export_raw=load(capture);source=exported.get('source',exported)
 carrier=artifact.read_bytes();generation=sha(carrier)
 if not carrier.startswith(b'OTNCW03\0') or source['client_content_generation']!=generation:raise ValueError('independent current public artifact')
 cursor=8
 for _ in range(2):
  size=int.from_bytes(carrier[cursor:cursor+4],'big');digest=carrier[cursor+4:cursor+36];cursor+=36;section=carrier[cursor:cursor+size];cursor+=size
  if not 0<size<=8*1024*1024 or len(section)!=size or hashlib.sha256(section).digest()!=digest:raise ValueError('public section integrity')
 if cursor!=len(carrier):raise ValueError('public carrier exact bound')
 keys,key_raw=load(root/'tools/content-schema/native-gameplay/item-keys.json')
 if sha(key_raw)!=source['item_keys_sha256'] or keys['schema']!='OTERYN_NATIVE_ITEM_KEYS/v1':raise ValueError('actual native key pin')
 rows=keys['records']
 if rows!=sorted(rows,key=lambda r:r[0].encode()) or len({r[0] for r in rows})!=len(rows):raise ValueError('unique ordered typed keys')
 key_index={r[0]:(n+1,r[1]) for n,r in enumerate(rows)}
 aliases,aliases_raw=load(root/'content/items/aliases.json')
 admission,admission_raw=load(root/'imports/official/appearance-membership/admitted.json')
 newest=next(r for r in admission['files'] if r['label']==admission['newest'])
 membership,member_raw=load(root/'imports/official/appearance-membership'/newest['manifest'])
 if sha(member_raw)!=newest['manifest_sha256']:raise ValueError('official membership pin')
 members={r[0]:r[1] for r in membership['entries']};refs=source['wrap_advisory_definition_refs']
 if set(refs)!=set(map(str,roster)):raise ValueError('actual index exact72 export')
 unavailable=source['unavailable_wrap_advisory_sources']
 expected_unavailable=[{'source_item_id':12699,'status':'CURRENT_OFFICIAL_IDENTITY_ABSENT','canonical_state':'RETIRED_WITHOUT_SUCCESSOR','admitted_set':'ABSENT'}]
 if unavailable!=expected_unavailable or source['raw_xml_only_wrap_source_count']!=73 or source['qualified_wrap_advisory_source_count']!=72:raise ValueError('explicit source-known ABSENT identity')
 if 12699 in objects or 12699 in members or 'oteryn:item.tibia.i12699' in key_index:raise ValueError('retired identity qualification changed; rerun owner review')
 if not any(r.get('state')=='RETIRED_WITHOUT_SUCCESSOR' and r.get('evidence',{}).get('source_item_id')==12699 and r.get('evidence',{}).get('admitted_set')=='ABSENT' for r in aliases['entries']):raise ValueError('immutable retirement evidence')
 records=[]
 for i in roster:
  key=f'oteryn:item.tibia.i{i}';reference,revision=key_index[key]
  if refs[str(i)]!=reference or i not in members:raise ValueError('actual typed reference or official membership')
  if not any(r.get('state')=='ALIAS' and r.get('target')==key and r.get('evidence',{}).get('source_item_id')==i and r.get('evidence',{}).get('identity_projection_sha256')==members[i] for r in aliases['entries']):raise ValueError('qualified canonical official identity')
  records.append({'definition_ref':reference,'appearance_id':i,'definition_key':key,'definition_revision':revision})
 records.sort(key=lambda r:r['definition_ref'])
 result={'schema':'OTERYN_HOUSE_WRAP_ADVISORY/v1','client_content_generation':generation,'items':records,'unavailable_sources':unavailable,'raw_xml_only_source_count':73,'sources':{'rules_sha256':sha(rules_raw),'xml_sha256':sha(xml),'dat_sha256':sha(dat),'item_keys_sha256':sha(key_raw),'public_artifact_sha256':generation,'actual_export_sha256':sha(export_raw),'official_admission_sha256':sha(admission_raw),'official_membership_sha256':sha(member_raw),'aliases_sha256':sha(aliases_raw)}}
 return (json.dumps(result,sort_keys=True,indent=2)+'\n').encode()

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,required=True);p.add_argument('--donor',type=Path,required=True);p.add_argument('--capture',type=Path,required=True);p.add_argument('--client-artifact',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--check',action='store_true');p.add_argument('--rust-output',type=Path,required=True);p.add_argument('--rust-template',type=Path,required=True);a=p.parse_args();raw=build(a.root,a.donor,a.capture,a.client_artifact)
 rust=a.rust_template.read_text().replace('@SHA256@',sha(raw))
 if a.check:
  if a.output.read_bytes()!=raw or a.rust_output.read_text()!=rust:raise ValueError('generated advisory drift')
 else:
  a.output.write_bytes(raw);a.rust_output.write_text(rust)
 print('qualified House Wrap advisory72 + explicit absent12699 SHA256='+sha(raw))
if __name__=='__main__':main()
