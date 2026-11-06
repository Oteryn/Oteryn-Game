from PIL import Image,ImageDraw
import sys,json,pathlib
sys.path.insert(0,'tools/content-schema/world-authoring');from world_region_codec import decode_region
colors={}
for p in pathlib.Path('content/world/terrain').glob('terrain*.json'):
 for r in json.load(open(p)).get('records',[]):colors[r['provenance']['source_item_id']]=r.get('automap',{}).get('value',{}).get('color',0)
pal=json.load(open('content/world/placements/index.json'))['palette']
out=Image.new('RGB',(1024,512));draw=ImageDraw.Draw(out)
for n,z in enumerate(range(8,12)):
 _,_,_,ss=decode_region(pathlib.Path(f'content/world/placements/region-z{z:02}-x125-y121.b3').read_bytes())
 im=Image.new('RGB',(256,256))
 for _,ts in ss:
  for t in ts:
   c=0
   for idx,depth,attrs in t[5]:
    if depth:continue
    color=colors.get(pal[idx]['source_item_id'],0)
    if color:c=color
   im.putpixel((t[0]%256,t[1]%256),(c//36*51,(c//6%6)*51,(c%6)*51))
 out.paste(im.resize((512,512)),((n%2)*512,(n//2)*256)) if False else None
 out.paste(im,((n%4)*256,20))
 draw.text(((n%4)*256,0),f'z{z} x32000-32255 y30976-31231',fill='white')
out.crop((0,0,1024,276)).save('.questpc/mastermind-map-floors.png')
