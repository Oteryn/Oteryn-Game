from PIL import Image
import sys,json,pathlib
sys.path.insert(0,'tools/content-schema/world-authoring')
from world_region_codec import decode_region
im=Image.open('.questpc/phase-a-source/File-Mastermindquest2.jpg').convert('RGB').crop((0,91,107,159))
im=im.resize((im.width*2//3, im.height*2//3), Image.Resampling.NEAREST)
w,h=im.size
pattern=[]
known=[]
for y in range(h):
 bits=sum(1<<x for x in range(w) if min(im.getpixel((x,y)))>170)
 mask_known=sum(1<<x for x in range(w) if min(im.getpixel((x,y)))>170 or (55<sum(im.getpixel((x,y)))/3<170 and max(im.getpixel((x,y)))-min(im.getpixel((x,y)))<40))
 pattern.append(bits);known.append(mask_known)
mask=(1<<w)-1
pal=json.load(open('content/world/placements/index.json'))['palette']
colors={}
for file in pathlib.Path('content/world/terrain').glob('terrain*.json'):
 for r in json.load(open(file)).get('records',[]):colors[r['provenance']['source_item_id']]=r.get('automap',{}).get('value',{}).get('color',0)
results=[]
for z in range(8,16):
 rows=[0]*512
 for rx,ry in [(a,b) for a in [124,125,126] for b in [121,122]]:
  p=pathlib.Path(f'content/world/placements/region-z{z:02}-x{rx:03}-y{ry:03}.b3')
  if not p.exists():continue
  _,_,_,ss=decode_region(p.read_bytes())
  for _,ts in ss:
   for t in ts:
    color=0
    for idx,depth,attrs in t[5]:
     if depth==0 and colors.get(pal[idx]['source_item_id'],0):color=colors[pal[idx]['source_item_id']]
    if color in [179,215]:rows[t[1]-30976]|=1<<(t[0]-31744)
 best=[]
 for y in range(512-h+1):
  for x in range(768-w+1):
   sample=sum(((((rows[y+dy]>>x)&mask)^pattern[dy])&known[dy]).bit_count() for dy in range(0,h,4))
   best.append((sample,x,y))
 best.sort()
 full=[]
 for _,x,y in best[:30]:
  score=sum(((((rows[y+dy]>>x)&mask)^pattern[dy])&known[dy]).bit_count() for dy in range(h))
  full.append((score,x+31744,y+30976,z))
 full.sort();print(z,full[:3],flush=True);results.extend(full[:3])
pathlib.Path('.questpc/restart12-mastermind-second-scaled-match.json').write_text(json.dumps(sorted(results),indent=2)+'\n')
