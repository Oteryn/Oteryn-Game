"""Selected Newhaven tour POIs, anchored to source NPC/map facts."""
import hashlib
import json
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[3]
POIS=[('school',32570,32507,32570,32506,'south',2184,2185),
      ('supply',32567,32489,32568,32489,'west',2161,2163)]

def extend(rows,make):
    sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
    sys.path.insert(0,str(ROOT/'tools/content-schema/item-authoring'))
    import world_region_codec
    from engine_items import load_appearance_objects
    client='2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
    raw=(ROOT/f'content/assets/files/appearances-{client}.dat').read_bytes()
    if hashlib.sha256(raw).hexdigest()!=client:raise ValueError('NEWHAVEN_POI_CLIENT_PIN')
    objects=load_appearance_objects(raw)
    palette=json.loads((ROOT/'content/world/placements/index.json').read_bytes())['palette']
    region=ROOT/'content/world/placements/region-z07-x127-y126.b3'
    tiles={(x,y):(flags,house,zones,items) for _,ts in world_region_codec.decode_region(region.read_bytes())[3] for x,y,flags,house,zones,items in ts}
    recipe='content/quests/definitions/quests-00200-00299.json'
    recipe_hash=hashlib.sha256((ROOT/recipe).read_bytes()).hexdigest()
    for name,x,y,ax,ay,direction,start,end in POIS:
        for cell in [(x,y),(ax,ay)]:
            flags,house,zones,items=tiles[cell]
            if house or len(items)!=1 or items[0][1]!=0:raise ValueError('NEWHAVEN_POI_NOT_CLEAR')
            f=objects[palette[items[0][0]]['source_item_id']]['flags']
            if not f.get('flags.bank') or f.get('flags.unpass',False):raise ValueError('NEWHAVEN_POI_NOT_WALKABLE')
        row=make('newhaven-'+name,'newhaven_quest','step_in',
            f'data-global/world/world-npc.xml:{start}-{end}:sha256:db07b021856008bce61efe816e8a8eea00a01cb9750f1be5e750093758d1c029',
            at=dict(x=x,y=y,floor=-7))
        row['storage']=[dict(track='oteryn:quest-progress/newhaven/stage',minimum=0,maximum=0,set=0,preserve=True),
            dict(track='oteryn:quest-progress/newhaven/'+name,minimum=-1,maximum=0,set=1)]
        row['minimum_level']=2
        row['success_message']='You visited the Newhaven '+('training school.' if name=='school' else 'village supply point.')
        row['sources'] += [f'oteryn:selected-recipe:{recipe}:/records/78/definition:sha256:{recipe_hash}',
                           'oteryn:source-map:'+str(region.relative_to(ROOT))+':sha256:'+hashlib.sha256(region.read_bytes()).hexdigest()]
        rows.append(row)
