"""M50: source-qualified Rafzan elevator ground, retaining its exact Item pointer."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import qengine_reward_item_profiles as shared

SHARD = 'content/world/terrain/terrain-08749-08749.json'
IDENTIFIER = 17970
KEY = 'oteryn:terrain.tibia.i17970'
MARKER_NOTE = ' Additionally one source-qualified Rafzan elevator ground; total 8694 Terrain records.'


def record(root=ROOT):
    xml = (root / shared.XML).read_bytes()
    appearances = (root / shared.CLIENT).read_bytes()
    if (hashlib.sha256(xml).hexdigest() != shared.XML_SHA
            or hashlib.sha256(appearances).hexdigest() != shared.CLIENT_SHA):
        raise ValueError('QM50_GROUND_SOURCE_PIN')
    facts, = shared.source.policies(appearances, xml, {IDENTIFIER})
    if (not facts['ground'] or facts['ground_speed'] != 110
            or facts['movable'] or facts['pickupable'] or facts['block_solid']):
        raise ValueError('QM50_GROUND_PHYSICAL_DRIFT')
    bindings = shared.source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    pointer = bindings[str(IDENTIFIER)]['target']
    if pointer != {'family': 'Item', 'key': 'oteryn:item.tibia.i17970', 'revision': 'definition-r1'}:
        raise ValueError('QM50_GROUND_IDENTITY_DRIFT')
    known = lambda value: {'state': 'KNOWN', 'value': value}
    fields = {
        'kind': known('ground'), 'walkable': known(not facts['block_solid']),
        'ground_speed': known(facts['ground_speed']),
        'blocks_projectile': known(facts['block_projectile']),
        'blocks_sight': {'state': 'UNKNOWN'}, 'floor_change': {'state': 'UNKNOWN'},
        'automap': {'state': 'UNKNOWN'},
        'client_projection': known({'appearance_id': IDENTIFIER}),
    }
    return {'identity': {'family': 'Terrain', 'key': KEY, 'revision': 'definition-r1'},
            **fields, 'provenance': {
                'source': {'engine': 'canary', 'repository': 'opentibiabr/canary',
                           'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                           'profile': 'OTERYN_QM50_ELEVATOR_GROUND/v1',
                           'items_xml_sha256': shared.XML_SHA,
                           'appearances_sha256': shared.CLIENT_SHA},
                'source_item_id': IDENTIFIER, 'item_pointer': pointer,
                'route': {'owner': 'Terrain', 'reason': 'ground_or_border'},
                'fields': {key: ['appearance:official15_30']
                           for key, value in fields.items() if value['state'] == 'KNOWN'}}}


def outputs(root=ROOT):
    marker_path = 'content/world/terrain/index.json'
    marker = json.loads((root / marker_path).read_bytes())
    if MARKER_NOTE not in marker['notes']:
        marker['notes'] += MARKER_NOTE
    return {
        SHARD: (json.dumps({'family': 'Terrain', 'records': [record(root)]},
                           sort_keys=True, separators=(',', ':')) + '\n').encode(),
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + '\n').encode(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f'stale M50 elevator ground: {name}')
        else:
            path.write_bytes(raw)


if __name__ == '__main__':
    main()
