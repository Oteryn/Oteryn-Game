import sys,subprocess,json
from pathlib import Path
sys.path.insert(0,'tools/content-schema/world-authoring');import world_region_codec as codec
p='content/world/placements/region-z13-x129-y123.b3';index='content/world/placements/index.json'
def stage(n,p):return subprocess.check_output(['git','show',f':{n}:{p}'])
merged=json.loads(Path(index).read_bytes())['palette']
key=lambda x:json.dumps(x,sort_keys=True)
# Definition identity, independent of generated digest and source metadata.
def ident(v):
 if isinstance(v,dict):
  for k in ['source_item_id','server_item_id','item_id','source_id']:
   if k in v:return(k,v[k])
  d=v.get('definition',v)
  if 'key' in d:return d['key']
 return key(v)
lookup={ident(v):i for i,v in enumerate(merged)}
alltiles=[]
for n in (1,2,3):
 raw=stage(n,index)
 if n==1 and raw.startswith(b'<<<<'):raw=raw.split(b'\n',1)[1].split(b'\n|||||||',1)[0].split(b'\n=======',1)[0]
 palette=json.loads(raw)['palette'];mapping={i:lookup[ident(v)] for i,v in enumerate(palette)}
 z,rx,ry,sectors=codec.decode_region(stage(n,p));tiles={}
 for _,rows in sectors:
  for x,y,flags,house,zones,items in rows:tiles[x,y]=(x,y,flags,house,zones,[(mapping[pi],depth,attrs) for pi,depth,attrs in items])
 alltiles.append(tiles)
b,a,c=alltiles;out={}
for pos in dict.fromkeys([*a,*c]):
 av=a.get(pos);bv=b.get(pos);cv=c.get(pos)
 if av==bv:out[pos]=cv
 elif cv==bv or av==cv or cv is None:out[pos]=av
 elif av is None:out[pos]=cv
 else:
  items=list(av[5]);items.extend(v for v in cv[5] if v not in items and (bv is None or v not in bv[5]));out[pos]=(*av[:5],items)
sectors={}
for row in out.values():
 if row is None:continue
 x,y=row[:2];local=((y//32)%8)*8+(x//32)%8;sectors.setdefault(local,[]).append(row)
encoded={}
for local,tiles in sectors.items():
 sx=rx*8+local%8;sy=ry*8+local//8
 encoded[local]=codec.encode_sector_tiles(sx,sy,sorted(tiles,key=lambda row:(row[1],row[0])))
Path(p).write_bytes(codec.encode_region(z,rx,ry,encoded));print('merged region tiles',len(out))
