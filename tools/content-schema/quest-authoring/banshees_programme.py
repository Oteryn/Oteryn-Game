"""Chosen seven-seal trial on source geometry; original source claims deliver rewards."""
import hashlib
import json
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.the_queen_of_the_banshees_quest'
TRACK = 'oteryn:quest-progress/banshees-trial/stage'
PREFIX = 'oteryn:reward-claim.quest.u7_2.the_queen_of_the_banshees.reward.'
CLAIMS = [PREFIX+'boots_of_haste', PREFIX+'platinum_coin']

def p(x,y,z):
    return dict(x=x,y=y,floor=-z)


def documents():
    from qengine_access import callback
    path='content/quests/definitions/quests-00200-00299.json'
    raw=(ROOT/path).read_bytes()
    definition=json.loads(raw)['records'][37]['definition']
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if (definition['identity']['key']!=QUEST or recipe['requirements']['min_level']!=60
            or recipe['requirements']['premium'] or recipe['repeat']['kind']!='once'
            or [(s['kind'],s['count']) for s in recipe['stages']]!=[('explore',3),('use',3),('talk',1),('explore',1),('complete',1)]
            or [(r['item']['key'],r['count']) for r in recipe['reward_intents']]!=[('oteryn:item.tibia.i3079',1),('oteryn:item.tibia.i3035',100)]):
        raise ValueError('BANSHEES_SELECTED_RECIPE_DRIFT')
    sources=[f'oteryn:selected-recipe:{path}:/records/37/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
        'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/startup/tables/teleport.lua:579-614:sha256:462248b3eeed3531d4f6bc14d47b6d7184741845106dc9ee9d9a0bee5856b042',
        'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/startup/tables/chest.lua:source-UID6060-6061:sha256:0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca']
    sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
    import world_region_codec
    palette=json.loads((ROOT/'content/world/placements/index.json').read_bytes())['palette']
    cells={}
    for z in [13,14,15]:
        for rx in [125,126]:
            region=f'content/world/placements/region-z{z:02}-x{rx}-y124.b3';world=(ROOT/region).read_bytes()
            sources.append(f'oteryn:source-map:{region}:sha256:{hashlib.sha256(world).hexdigest()}')
            for _,tiles in world_region_codec.decode_region(world)[3]:
                for x,y,flags,house,zones,items in tiles:
                    cells[(x,y,z)]=[palette[i]['source_item_id'] for i,depth,attrs in items if depth==0]
    expected={(32278,31903,13):[410,1959],(32171,31853,15):[410,1959],(32215,31849,15):[410,1959],
        (32250,31892,14):[4406,1959],(32192,31938,14):[429,1959],(32311,31978,13):[410,1959],
        (32277,31903,13):[410],(32171,31854,15):[410],(32215,31848,15):[410],
        (32260,31866,14):[410],(32219,31911,15):[410],(32219,31913,15):[410,1949],
        (32218,31912,15):[410,2472],(32220,31912,15):[410,2472]}
    if any(cells.get(at)!=items for at,items in expected.items()):raise ValueError('BANSHEES_SOURCE_GEOMETRY_DRIFT')
    candidate='tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'
    sources.append(f'oteryn:source-npc:{candidate}:the_queen_of_the_banshees:sha256:{hashlib.sha256((ROOT/candidate).read_bytes()).hexdigest()}')
    world=[]
    def add(name,trigger,at,appearance,before,after,**effects):
        row=callback('authored-banshees-'+name,'the_queen_of_the_banshees_quest',trigger,
            'selected-seven-recorded-seals:'+name,source_pin='oteryn-authored:',at=at,appearance=appearance,**effects)
        row['minimum_level']=60
        row['storage']=[dict(track=TRACK,minimum=before,maximum=before,set=after)]
        row['sources'] += [s for s in sources if 'source-map:' not in s or f"region-z{-at['floor']:02}-x{at['x']//256}-y{at['y']//256}.b3" in s]
        world.append(row)
        return row
    for i,(name,at) in enumerate([('hidden',p(32277,31903,13)),('plague',p(32171,31854,15)),('demonrage',p(32215,31848,15))]):
        add(name,'step_in',at,410,i-1,i)
    for i,(name,at) in enumerate([('sacrifice',p(32250,31892,14)),('true-path',p(32192,31938,14)),('logic',p(32311,31978,13))]):
        add(name,'use',at,1959,i+2,i+3)
    npc=[dict(key='oteryn:quest-reward/banshees-pact',quest=QUEST,npc='oteryn:npc.the_queen_of_the_banshees',
        keywords=['pact','seal'],topic=0,next_topic=0,minimum_level=60,
        storage=[dict(track=TRACK,minimum=5,maximum=5,set=6)],take=[],give=None,outfit=None,achievement=None,
        reply=['You have recorded the six seals. Enter the final chamber, claim your boots and platinum, then leave through the magic portal. This chosen trial takes no soul pact.'],sources=[sources[0],sources[-1]])]
    add('final-chamber','step_in',p(32219,31911,15),410,6,7)
    escape=add('escape','step_in',p(32219,31913,15),1949,7,8,teleport=p(32269,31853,15))
    escape['completes']=True;escape['required_claims']=CLAIMS
    gates=[dict(claim=claim,quest=QUEST,conditions=[dict(track=TRACK,minimum=7,maximum=7)],sources=[sources[0],sources[2]]) for claim in CLAIMS]
    return world,npc,gates
