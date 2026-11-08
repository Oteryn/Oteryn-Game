"""Selected Cook source trashcan: official immovable geometry, never a portable Item."""
import copy
import json
import grizzly_ground as ground
from client_appearance_reader import read_appearances
ROOT = ground.ROOT
IDENTIFIER = 44563
KEY = 'oteryn:terrain.tibia.i44563'
SHARD = 'content/world/terrain/terrain-questpc5-cook-trashcan.json'

def outputs(root=ROOT):
    files = ground.outputs(root)
    record = copy.deepcopy(json.loads(files[ground.SHARD])['records'][0])
    source = record['provenance']['source']
    appearance = read_appearances((root / source['artifact']).read_bytes())[IDENTIFIER]
    if appearance.flags != frozenset({'unmove'}) or appearance.speed is not None:
        raise ValueError('COOK_TRASHCAN_OFFICIAL_FLAGS_DRIFT')
    record['identity']['key'] = KEY
    record['kind'] = dict(state='KNOWN', value='common')
    record.pop('ground_speed')
    record['client_projection']['value']['appearance_id'] = IDENTIFIER
    record['automap'] = dict(state='KNOWN', value=dict(shown=False))
    source['profile'] = 'OTERYN_OFFICIAL_CLIENT_COOK_STATIC_TRASHCAN/v1'
    provenance = record['provenance']
    provenance['source_item_id'] = IDENTIFIER
    provenance['route']['reason'] = 'selected_cook_immovable_geometry'
    provenance['fields'].pop('ground_speed')
    provenance['fields']['kind'] = [f"official_client:{source['revision']}.flags.unmove"]
    index_path = 'content/world/placements/index.json'
    index = json.loads((root / index_path).read_bytes())
    row, = [r for r in index['palette'] if r['source_item_id'] == IDENTIFIER]
    if row['key'] not in (KEY, 'donor:crystalserver@00ce02a5:item/44563'):
        raise ValueError('COOK_TRASHCAN_PALETTE_IDENTITY_DRIFT')
    row.update(key=KEY, provisional=False)
    encode = lambda value: (json.dumps(value, sort_keys=True, separators=(',', ':'))+'\n').encode()
    return {SHARD: encode(dict(family='Terrain', records=[record])), index_path: encode(index)}

if __name__ == '__main__':
    import sys
    for name, raw in outputs().items():
        path = ROOT / name
        if '--check' in sys.argv:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit('stale Cook trashcan: '+name)
        elif not path.exists() or path.read_bytes() != raw:
            path.write_bytes(raw)
