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
    row['effects']['item_reward']=dict(item='oteryn:item.tibia.i2992',count=20,contents=[])
    row['sources'] += [f'oteryn:selected-recipe:{path}:/records/19/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                       f'oteryn:source-map:{region}:sha256:{hashlib.sha256(region_raw).hexdigest()}']
    rows.append(row)
