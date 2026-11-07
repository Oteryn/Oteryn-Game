"""Chosen personal Nimmersatt supplies; global raid/event scheduling is untouched."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
QUEST='oteryn:quest.authored.nimmersatt_s_lair'

def extend_world(rows):
    from qengine_access import callback
    path='content/quests/definitions/quests-00300-00351.json'
    raw=(ROOT/path).read_bytes()
    definition,=[r['definition'] for r in json.loads(raw)['records'] if r['definition']['identity']['key']==QUEST]
    recipe=definition['recipe']
    if (recipe['stages'][1]['count']!=20 or recipe['stages'][2]['count']!=20
            or recipe['stages'][1]['target_items'][0]['item']['key']!='oteryn:item.tibia.i2992'
            or recipe['reward_intents'][0]['item']['key']!='oteryn:item.tibia.i44179'):
        raise ValueError('NIMMERSATT_SELECTED_RECIPE_DRIFT')
    import sys
    sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
    import world_region_codec
    region='content/world/placements/region-z07-x125-y123.b3'
    region_raw=(ROOT/region).read_bytes()
    palette=json.loads((ROOT/'content/world/placements/index.json').read_bytes())['palette']
    cells={(x,y):(house,items) for _,tiles in world_region_codec.decode_region(region_raw)[3] for x,y,flags,house,zones,items in tiles}
    for cell in [(32232,31578),(32232,31579)]:
        house,items=cells[cell]
        if house or len(items)!=1 or palette[items[0][0]]['source_item_id']!=799:
            raise ValueError('NIMMERSATT_NORTH_SUPPLY_SOURCE_DRIFT')
    row=callback('authored-nimmersatt-snowbank','authored.nimmersatt_s_lair','use',
        'selected-personal-programme:northern-supply-bundle',source_pin='oteryn-authored:',
        at=dict(x=32232,y=31578,floor=-7),appearance=799)
    row['storage']=[dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=0,maximum=0,set=1),
                    dict(track='oteryn:quest-progress/nimmersatt/wall_hits',minimum=-1,maximum=-1,set=0)]
    row['passages']=[dict(position=dict(x=32236,y=31582,floor=-7),appearance=799,replacement=1441)]
    row['effects']['item_reward']=dict(item='oteryn:item.tibia.i2992',count=20,contents=[])
    row['sources'] += [f'oteryn:selected-recipe:{path}:/records/19/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                       f'oteryn:source-map:{region}:sha256:{hashlib.sha256(region_raw).hexdigest()}']
    rows.append(row)

    row=callback('authored-nimmersatt-wall','authored.nimmersatt_s_lair','use',
        'selected-personal-programme:twenty-exact-snowball-wall-uses',source_pin='oteryn-authored:',
        at=dict(x=32236,y=31582,floor=-7))
    row['storage']=[dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=1,maximum=1,set=1,preserve=True)]
    row['item_on_target']=dict(item='oteryn:item.tibia.i2992',count=1,replacement=None,target_replacement=None)
    row['counted_use']=dict(track='oteryn:quest-progress/nimmersatt/wall_hits',limit=20,project_on_complete=True)
    row['effects']['transform']=799
    row['sources'] += [f'oteryn:selected-recipe:{path}:/records/19/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                       f'oteryn:source-map:{region}:sha256:{hashlib.sha256(region_raw).hexdigest()}']
    rows.append(row)

def extend_rewards(callbacks, greetings=None):
    path='content/quests/definitions/quests-00300-00351.json'
    raw=(ROOT/path).read_bytes()
    source=f'oteryn:selected-recipe:{path}:/records/19/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    if greetings is not None:
        greetings.append(dict(npc='oteryn:npc.rascalio',
            greet='Welcome. Ask about passage to the personal Nimmersatt trial.',
            farewell='Farewell. Bring your report when the lava wall is broken.',
            walkaway='Come back when you are ready for the trial.',sources=[source]))
    def add(name,words,storage,reply,give=None,completes=False):
        callbacks.append(dict(key='oteryn:quest-reward/nimmersatt-'+name,
            quest=QUEST,npc='oteryn:npc.rascalio',keywords=words,topic=0,next_topic=0,
            storage=storage,take=[],give=give,outfit=None,achievement=None,
            premium_required=True,reply=[reply],sources=[source],**({'completes':True} if completes else {})))
    add('passage',['passage','nimmersatt','mission'],
        [dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=-1,maximum=-1,set=0)],
        'You may enter the personal Nimmersatt trial. Collect twenty snowballs at the northern snowbank, use each against the lava wall, then report to me for bottled dragon breath.')
    add('breath',['breath','report','reward'],[
        dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=1,maximum=1,set=2),
        dict(track='oteryn:quest-progress/nimmersatt/wall_hits',minimum=20,maximum=20,set=20,preserve=True)],
        'You broke the lava wall. Take this bottled dragon breath as your reward.',
        give=dict(item='oteryn:item.tibia.i44179',count=1,contents=[]),completes=True)
