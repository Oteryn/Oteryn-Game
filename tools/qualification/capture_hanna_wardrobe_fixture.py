"""Extract exact captured wardrobe domain21 bytes; never invent admission frames."""
import argparse, hashlib, json
from pathlib import Path
from seasonal_capture_fields import fields

def materialize(value):
 if 25 not in value['selected_capabilities'] or value['award']!={'addons':2,'command_id':9,'outfit_key':'oteryn:content.outfit.citizen_outfits'}:raise ValueError('actual Hanna award/selection')
 for phase in ('grant','reconnect'):
  records=[]
  for raw in value[phase+'_frames']:
   e=fields(bytes.fromhex(raw));kind=e[1][0];p=fields(e[4][0]);seq=e.get(3,[0])[0]
   if kind==2:raise ValueError('capture contract has no Accepted')
   if kind==12:
    for domain in fields(p[3][0]).get(1,[]):
     d=fields(domain)
     if d[1]==[21]:records.append({'kind':'snapshot','revision':d[2][0],'snapshot_type':d[3][0],'payload_hex':d[4][0].hex()})
   elif kind==9 and p[1]==[21]:records.append({'kind':'delta','server_sequence':seq,'base_revision':p.get(2,[0])[0],'new_revision':p[3][0],'delta_type':p[4][0],'payload_hex':p[5][0].hex()})
  if not any(r['kind']=='snapshot' for r in records):raise ValueError('actual appearance snapshot')
  value[phase+'_appearance']=records
 value['capture_sha256']=hashlib.sha256(json.dumps({k:v for k,v in value.items() if k not in ('grant_appearance','reconnect_appearance')},sort_keys=True,separators=(',',':')).encode()).hexdigest()
 return (json.dumps(value,sort_keys=True,indent=2)+'\n').encode()
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('output',type=Path);p.add_argument('--check',action='store_true');a=p.parse_args();data=materialize(json.loads(a.capture.read_bytes()))
 if a.check:
  if a.output.read_bytes()!=data:raise ValueError('Hanna fixture drift')
 else:a.output.write_bytes(data)
 print('actual Hanna wardrobe fixture verified' if a.check else 'actual Hanna wardrobe fixture generated')
