"""SRV7: bounded DERIVED Halloween reward chamber, never an Instance or corpse MINT.

The public source specifies portal33317,32415,7, thirty minutes, a room reward
body12816 and five-minute visit. It supplies no room coordinates. This explicit
native layout choice uses previously empty cells within the existing World frame
and copies the actual Darashia entry's qualified sand and flags, without PZ.
"""
from __future__ import annotations
import argparse, hashlib, json, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
import world_region_codec as codec
INDEX='content/world/placements/index.json'
SUMMARY='tools/content-schema/world-authoring/samples/world-base-capture-v1.json'
SURFACE='content/world/placements/region-z07-x130-y126.b3'
ROOM='content/world/placements/region-z15-x130-y126.b3'
SPEC='tools/content-migration/pumpkin_room.json'
BODY=(33317,32414,15); LANDING=(33317,32417,15); EXIT=(33317,32419,15)
BOUNDS=(33312,32410,33322,32420)
def canonical(v):return (json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode()
def sha(v):return hashlib.sha256(v).hexdigest()
def tiles(raw):
 z,rx,ry,sectors=codec.decode_region(raw)
 return z,rx,ry,{(x,y):(x,y,f,h,zones,items) for _,ts in sectors for x,y,f,h,zones,items in ts},len(sectors)
def encode(z,rx,ry,rows):
 sectors={}
 for (x,y),row in rows.items():sectors.setdefault((y//32%8)*8+x//32%8,[]).append(row)
 return codec.encode_region(z,rx,ry,{local:codec.encode_sector_tiles(rx*8+local%8,ry*8+local//8,sorted(ts,key=lambda t:(t[1],t[0]))) for local,ts in sectors.items()})
SPEC_SHA='48b645d0d99acc8e7424ab44407ce43fa0370bf4108785b7433941c5f8476c86'
TRANSITION_KEY='oteryn:transition.teleport.x33317_y32419_z15'
def expected_transition():
 frame='global-target-2026-09-27'
 target={'family':'Transition','key':TRANSITION_KEY,'revision':'definition-r1'}
 return {'declaration':{'from':{'coordinate_frame':frame,'floor':15,'x':33317,'y':32419},'identity':{'key':TRANSITION_KEY,'revision':'definition-r1'},'kind':'Transition','object':{'family':'Item','key':'oteryn:item.tibia.i1949','revision':'definition-r1'},'to':{'coordinate_frame':frame,'floor':1,'x':33213,'y':32454},'transition_kind':'teleport'},'source_bindings':[{'disposition':'EXACT','external_id':'33317,32419,15:1949','identity_namespace':'oteryn/derived-halloween-reward-room','source_key':'oteryn:source.native-derived-halloween-room','source_revision':SPEC_SHA,'target':target}]}
def transition_ledger():
 return {'classification':'DERIVED_NATIVE_ROOM','generator':'tools/content-migration/pumpkin_room.py','path':SPEC,'sha256':SPEC_SHA,'transition':TRANSITION_KEY}
def qualified_transition(root,record):
 try:return record==expected_transition() and sha((root/SPEC).read_bytes())==SPEC_SHA
 except OSError:return False

def transition_outputs(root):
 import convert_world_metadata as metadata
 index=json.loads((root/'content/world/transitions/index.json').read_bytes())
 capture_path='tools/content-schema/world-authoring/samples/source-capture-v1.json'
 capture=json.loads((root/capture_path).read_bytes())
 records=[]
 for name in index['shards']:records.extend(json.loads((root/name).read_bytes())['records'])
 found=[r for r in records if r['declaration']['identity']['key']==TRANSITION_KEY]
 if found and found!=[expected_transition()]:raise ValueError('derived Pumpkin Transition conflicts with authored record')
 added=not found
 if added:records.append(expected_transition())
 generated=metadata.shard_files('Transition.Teleport',metadata.unique(records,'Transition.Teleport'),source=index['source'])
 updated=json.loads(generated['content/world/transitions/index.json']);updated['native_derived_transitions']=[transition_ledger()]
 generated['content/world/transitions/index.json']=canonical(updated)
 capture['families']['Transition.Teleport']+=int(added)
 capture['teleport_object_families']['Item']+=int(added)
 capture['native_derived_transitions']=[transition_ledger()]
 generated[capture_path]=canonical(capture)
 return generated

def outputs(root=ROOT, specification_bytes=None):
 original={name:(root/name).read_bytes() for name in (INDEX,SUMMARY,SURFACE,ROOM)}
 world=json.loads((root/'content/world/worlds/worlds-00000-00000.json').read_bytes())['records'][0]['declaration']
 bounds=world['bounds']
 if 15 not in world['floors'] or not(bounds['min_x']<=BOUNDS[0]<=BOUNDS[2]<bounds['max_x_exclusive'] and bounds['min_y']<=BOUNDS[1]<=BOUNDS[3]<bounds['max_y_exclusive']):raise ValueError('derived room outside existing World ownership')
 bindings=json.loads((root/'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
 if not any(row['external_id']=='12816' and row['target']=={'family':'Item','key':'oteryn:item.tibia.i12816','revision':'definition-r1'} for row in bindings):raise ValueError('room body requires actual canonical Item binding')
 index=json.loads(original[INDEX]);summary=json.loads(original[SUMMARY]);palette=index['palette']
 def pid(identifier):
  selected=[n for n,p in enumerate(palette) if p['source_item_id']==identifier]
  if len(selected)>1:raise ValueError('duplicate source palette identity')
  if not selected:
   if identifier!=12816:raise ValueError('missing existing physical source palette')
   palette.append({'key':'oteryn:item.tibia.i12816','provisional':False,'source_item_id':12816});return len(palette)-1,True
  n=selected[0]
  if palette[n]['key']!=f'oteryn:item.tibia.i{identifier}' or palette[n]['provisional']:raise ValueError('source palette requires canonical qualification')
  return n,False
 floor,_=pid(231);wall,_=pid(1112);portal,_=pid(1949);body,new_palette=pid(12816)
 z,rx,ry,surface,old_surface_sectors=tiles(original[SURFACE]);source=surface.get((33317,32415))
 if source is None or source[2:5]!=(0,0,()) or source[-1] not in ([(floor,0,None)],[(floor,0,None),(portal,0,None)]):raise ValueError('exact existing Darashia source floor changed')
 surface_added=0
 if source[-1]==[(floor,0,None)]:surface[33317,32415]=(*source[:-1],source[-1]+[(portal,0,None)]);surface_added=1
 surface_raw=encode(z,rx,ry,surface)
 z,rx,ry,room,old_room_sectors=tiles(original[ROOM]);new_tiles=new_items=0
 for y in range(BOUNDS[1],BOUNDS[3]+1):
  for x in range(BOUNDS[0],BOUNDS[2]+1):
   items=[(floor,0,None)]
   if x in (BOUNDS[0],BOUNDS[2]) or y in (BOUNDS[1],BOUNDS[3]):items.append((wall,0,None))
   if (x,y)==BODY[:2]:items.append((body,0,None))
   if (x,y)==EXIT[:2]:items.append((portal,0,{'teleport':(33213,32454,1)}))
   expected=(x,y,0,0,(),items)
   if (x,y) in room:
    if room[x,y]!=expected:raise ValueError('derived room would overwrite existing World cells')
   else:room[x,y]=expected;new_tiles+=1;new_items+=len(items)
 room_raw=encode(z,rx,ry,room)
 result={SURFACE:surface_raw,ROOM:room_raw};sector_delta=0
 for name,raw,before_sectors in [(SURFACE,surface_raw,old_surface_sectors),(ROOM,room_raw,old_room_sectors)]:
  _,_,_,rows,sector_count=tiles(raw);meta=next(p for p in index['regions'] if p['path']==name)
  meta.update(sha256=sha(raw),tiles=len(rows),items=sum(len(t[-1]) for t in rows.values()))
  sector_delta+=sector_count-before_sectors
 delta_items=new_items+surface_added
 index['totals']['tiles']+=new_tiles;index['totals']['items']+=delta_items;index['totals']['sectors']+=sector_delta
 specification_bytes = (root/SPEC).read_bytes() if specification_bytes is None else specification_bytes
 spec=json.loads(specification_bytes)
 if spec['classification']!='DERIVED_NATIVE_ROOM' or spec['reward_body']['cell']!=list(BODY) or spec['landing']!=list(LANDING) or spec['exit']!=list(EXIT):raise ValueError('derived room source layout drift')
 index['source']['halloween_reward_room']={'path':SPEC,'sha256':sha(specification_bytes),'classification':'DERIVED_NATIVE_ROOM','tiles':121,'static_reward_body':12816}
 summary['source']=index['source'];summary['totals']=index['totals']
 summary['tiles_by_floor']['15']+=new_tiles;summary['palette']['entries']+=int(new_palette)
 summary['palette']['families']['item']['entries']+=int(new_palette)
 summary['palette']['families']['item']['occurrences']+=delta_items
 summary['item_attributes']['teleport']+=int(new_tiles>0)
 summary['bytes_on_disk']+=sum(len(result[name])-len(original[name]) for name in (SURFACE,ROOM))
 result.update(transition_outputs(root))
 result[INDEX]=canonical(index);result[SUMMARY]=canonical(summary)
 return result
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args()
 generated=outputs()
 stale=set(p for p in ROOT.glob('content/world/transitions/teleports-*.json') if str(p.relative_to(ROOT)) not in generated)
 if a.check and stale:raise SystemExit('stale derived Pumpkin transition shard')
 if not a.check:
  for path in stale:path.unlink()
 for name,raw in generated.items():
  if a.check:
   if (ROOT/name).read_bytes()!=raw:raise SystemExit('stale derived Pumpkin room: '+name)
  else:(ROOT/name).write_bytes(raw)
