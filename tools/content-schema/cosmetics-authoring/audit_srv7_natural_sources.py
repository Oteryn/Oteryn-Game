#!/usr/bin/env python3
"""Join S06's qualified official operands to real native natural-source placements.

This audits current repository spawn authoring and raid waves. An admitted Item or
Creature identity is not an admitted boss Instance, and this report never infers one.
"""
import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/cosmetics-authoring/srv7-natural-source-reachability.json'
IDS = {12551,12552,12553,12554,12555,12556,23684,23685,27655,27656,27657,32629,32630,32631,34072,34073,34074,34075,34076,35595,35695,36938,37604,37605,39544,39545,43899,43900,44048,50060,50061,50064,50067}


def capture(root=ROOT):
    sources = []
    def read(path):
        raw=(root/path).read_bytes()
        sources.append({'path':path,'sha256':hashlib.sha256(raw).hexdigest()})
        return json.loads(raw)
    loot=read('tools/content-schema/native-gameplay/loot-tables.json')
    raids=read('content/encounters/raids/catalogue.json')
    tables=defaultdict(set)
    for row in loot['tables']:
        for entry in row['entries']:
            key=entry['item']['key']
            if key.startswith('oteryn:item.tibia.i'):
                item=int(key.rsplit('i',1)[1])
                if item in IDS:
                    tables[item].add(row['identity']['key'])
    creatures=defaultdict(set)
    for row in loot['creature_loot']:
        if row['loot']:
            for item in IDS:
                if row['loot']['key'] in tables[item]:
                    creatures[item].add(row['creature']['key'])
    if any(not creatures[item] for item in IDS):
        raise ValueError('SRV7_NATURAL_LOOT_SOURCE_MISSING')
    item_facts={row['item']['key']:row for row in loot['items']}
    wanted=set().union(*creatures.values())
    placements=defaultdict(list)
    for path in sorted((root/'content/world/spawns').glob('spawns-*.json')):
        relative=path.relative_to(root).as_posix()
        for ri,row in enumerate(read(relative)['records']):
            declaration=row['declaration']
            for pi,point in enumerate(declaration.get('points',[])):
                prefix=f'$.records[{ri}].declaration.points[{pi}]'
                variants=point.get('weighted')
                candidates=[(point['creature'],prefix,None)] if not variants else [
                    (variant['creature'],prefix+f'.weighted[{wi}]',variant['weight'])
                    for wi,variant in enumerate(variants) if variant['weight']>0]
                for key,json_path,weight in candidates:
                    if key in wanted:
                        placements[key].append({'path':relative,'json_path':json_path,'spawn':declaration['identity']['key'],'cell':point['cell'],'weight':weight})
    waves=defaultdict(list)
    for ri,raid in enumerate(raids['records']):
        for vi,variant in enumerate(raid.get('variants',[])):
            for wi,wave in enumerate(variant.get('waves',[])):
                for ei,entry in enumerate(wave.get('entries',[])):
                    if entry.get('creature') in wanted:
                        waves[entry['creature']].append({'raid':raid['key'],'active':raid['active'],'inactive_reasons':raid['inactive_reasons'],'json_path':f'$.records[{ri}].variants[{vi}].waves[{wi}].entries[{ei}]','entry':entry,'schedule':raid['schedule']})
    records=[]
    for item in sorted(IDS):
        natural=[{'creature':key,'placement_count':len(placements[key]),'first_placement':placements[key][0] if placements[key] else None,'raid_sources':waves[key]} for key in sorted(creatures[item])]
        ordinary=sum(bool(row['placement_count']) for row in natural)
        active=sum(any(raid['active'] for raid in row['raid_sources']) for row in natural)
        status='CURRENT_STATIC_OR_ACTIVE_RAID' if ordinary or active else 'NO_STATIC_OR_RAID_SOURCE_INSTANCE_ADMISSION_REQUIRED'
        records.append({'item_id':item,'native_item_materializable':item_facts['oteryn:item.tibia.i'+str(item)]['materializable'],'natural_loot_tables':sorted(tables[item]),'source_creatures':natural,'placed_source_creatures':ordinary,'active_raid_source_creatures':active,'status':status})
    return {'schema':1,'classification':'CURRENT_REPOSITORY_SOURCE_REACHABILITY_AUDIT','scope':'native natural loot, ordinary spawn authoring and active/inactive raids; no test rat override and no inferred boss Instance admission','sources':sources,'records':records}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    value=capture()
    raw=(json.dumps(value,indent=2,ensure_ascii=False)+'\n').encode()
    path=ROOT/OUTPUT
    if args.check:
        if path.read_bytes()!=raw:
            raise ValueError('SRV7_NATURAL_SOURCE_CENSUS_DRIFT')
    else:
        path.write_bytes(raw)
    reachable=sum(bool(row['placed_source_creatures'] or row['active_raid_source_creatures']) for row in value['records'])
    print(f'SRV7_NATURAL_SOURCE_CENSUS operands={len(IDS)} static_or_active_raid={reachable} instance_admission_required={len(IDS)-reachable}')


if __name__=='__main__':
    main()
