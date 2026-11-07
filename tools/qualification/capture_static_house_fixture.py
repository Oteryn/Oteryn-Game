#!/usr/bin/env python3
"""Preserve actual static House map encodings and independent viewer binding."""
import argparse,hashlib,json
from pathlib import Path

def materialize(v):
 if v['house']!='oteryn:content.house.the_farms_5' or v['viewer']['house']!=v['house'] or v['source_house_id']!=2844:raise ValueError('actual named House binding')
 if v['source_item_id']!=2929 or v['source_position']!=[32655,31888,2] or v['viewer']['position']!=[32654,31889,2] or v['viewer']['generation']!=1:raise ValueError('actual source/viewer cell')
 rawcapture=(json.dumps(v,sort_keys=True,separators=(',',':'))+'\n').encode()
 # Keep the actual HouseServe wire snapshot/delta. The two direct reference
 # observations remain sealed by capture_sha256, outside this native play input.
 result={k:val for k,val in v.items() if k not in ('source_snapshot_hex','after_snapshot_hex')}
 result['capture_sha256']=hashlib.sha256(rawcapture).hexdigest();result['payload_sha256']={}
 for key in ('world_map_snapshot_hex','world_map_delta_hex'):
  raw=bytes.fromhex(v[key])
  if not raw or len(raw)>1048576:raise ValueError('bounded original map bytes')
  result['payload_sha256'][key]=hashlib.sha256(raw).hexdigest()
 return (json.dumps(result,sort_keys=True,indent=2)+'\n').encode()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('output',type=Path);p.add_argument('--check',action='store_true');a=p.parse_args();data=materialize(json.loads(a.capture.read_bytes()))
 if a.check:
  if a.output.read_bytes()!=data:raise ValueError('actual static House fixture drift')
 else:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_bytes(data)
 print('actual static House fixture '+('verified' if a.check else 'generated'))
if __name__=='__main__':main()
