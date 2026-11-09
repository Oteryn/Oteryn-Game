"""Finite chosen Oteryn restoration: four shrine props and the archived vault."""
import hashlib
import json
import sys
from pathlib import Path
import world_region_codec as codec
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/reward-claim-authoring'))
import engclaim_native as native
INDEX='content/world/placements/index.json'
SUMMARY='tools/content-schema/world-authoring/samples/world-base-capture-v1.json'

def donor_delta(summary):
    """M34 adds eight items, no tiles (9af2c9fcfd versus its parent)."""
    declared = summary.get('source', {}).get('engclaim')
    if declared is None:
        return 0
    path = 'tools/content-schema/reward-claim-authoring/engclaim_native.py'
    expected = {'path': path, 'sha256': hashlib.sha256((ROOT / path).read_bytes()).hexdigest()}
    if declared != expected:
        raise ValueError('ENGCLAIM_PLACEMENT_SOURCE_CHANGED')
    return 8

def overlay_outputs(outputs):
    index=json.loads(outputs[INDEX]); palette=index['palette']
    ids={p['source_item_id']:i for i,p in enumerate(palette)}
    # All appearances already exist in the map palette. No identity is minted.
    edits={}
    for quest,_,pos,_,appearance,*_ in native.CASES:
        if quest=='mystery_of_the_valley': edits[pos]=('append',appearance)
    for x in range(32410,32414):
        for y in range(32245,32248): edits[(x,y,15)]=('floor',351)
    edits[(32411,32247,15)]=('replace',2472)
    edits[(32412,32247,15)]=('replace',2472)
    edits[(32411,32246,15)]=('replace',1948)
    edits[(32411,32246,14)]=('replace',432)
    seen=set();item_delta=0;byte_delta=0
    for region in index['regions']:
        path=region['path']; wanted={p:v for p,v in edits.items() if codec.region_name(p[2],p[0]//256,p[1]//256)==Path(path).name}
        if not wanted: continue
        raw=outputs[path];z,rx,ry,table=codec.decode_region(raw);sectors={};delta=0
        for local,tiles in table:
            altered=False
            for tile in tiles:
                x,y,flags,house,zones,items=tile;action=wanted.get((x,y,z))
                if action is None: continue
                seen.add((x,y,z));kind,item=action
                if house or zones: raise ValueError('ENGCLAIM_PLACEMENT_OWNERSHIP')
                old=len(items)
                if kind=='append':
                    if len(items)==1: items.append((ids[item],0,None));altered=True
                    elif len(items)!=2 or palette[items[1][0]]['source_item_id']!=item or items[1][2]: raise ValueError('ENGCLAIM_SHRINE_CONFLICT')
                else:
                    expected=[(ids[351],0,None)] + ([] if kind=='floor' else [(ids[item],0,None)])
                    if items!=expected:
                        if any(attrs or depth for _,depth,attrs in items) or any(palette[i]['source_item_id'] not in (1128,351,352,353,item) for i,_,_ in items): raise ValueError(f'ENGCLAIM_VAULT_CONFLICT:{x},{y},{z}')
                        items[:]=expected;altered=True
                delta+=len(items)-old
            sectors[local]=codec.encode_sector_tiles(rx*8+local%8,ry*8+local//8,tiles)
        new=codec.encode_region(z,rx,ry,sectors)
        outputs[path]=new;region['sha256']=hashlib.sha256(new).hexdigest();region['items']+=delta
        byte_delta+=len(new)-len(raw);item_delta+=delta
    if seen!=edits.keys(): raise ValueError(f'ENGCLAIM_MISSING_PLACEMENT:{edits.keys()-seen}')
    index['totals']['items']+=item_delta
    index['source']['engclaim']={'path':'tools/content-schema/reward-claim-authoring/engclaim_native.py','sha256':hashlib.sha256((ROOT/'tools/content-schema/reward-claim-authoring/engclaim_native.py').read_bytes()).hexdigest()}
    outputs[INDEX]=native.canonical(index)
    summary=json.loads(outputs[SUMMARY]);summary['source']=index['source'];summary['totals']['items']+=item_delta;summary['bytes_on_disk']+=byte_delta
    outputs[SUMMARY]=native.canonical(summary)

if __name__=='__main__':
    index=json.loads((ROOT/INDEX).read_bytes());paths=[INDEX,SUMMARY,*[r['path'] for r in index['regions']]]
    outputs={p:(ROOT/p).read_bytes() for p in paths};overlay_outputs(outputs)
    for path,raw in outputs.items():
        if (ROOT/path).read_bytes()!=raw: (ROOT/path).write_bytes(raw)
