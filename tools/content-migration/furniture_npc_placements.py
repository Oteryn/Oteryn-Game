"""W18: closed source-backed normal furniture retailers on the existing placement owner.

Only normalized cells/identity/provenance are emitted; no Lua, geometry or NPC rule
is executed or invented. Messenger remains on the separate Christmas cadence.
"""
from __future__ import annotations
import copy
import hashlib
import json
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
INPUT = ROOT / 'tools/content-migration/furniture_npc_placements.json'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
def dump(value):
    return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n'
def data():
    value=json.loads(INPUT.read_bytes())
    assert value['schema']==1 and len(value['rows'])==29
    assert len({r['npc'] for r in value['rows']})==29
    assert value['coverage_offer_rows']==904
    return value

def augment_placements(files, frame):
    value=data();index=json.loads(files['index.json']);existing=[]
    for path in index['shards']:
        existing.extend(json.loads(files[Path(path).name])['records'])
    ids={r['declaration']['identity']['key']:r for r in existing}
    cells={tuple(r['declaration']['cell'][c] for c in ('x','y','floor')) for r in existing}
    new=[]
    for row in value['rows']:
        cell=row['cell'];cell_id=tuple(cell[c] for c in ('x','y','floor'))
        assert 0<=cell['x']<=65535 and 0<=cell['y']<=65535 and 0<=cell['floor']<=15
        key=f"oteryn:npc_placement.{row['npc'].removeprefix('oteryn:npc.')}.x{cell['x']}_y{cell['y']}_z{cell['floor']}"
        if key in ids:
            if ids[key]['declaration']['npc']!=row['npc'] or ids[key]['declaration']['cell']!=cell:
                raise ValueError('FURNITURE_NPC_EXISTING_PLACEMENT_CONFLICT')
            continue
        if cell_id in cells:raise ValueError('FURNITURE_NPC_CELL_COLLISION')
        # Definition identity remains pinned Crystal; the independent position
        # evidence is retained verbatim, including Canary John and Wiki Nicholas.
        provenance={'arbitration':[],'definition_sha256':row['definition_sha256'],
            'origin':'crystal','repository':'zimbadev/crystalserver','revision':PIN,
            'source_key':'oteryn:source.crystalserver',
            'source_row':'crystal:npc/'+row['npc'].removeprefix('oteryn:npc.'),
            'position_evidence':copy.deepcopy(row['source'])}
        if row['source']['repository']!='zimbadev/crystalserver':
            provenance['arbitration']=[{'chosen':'canary' if row['source']['repository']=='opentibiabr/canary' else 'wiki',
                'fact':'placements','rule':'CURRENT_RETAIL_SOURCE_POSITION'}]
        new.append({'declaration':{'cell':cell,'direction':row['direction'],
            'identity':{'key':key,'revision':'definition-r1'},'npc':row['npc'],'provenance':[provenance]}})
        cells.add(cell_id)
    if new:
        new.sort(key=lambda r:r['declaration']['identity']['key'])
        name=f'placements-{len(existing):05d}-{len(existing)+len(new)-1:05d}.json'
        files[name]=dump({'coordinate_frame':frame,'family':'Npc.Placement','records':new})
        index['shards'].append('content/world/npc-placements/'+name)
        index['record_count']+=len(new)
    index['npc_count']=len({r['declaration']['npc'] for r in existing+new})
    index['inputs']['furniture_npc_placements_sha256']=hashlib.sha256(INPUT.read_bytes()).hexdigest()
    files['index.json']=dump(index)
    return files

def main():
    # One canonical writer: the existing converter also calls this augmentation
    # after its original base and Summer shards.
    sys.path.insert(0,str(ROOT/'tools/content-schema/npc-authoring'))
    import convert_placements
    convert_placements.main()
if __name__=='__main__':main()
