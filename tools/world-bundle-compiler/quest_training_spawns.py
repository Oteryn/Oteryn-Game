"""Bounded authored training creatures on verified existing Newhaven ground.

Only the selected finite recipe is authored here. The donor population, map,
NPC identities, ordinary creature rules and all held spawn groups stay intact.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
REGION = 'content/world/placements/region-z07-x127-y126.b3'
CLIENT = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
KEY = 'oteryn:spawn.newhaven_training_trolls'
FRAME = 'global-target-2026-09-27'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def document():
    sys.path.insert(0, str(ROOT/'tools/content-schema/world-authoring'))
    sys.path.insert(0, str(ROOT/'tools/content-schema/item-authoring'))
    import world_region_codec
    from engine_items import load_appearance_objects
    quest_path = 'content/quests/definitions/quests-00200-00299.json'
    quest_raw = (ROOT/quest_path).read_bytes()
    definition, = [r['definition'] for r in json.loads(quest_raw)['records']
                   if r['definition']['identity']['key']=='oteryn:quest.newhaven_quest']
    stage = definition['oteryn_recipe']['payload']['recipe']['stages'][2]
    if stage['targets'] != ['Troll'] or stage['count'] != 10:
        raise ValueError('NEWHAVEN_TRAINING_RECIPE')
    raw = (ROOT/f'content/assets/files/appearances-{CLIENT}.dat').read_bytes()
    if digest(raw) != CLIENT:
        raise ValueError('NEWHAVEN_TRAINING_CLIENT_PIN')
    appearances = load_appearance_objects(raw)
    palette_path = 'content/world/placements/index.json'
    palette_raw = (ROOT/palette_path).read_bytes()
    palette = json.loads(palette_raw)['palette']
    region_raw = (ROOT/REGION).read_bytes()
    floor, _, _, sectors = world_region_codec.decode_region(region_raw)
    if floor != 7:
        raise ValueError('NEWHAVEN_TRAINING_FLOOR')
    occupied = set()
    for path in (ROOT/'content/world/spawns').glob('spawns-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            if row['declaration']['identity']['key']==KEY:
                continue
            for point in row['declaration']['points']:
                c=point['cell']
                if c['floor']==7:
                    occupied.add((c['x'],c['y']))
    clear = set()
    for _, tiles in sectors:
        for x,y,flags,house,zones,items in tiles:
            if flags or house or zones or len(items)!=1 or items[0][1]!=0:
                continue
            f=appearances.get(palette[items[0][0]]['source_item_id'],{}).get('flags',{})
            if f.get('flags.bank') and not f.get('flags.unpass',False):
                clear.add((x,y))
    points=[]
    approaches=[]
    for _, tiles in sectors:
        for x,y,flags,house,zones,items in tiles:
            if not (32580<=x<=32605 and 32455<=y<=32485):
                continue
            if flags or house or zones or len(items)!=1 or items[0][1]!=0:
                continue
            tile_flags=appearances.get(palette[items[0][0]]['source_item_id'],{}).get('flags',{})
            if not tile_flags.get('flags.bank') or tile_flags.get('flags.unpass',False):
                continue
            if any(abs(x-ox)+abs(y-oy)<3 for ox,oy in occupied):
                continue
            if any(abs(x-p['cell']['x'])+abs(y-p['cell']['y'])<5 for p in points):
                continue
            approach=next(((ax,ay) for ax,ay in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)] if (ax,ay) in clear and (ax,ay) not in occupied),None)
            if approach is None:
                continue
            approaches.append(dict(x=approach[0],y=approach[1],floor=7))
            points.append(dict(cell=dict(x=x,y=y,floor=7),creature='oteryn:creature.troll',direction='north',respawn_ms=90000))
            if len(points)==10:
                break
        if len(points)==10:
            break
    if len(points)!=10:
        raise ValueError('NEWHAVEN_TRAINING_TEN_CLEAR_CELLS')
    sources=[dict(path=path,sha256=digest(raw)) for path,raw in
             [(quest_path,quest_raw),(REGION,region_raw),(palette_path,palette_raw)]]
    return dict(coordinate_frame=FRAME,family='Spawn.Source',records=[dict(declaration=dict(
        identity=dict(key=KEY,revision='definition-r1'),centre=dict(x=32592,y=32470,floor=7),
        points=points,provenance=dict(kind='CHOSEN_OTERYN_TRAINING',recipe='oteryn:quest.newhaven_quest',
        sources=sources,client_sha256=CLIENT,approaches=approaches,rule='Ten spaced, unprotected, empty walkable source cells; ordinary 90-second respawn.')))])


def goal_document(spawn_document, raw):
    points=spawn_document['records'][0]['declaration']['points']
    source_path='content/world/spawns/spawns-quest-training.json'
    return dict(schema=1,goals=[dict(key='oteryn:quest-transition/native-kill/newhaven-trolls',
        quest='oteryn:quest.newhaven_quest',phase='oteryn:quest-progress/newhaven/native_run',
        counter='oteryn:quest-progress/newhaven/troll_kills',activation='oteryn:quest-reward/newhaven-training-ready',
        count=10,origins=[dict(creature=p['creature'],x=p['cell']['x'],y=p['cell']['y'],floor=7,
            source_pointer=f'/records/0/declaration/points/{i}') for i,p in enumerate(points)],
        sources=[dict(path=source_path,sha256=digest(raw))])])


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    source=ROOT/'content/world/spawns/spawns-quest-training.json'
    raw=source.read_bytes()
    expected=(json.dumps(goal_document(json.loads(raw),raw),indent=2)+'\n').encode()
    target=ROOT/'tools/content-schema/quest-authoring/samples/native-kill-goals.json'
    if args.check:
        if target.read_bytes()!=expected:
            raise SystemExit('NEWHAVEN_NATIVE_GOAL_DRIFT')
    else:
        target.write_bytes(expected)
    print('PASS native Newhaven goal origins=10')
