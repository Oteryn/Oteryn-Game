#!/usr/bin/env python3
"""Runner-only source facts extraction; never emits source asset bytes."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import urllib.request

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE_PATH = 'data-global/world/world.otbm'
BLOB = 'e61658ec0ab7b70ec35025ae969426d6fc57ae75'
SHA256 = 'c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85'
SIZE = 53471153
DECODER_SHA256 = 'c8c279d0ee93188cf2b10cdd9e299724f033f75d96f1beb17c577696b1436333'
URL = f'https://raw.githubusercontent.com/zimbadev/crystalserver/{PIN}/{SOURCE_PATH}'
AIDS = (24868, 9299, 14332, 14324, 14352, 14343, 14348)
POSITIONS = {
    (33718, 32047, 15): (1949, 22761),
    (32245, 31244, 14): None,
    (32309, 31247, 14): None,
    (32261, 31250, 14): (23482, 23483),
    (32281, 31348, 14): (23482, 23483),
    (32326, 31250, 14): (23482, 23483),
    # Exact Eradicator onUse literals;14348 is the OUTPUT portal, not lever AID.
    (32334, 31283, 14): None,
    (32334, 31284, 14): None,
    (32334, 31285, 14): None,
    (32334, 31286, 14): None,
    (32334, 31287, 14): None,
    (32334, 31288, 14): None,
    (32309, 31290, 14): None,
    (32309, 31283, 14): None,
    (32318, 31284, 14): None,
    (32218, 31375, 11): None,
}

def extract(raw, otbm):
    rows = []
    selected_tiles = {}
    def on_tile(x, y, z, flags, house, zones, items):
        position = (x, y, z)
        if position in POSITIONS:
            if position in selected_tiles:
                raise ValueError('duplicate selected source tile')
            selected_tiles[position] = {
                'source_position': list(position), 'present': True,
                'item_count': len(items), 'tile_flags': flags,
                'house_id': house, 'zones': list(zones),
            }
        for ordinal, (item, depth, attrs) in enumerate(items):
            aid = attrs.get(4)
            position = (x, y, z)
            selected_position = position in POSITIONS and (POSITIONS[position] is None or item in POSITIONS[position])
            if aid not in AIDS and not selected_position:
                continue
            rows.append({'source_position': [x,y,z], 'server_item_id': item,
                         'depth': depth, 'source_preorder_ordinal': ordinal,
                         'action_id': aid, 'unique_id': attrs.get(5),
                         'teleport_destination': attrs.get('dest'),
                         'otbm_attributes': {str(k): v for k,v in attrs.items()},
                         'tile_flags': flags, 'house_id': house, 'zones': list(zones)})
            if len(rows) > 512:
                raise ValueError('bounded output capacity exceeded (512 facts)')
    facts = otbm.read_tiles(raw, on_tile)
    return {'rows': sorted(rows, key=lambda r:(r['source_position'],r['source_preorder_ordinal'])),
            'selected_tiles': [selected_tiles.get(position, {
                'source_position': list(position), 'present': False,
            }) for position in sorted(POSITIONS)],
            'tiles_scanned': facts.tiles,
            'unknown_item_attributes': dict(facts.unknown_item_attrs),
            'unknown_tile_attributes': dict(facts.unknown_tile_attrs),
            'absence_claims_qualified': not (facts.unknown_item_attrs or facts.unknown_tile_attrs),
            'counts': {str(a): sum(r['action_id'] == a for r in rows) for a in AIDS}}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--repository', type=Path, required=True)
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    decoder = args.repository / 'tools/content-schema/world-authoring/otbm_reader.py'
    if hashlib.sha256(decoder.read_bytes()).hexdigest() != DECODER_SHA256:
        raise ValueError('reviewed decoder hash mismatch')
    spec = importlib.util.spec_from_file_location('maxxen_otbm', decoder)
    otbm = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = otbm
    spec.loader.exec_module(otbm)
    # Never persist the source asset. Bound transport before decode; reject LFS pointers.
    with urllib.request.urlopen(URL, timeout=60) as response:
        raw = response.read(SIZE + 1)
    if len(raw) != SIZE or hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('pinned raw asset size/SHA256 mismatch')
    if hashlib.sha1(b'blob ' + str(SIZE).encode() + b'\0' + raw).hexdigest() != BLOB:
        raise ValueError('pinned Git blob mismatch')
    result = extract(raw, otbm)
    result['source'] = {'repository': 'https://github.com/zimbadev/crystalserver',
                        'revision': PIN, 'path': SOURCE_PATH, 'git_blob': BLOB,
                        'sha256': SHA256, 'size': SIZE, 'decoder_sha256': DECODER_SHA256}
    result['scope'] = {'global_action_ids': list(AIDS),
                       'literal_positions': [{'source_position': list(position), 'item_ids': values}
                                             for position, values in sorted(POSITIONS.items())]}
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')

if __name__ == '__main__':
    main()
