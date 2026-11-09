"""M49: qualify the Earth altar's existing Item7748 as its actual ground.

Crystal201189f Elemental Spheres movements select7748. The retained official15.30
appearance independently proves bank=true, unpass=false and speed150. Keep the
Item identity and add its one-to-one Terrain route; no Item MINT admission changes.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from client_appearance_reader import read_appearances

SHARD = 'content/world/terrain/terrain-08693-08693.json'
NOTE = ' Additionally M49 Earth altar ground7748 retains its canonical Item identity.'
DIGEST = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'


def output(root=ROOT):
    raw = (root / f'content/assets/files/appearances-{DIGEST}.dat').read_bytes()
    if hashlib.sha256(raw).hexdigest() != DIGEST:
        raise ValueError('M49 official appearance digest')
    appearance = read_appearances(raw)[7748]
    if 'bank' not in appearance.flags or 'unpass' in appearance.flags or appearance.speed != 150:
        raise ValueError('M49 Earth ground differs from the qualified source')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    binding = next(row for row in bindings['bindings'] if row['external_id'] == '7748')
    if binding['target'] != {'family': 'Item', 'key': 'oteryn:item.tibia.i7748', 'revision': 'definition-r1'}:
        raise ValueError('M49 Earth identity binding')
    known = lambda value: {'state': 'KNOWN', 'value': value}
    record = {
        'identity': {'family': 'Terrain', 'key': 'oteryn:terrain.tibia.i7748', 'revision': 'definition-r1'},
        'kind': known('ground'), 'walkable': known(True), 'ground_speed': known(appearance.speed),
        'blocks_projectile': {'state': 'UNKNOWN'}, 'blocks_sight': {'state': 'UNKNOWN'},
        'floor_change': {'state': 'UNKNOWN'},
        'automap': known({'shown': appearance.automap_color is not None, 'color': appearance.automap_color}),
        'client_projection': known({'appearance_id': 7748}),
        'provenance': {
            'source': {'engine': 'crystal', 'repository': 'zimbadev/crystalserver',
                       'revision': '201189f871d23597b82caf88fd611454a2ddefb8',
                       'profile': 'OTERYN_M49_OFFICIAL_APPEARANCE_CROSS_CHECKED_GROUND/v1'},
            'source_item_id': 7748, 'item_pointer': binding['target'],
            'route': {'owner': 'Terrain', 'reason': 'ground_or_border'},
            'fields': {key: [f'official_client:{DIGEST}.{flag}'] for key, flag in [
                ('kind', 'flags.bank'), ('walkable', 'flags.unpass'), ('ground_speed', 'bank.waypoints'),
                ('automap', 'flags.automap'), ('client_projection', 'id')]},
        },
    }
    return (json.dumps({'family': 'Terrain', 'records': [record]}, indent=2) + '\n').encode()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    path = ROOT / SHARD
    data = output()
    if args.check:
        if path.read_bytes() != data:
            raise SystemExit('M49 Earth ground drift')
    else:
        path.write_bytes(data)
        marker = ROOT / 'content/world/terrain/index.json'
        index = json.loads(marker.read_bytes())
        if NOTE not in index['notes']:
            index['notes'] += NOTE
        marker.write_text(json.dumps(index, indent=2, ensure_ascii=False) + '\n')
