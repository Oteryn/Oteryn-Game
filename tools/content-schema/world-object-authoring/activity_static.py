"""Publish bounded pinned activity-map static geometry without inventory Item aliases."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
from functools import lru_cache
from pathlib import Path
import world_objects

ROOT = Path(__file__).resolve().parents[3]
IDENTIFIERS = frozenset([20661,33067,33068,34009,39095,23467,23468,33017,42852,42941,42944,42947,42949,42950,42952,42958,42961,42973,42974,42975,42976,42977,42978,42979,42981,42987,42991,42995,42997,42370,42371,42374,42376,42377,42380,42382,42385,42654,42655,42656,42658,42659,42660,42665,42666,42667,42753,42856,42858,42859,42865,43358,43359,43360,43361,43362,43363,43364,43365,43366,33066,33069,10649,10650,10653,10654,10728,10730,10735,10736,10737,10740,10747,33070,25181,42940,42982,42983,43308,43309,43311,43312,43313,43314,43315,43316,43317,32898,42379,42651,42653,43310,42662,42663,42664,42680,42681,42863,42864,43089,43092,11075,11077,11079,11102,11103,11104,11105,34943,34946,34951,34953,42880,42887,42892,42893,42894,42895,42896,42897,42898,42899,42900,42901,42902,42910,42923,42933,43516,46629,46630,46631,46632,46633,46634,46635,46636,46637,46638,46639,46640,46641,46643,46644,46645,46646,48236,32906,43497,43498,43507,43508,43983,43984,43985,43986,43987,43988,43989,42671,23049,33276])
OFFICIAL_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
SHARDS = {'Terrain': 'content/world/terrain/terrain-08614-08650.json',
          'WorldObject': 'content/world/objects/objects-12932-13055.json'}
REGISTRY = 'content/encounters/source-static-definitions.json'
NOTE = ' Additionally SRX-3 pinned activity static geometry: 38 Terrain and 124 WorldObject records; no Item aliases.'

def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)+'\n').encode()

@lru_cache(maxsize=4)
def outputs(root=ROOT):
    policies = {}
    captures = []
    for path in sorted((root/'content/encounters').glob('*/source-room.json')):
        document = json.loads(path.read_bytes())
        if document['source_revision'] != PIN:
            raise ValueError('activity source revision')
        captures.append(document)
        for policy in document['item_policies']:
            identifier = policy['server_item_id']
            if identifier not in IDENTIFIERS:
                continue
            if policy['movable'] or policy['pickupable'] or policy['dynamic_kind'] is not None:
                raise ValueError('static scene source has mutable Item behavior')
            if identifier in policies and policies[identifier] != policy:
                raise ValueError('inconsistent pinned source static policy')
            policies[identifier] = policy
    # These three literal replacement maps have their own source schema. Only
    # source-proven static decoration joins this publisher; no mutable Item is
    # inferred from a map stack or an appearance.
    ebb_root=root/'content/encounters/soul-war-ebb-source-maps'
    for name in ('ebb-flow-empty.json','ebb-flow-inundate.json','ebb-flow.json'):
        document=json.loads((ebb_root/name).read_bytes())
        if document['schema']!='OTERYN_SOUL_WAR_EBB_SOURCE_MAP/v1' or document['source_revision']!=PIN:
            raise ValueError('Ebb static source revision')
        for policy in document['item_policies']:
            identifier=policy['server_item_id']
            if identifier!=33276:
                continue
            if policy['movable'] or policy['pickupable'] or policy['dynamic_kind'] is not None:
                raise ValueError('Ebb decoration has mutable Item behavior')
            if identifier in policies and policies[identifier]!=policy:
                raise ValueError('inconsistent Ebb static policy')
            policies[identifier]=policy
    if policies.keys() != IDENTIFIERS:
        raise ValueError('closed static geometry census')
    raw = (root/f'content/assets/files/appearances-{OFFICIAL_SHA}.dat').read_bytes()
    if hashlib.sha256(raw).hexdigest() != OFFICIAL_SHA:
        raise ValueError('official appearance artifact digest')
    appearances = world_objects.engine_items.load_appearance_objects(raw)
    admitted = json.loads((root/'imports/official/appearance-membership/admitted.json').read_bytes())
    membership = next(row for row in admitted['files'] if row['label']=='client-15.30')
    members_raw = (root/'imports/official/appearance-membership'/membership['manifest']).read_bytes()
    if hashlib.sha256(members_raw).hexdigest() != membership['manifest_sha256'] or membership['appearances_sha256'] != OFFICIAL_SHA:
        raise ValueError('official membership digest')
    members = {row[0] for row in json.loads(members_raw)['entries']}
    source = {'engine':'crystal', 'profile':'OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1',
              'repository':'zimbadev/crystalserver', 'revision':PIN}
    records = {'Terrain':[], 'WorldObject':[]}; registry = []
    for identifier, policy in sorted(policies.items()):
        if identifier not in members or identifier not in appearances:
            raise ValueError('activity static geometry lacks official appearance membership')
        family = 'Terrain' if policy['ground'] else 'WorldObject'
        record = world_objects.build_record(family, identifier, f'oteryn:item.tibia.i{identifier}',
            'ground' if policy['ground'] else 'object', None, appearances[identifier], source)
        del record['provenance']['item_pointer']
        # Source C++ loads projectile blocking from appearance unsight; absent is false.
        known = lambda value: {'state':'KNOWN', 'value':value}
        if family == 'Terrain':
            record['kind'] = known('ground')
            record['walkable'] = known(not policy['block_solid'])
            record['ground_speed'] = known(policy['ground_speed'])
            record['blocks_projectile'] = known(policy['block_projectile'])
        else:
            record['kind'] = known('object')
            record['movable'] = known(False)
            record['collision']['blocks_movement'] = known(policy['block_solid'])
            record['collision']['blocks_projectile'] = known(policy['block_projectile'])
        records[family].append(record)
        registry.append({'source_item_id':identifier, 'identity':record['identity'], 'policy':policy, 'catalogue_record':record})
    # This restore-only key already has a canonical Terrain ordinal. Replace
    # exactly that row via this publisher instead of introducing a duplicate.
    restore=next(row for row in records['Terrain'] if row['identity']['key']=='oteryn:terrain.tibia.i23049')
    existing_path='content/world/terrain/terrain-05500-05999.json'
    existing=json.loads((root/existing_path).read_bytes())
    matches=[i for i,row in enumerate(existing['records']) if row['identity']['key']==restore['identity']['key']]
    if len(matches)!=1:
        raise ValueError('Anomaly restore target must retain one original canonical ordinal')
    previous=existing['records'][matches[0]]
    if previous['provenance']['source']['profile'] not in ('crystal_ff7ede5_item_definition_v1','OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1') or previous['provenance']['source_item_id']!=23049:
        raise ValueError('unreviewed Anomaly canonical source body')
    existing['records'][matches[0]]=restore
    appended={family:[row for row in rows if row is not restore] for family,rows in records.items()}
    files={SHARDS[family]:encode({'family':family,'records':rows}) for family,rows in appended.items()}
    files[existing_path]=encode(existing)
    terrain_keys=set()
    for path in (root/'content/world/terrain').glob('terrain-*.json'):
        terrain_keys.update(row['identity']['key'] for row in json.loads(path.read_bytes()).get('records', []))
    terrain_keys.update(row['identity']['key'] for row in records['Terrain'])
    terrain_keys.update(row['identity']['key'] for row in records['WorldObject'])
    files[REGISTRY]=encode({'schema':'OTERYN_ACTIVITY_STATIC_DEFINITIONS/v1', 'technical_terrain_keys':sorted(terrain_keys), 'source_revision':PIN,
        'official_appearances_sha256':OFFICIAL_SHA, 'source_closure':captures[0]['source_closure'], 'records':registry})
    placement_path='content/world/placements/index.json'; placement=json.loads((root/placement_path).read_bytes())
    by_id={row['source_item_id']:row for row in registry}
    for row in placement['palette']:
        if row['source_item_id'] in by_id:
            row.update(key=by_id[row['source_item_id']]['identity']['key'], provisional=False)
    files[placement_path]=encode(placement)
    for family,directory in [('Terrain','terrain'),('WorldObject','objects')]:
        name=f'content/world/{directory}/index.json'; marker=json.loads((root/name).read_bytes())
        marker['notes']=re.sub(r' Additionally SRX-3 pinned activity static geometry: [0-9]+ Terrain and [0-9]+ WorldObject records; no Item aliases\.', '', marker['notes'])+NOTE
        files[name]=(json.dumps(marker,indent=2,ensure_ascii=False)+'\n').encode()
    return files

def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('--check',action='store_true'); args=parser.parse_args()
    if not args.check:
        previous_terrain=ROOT/'content/world/terrain/terrain-08614-08651.json'
        if previous_terrain.exists():
            old=json.loads(previous_terrain.read_bytes())
            if any(row['provenance']['source']['profile'] != 'OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1' for row in old['records']):
                raise ValueError('refusing to replace unrelated static terrain shard')
            previous_terrain.unlink()
        for old_name in ('objects-12932-13052.json','objects-12932-13053.json','objects-12932-13054.json'):
            previous = ROOT/'content/world/objects'/old_name
            if previous.exists():
                old = json.loads(previous.read_bytes())
                if any(row['provenance']['source']['profile'] != 'OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1' for row in old['records']):
                    raise ValueError('refusing to replace unrelated static geometry shard')
                previous.unlink()
    for name,raw in outputs().items():
        path=ROOT/name
        if args.check:
            if not path.exists() or path.read_bytes()!=raw: raise SystemExit(f'stale activity static geometry: {name}')
        else: path.write_bytes(raw)
if __name__=='__main__': main()
