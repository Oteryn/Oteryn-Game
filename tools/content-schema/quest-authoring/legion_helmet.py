"""Chosen Legion route contacts; original source claims and prerequisites retained."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]

def extend(callbacks):
    path = 'content/quests/definitions/quests-00000-00099.json'
    raw = (ROOT/path).read_bytes()
    definition, = [r['definition'] for r in json.loads(raw)['records']
                   if r['definition']['identity']['key']=='oteryn:quest.newhaven_legion_helmet_quest']
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if (recipe['requirements']['min_level'] != 0 or recipe['requirements']['prerequisites'] != []
            or recipe['stages'][0]['targets'] != ['Gustavo, The Guard','Viola']
            or recipe['reward_intents'][0]['item']['key'] != 'oteryn:item.tibia.i3374'):
        raise ValueError('LEGION_SELECTED_RECIPE_DRIFT')
    source = f'oteryn:selected-recipe:{path}:/records/62/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    for name,npc in [('gustavo','gustavo_the_guard'),('viola','viola')]:
        callbacks.append(dict(key='oteryn:quest-reward/legion-contact-'+name,
            quest='oteryn:quest.newhaven_legion_helmet_quest',npc='oteryn:npc.'+npc,
            keywords=['helmet','legion'],topic=0,next_topic=0,
            storage=[dict(track='oteryn:quest-progress/legion-helmet/'+name,minimum=-1,maximum=-1,set=1)],
            take=[],give=None,outfit=None,achievement=None,
            reply=['The helmet route leads through the training cave. Speak to both of us, descend to the equipment room, search the dead human, then return through the magic forcefield.'],
            sources=[source]))


def claim_gates():
    path='content/quests/definitions/quests-00000-00099.json'
    raw=(ROOT/path).read_bytes()
    return dict(schema=1,gates=[dict(claim='oteryn:reward-claim.kv.legiondawnport',
        quest='oteryn:quest.newhaven_legion_helmet_quest',
        conditions=[dict(track='oteryn:quest-progress/legion-helmet/'+name,minimum=1,maximum=1)
                    for name in ['gustavo','viola','cave']],
        sources=[f'oteryn:selected-recipe:{path}:/records/62/definition:sha256:{hashlib.sha256(raw).hexdigest()}'])])

def extend_world(rows,make):
    import sys
    sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
    import world_region_codec
    palette=json.loads((ROOT/'content/world/placements/index.json').read_bytes())['palette']
    sources=[]
    cells={}
    for z in [7,8]:
        path=f'content/world/placements/region-z{z:02}-x125-y124.b3'
        raw=(ROOT/path).read_bytes()
        sources.append(f'oteryn:source-map:{path}:sha256:{hashlib.sha256(raw).hexdigest()}')
        for _,tiles in world_region_codec.decode_region(raw)[3]:
            for x,y,flags,house,zones,items in tiles:
                if (x,y) in [(32111,31903),(32122,31939),(32123,31939)]:
                    cells[(x,y,z)]=[(palette[p]['source_item_id'],attrs) for p,depth,attrs in items if depth==0]
    if (cells[(32111,31903,7)] != [(593,None)]
            or cells[(32111,31903,8)] != [(386,None)]
            or cells[(32122,31939,8)][0] != (4409,None)
            or cells[(32123,31939,8)] != [(500,None),(1949,{'teleport':(32110,31937,8)})]):
        raise ValueError('LEGION_SOURCE_CAVE_GEOMETRY_DRIFT')
    def storage(name,minimum,maximum,value,preserve=False):
        return dict(track='oteryn:quest-progress/legion-helmet/'+name,minimum=minimum,maximum=maximum,set=value,**({'preserve':True} if preserve else {}))
    contacts=[storage(name,1,1,1,True) for name in ['gustavo','viola']]
    # Chosen Oteryn bridge: keep the inherited physical cave objects while
    # making the selected Newhaven route reachable from the school entrance.
    entry_path='content/world/placements/region-z07-x127-y126.b3'
    entry_raw=(ROOT/entry_path).read_bytes()
    entry_cells={(x,y):items for _,tiles in world_region_codec.decode_region(entry_raw)[3]
                 for x,y,flags,house,zones,items in tiles}
    ground=palette[entry_cells[(32570,32508)][0][0]]['source_item_id']
    row=make('authored-legion-route-entry','newhaven_legion_helmet_quest','step_in',
             'selected-recipe:authored-newhaven-school-to-inherited-training-cave',source_pin='oteryn-authored:',
             at=dict(x=32570,y=32508,floor=-7),appearance=ground,
             teleport=dict(x=32111,y=31904,floor=-7),
             passages=[dict(position=dict(x=32111,y=31903,floor=-7),appearance=593,replacement=594)],
             portals=dict(appearance=1949,lifetime_us=None,portals=[dict(position=dict(x=32570,y=32508,floor=-7),destination=dict(x=32111,y=31904,floor=-7)),dict(position=dict(x=32111,y=31904,floor=-7),destination=dict(x=32570,y=32507,floor=-7))]))
    row['storage']=contacts+[storage('route',-1,1,1)]
    row['minimum_level']=1
    row['sources']+=sources+[f'oteryn:source-map:{entry_path}:sha256:{hashlib.sha256(entry_raw).hexdigest()}']
    rows.append(row)
    row=make('authored-legion-cave-entry','newhaven_legion_helmet_quest','step_in',
             'selected-recipe:legion-training-cave-source-hole-and-rope-spot',source_pin='oteryn-authored:',
             at=dict(x=32111,y=31903,floor=-8),appearance=386)
    row['storage']=contacts+[storage('cave',-1,-1,1)]
    row['minimum_level']=1 # Every legal character; makes source world context mandatory.
    row['sources']+=sources
    rows.append(row)
    row=make('authored-legion-cave-return','newhaven_legion_helmet_quest','step_in',
             'selected-recipe:legion-source-magic-forcefield-return',source_pin='oteryn-authored:',
             at=dict(x=32110,y=31937,floor=-8),appearance=4399)
    row['movement_origins']=[dict(x=x,y=y,floor=-8) for x,y in [(32122,31939),(32124,31939),(32123,31938),(32123,31940)]]
    row['teleport']=dict(x=32570,y=32507,floor=-7)
    row['required_claims']=['oteryn:reward-claim.kv.legiondawnport']
    row['completes']=True
    row['storage']=contacts+[storage('cave',1,1,1,True),storage('finished',-1,-1,1)]
    row['sources']+=sources
    rows.append(row)


def world_callbacks():
    from qengine_access import callback
    rows=[]
    extend_world(rows,callback)
    for row in rows:
        row['sources'] += claim_gates()['gates'][0]['sources']
    return dict(schema=1,callbacks=sorted(rows,key=lambda r:r['key']))
