import json,pathlib,sys,collections
sys.path.insert(0,'tools/content-schema/world-authoring');from world_region_codec import decode_region
pal=json.load(open('content/world/placements/index.json'))['palette'];names={};fires=set();boxes=set()
for p in pathlib.Path('content/items/definitions').glob('items*.json'):
 for r in json.load(open(p)).get('records',[]):
  d=r.get('definition',{});key=d.get('identity',{}).get('key','')
  if not key.startswith('oteryn:item.tibia.i'):continue
  n=d.get('semantics',{}).get('presentation',{}).get('value',{}).get('name',{}).get('value','')
  id=int(key.rsplit('i',1)[-1]);names[id]=n
  if n in ['campfire','fire']:fires.add(id)
  if n=='box':boxes.add(id)
print('box ids',sorted(boxes),'fire ids',sorted(fires))
rs=[]
for z in range(8,14):
 ts=[]
 for rx in [124,125,126]:
  for ry in [121,122]:
   p=pathlib.Path(f'content/world/placements/region-z{z:02}-x{rx:03}-y{ry:03}.b3')
   if p.exists():
    _,_,_,ss=decode_region(p.read_bytes());ts.extend(t for _,vs in ss for t in vs)
 ft=[t for t in ts if any(pal[i]['source_item_id'] in fires for i,d,a in t[5])]
 for t in ts:
  bs=[(pal[i]['source_item_id'],d,a) for i,d,a in t[5] if pal[i]['source_item_id'] in boxes]
  if bs:
   near=[list(f[:2]) for f in ft if max(abs(f[0]-t[0]),abs(f[1]-t[1]))<=5]
   rs.append({'position':[t[0],t[1],z],'boxes':bs,'near_fire':near})
pathlib.Path('.questpc/restart12-mastermind-boxes.json').write_text(json.dumps(rs,indent=2)+'\n')
print('total',len(rs))
for r in rs:
 if r['near_fire']:print(r)
