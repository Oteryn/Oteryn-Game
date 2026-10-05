"""Extract source ground policy from exact pinned Canary protobuf + XML."""
import argparse,subprocess,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('--source-root',type=Path,default=Path('/workspace/spell-sources/canary'))
parser.add_argument('--output',type=Path,default=Path(__file__).with_name('canary-ground-proof.json'))
args=parser.parse_args()
repo=args.source_root
pin='99902524e052f37574194466c2949c576e4ab269'
if subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()!=pin:
 raise ValueError('source HEAD does not match pinned Canary revision')
def blob(p):return subprocess.check_output(['git','show',f'{pin}:{p}'],cwd=repo)
def var(b,i):
 v=0;s=0
 while True:
  x=b[i];i+=1;v|=(x&127)<<s
  if x<128:return v,i
  s+=7
  if s>63:raise ValueError('overflow')
def fields(b):
 i=0;out={}
 while i<len(b):
  key,i=var(b,i);n=key>>3;w=key&7
  if w==0:v,i=var(b,i)
  elif w==2:
   size,i=var(b,i);v=b[i:i+size];i+=size
  elif w==5:v=b[i:i+4];i+=4
  elif w==1:v=b[i:i+8];i+=8
  else:raise ValueError(w)
  out.setdefault(n,[]).append(v)
 return out
paths=['data/items/appearances.dat','src/protobuf/appearances.proto','src/items/items.cpp','data/items/items.xml','src/items/functions/item/item_parse.cpp']
sources={p:blob(p) for p in paths}
xml=ET.fromstring(sources['data/items/items.xml']);records=[]
for obj in fields(sources['data/items/appearances.dat'])[1]:
 f=fields(obj);id=f.get(1,[None])[0]
 if id not in [416,418,436]:continue
 fl=fields(f[3][0]);bank=fields(fl[1][0]);speed=bank.get(1,[0])[0];attrs=[]
 for item in xml:
  if int(item.get('id',item.get('fromid','-1')))<=id<=int(item.get('id',item.get('toid','-1'))):
   attrs.extend([dict(x.attrib) for x in item]);name=item.get('name')
 for a in attrs:
  if a.get('key')=='speed':speed=int(a['value'])&65535
 records.append({'server_item_id':id,'name':name,'appearance_bank_waypoints':bank.get(1,[0])[0],'ground_speed':speed,'is_ground':1 in fl and not fl.get(5,[0])[0],'block_solid':bool(fl.get(13,[0])[0]),'block_projectile':bool(fl.get(15,[0])[0]),'block_pathfind':bool(fl.get(16,[0])[0]),'xml_attributes':attrs,'protobuf_flag_field_numbers':sorted(fl)})
doc={'schema_revision':'native-ground-source-proof-r1','server':'canary','revision':pin,'sources':[{'path':p,'sha256':hashlib.sha256(v).hexdigest(),'blob':subprocess.check_output(['git','rev-parse',f'{pin}:{p}'],cwd=repo,text=True).strip()} for p,v in sources.items()],'records':records,'qualification':'Ground class and speed are loaded from source appearances protobuf bank, then explicitly overridden by XML if present. Flags use source protobuf defaults where fields are absent. This proof does not attribute designed room coordinates to source OTBM coordinates.'}
args.output.write_text(json.dumps(doc,indent=2)+'\n');print(f'{args.output}: {len(records)} verified ground policies')
