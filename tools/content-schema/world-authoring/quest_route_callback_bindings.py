"""Capture exact Draconia callback anchors from pinned startup tables and native map."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import lua_tables
import world_region_codec as codec

REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
OUTPUT = ROOT / 'tools/content-schema/world-authoring/samples/quest-route-callback-bindings.json'
REQUESTED = {'lever.lua': ('LeverUnique', {30006, 30035}),
             'teleport.lua': ('TeleportUnique', {35011, 35012, 38004, 38005, 38006}),
             'tile.lua': ('TileUnique', {29002, 29003})}

def document():
    index = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())
    bindings, sources = [], []
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        for name, (group, selected) in REQUESTED.items():
            path = 'data-global/startup/tables/' + name
            row, = [f for f in manifest['files'] if f['source'] == 'crystalserver'
                    and f['revision'] == REVISION and f['path'] == path]
            raw = archive.extractfile(row['cache_path']).read()
            if hashlib.sha256(raw).hexdigest() != row['sha256'] or hashlib.sha1(
                    b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() != row['git_blob_sha1']:
                raise ValueError('callback source digest mismatch')
            sources.append({k: row[k] for k in ('repository','revision','path','sha256','git_blob_sha1')})
            fields = lua_tables.assignments(raw.decode(), {group})[group]['fields']
            for field in fields:
                if not isinstance(field['key'], int) or field['key'] not in selected:
                    continue
                value = lua_tables.as_python(field['value'])
                pos = value['itemPos']
                x, y, z = (pos[k] for k in 'xyz')
                item = value['itemId']
                region, = [r for r in index['regions'] if r['path'].endswith(codec.region_name(z,x//256,y//256))]
                data = (ROOT / region['path']).read_bytes()
                if hashlib.sha256(data).hexdigest() != region['sha256']:
                    raise ValueError('callback native map digest mismatch')
                tile, = [t for _, tiles in codec.decode_region(data)[3] for t in tiles if t[:2] == (x,y)]
                matches = [(depth, attrs) for i,depth,attrs in tile[-1] if index['palette'][i]['source_item_id'] == item]
                if tile[3] or matches != [(0,None)]:
                    raise ValueError('callback native root missing, ambiguous, bound or housed')
                bindings.append(dict(position=[x,y,z], item=item, attribute='unique', value=field['key'], source=path,
                                     source_line=field['line'], map_path=region['path'], map_sha256=region['sha256']))
    if len(bindings) != 9 or {b['value'] for b in bindings} != {30006,30035,35011,35012,29002,29003,38004,38005,38006}:
        raise ValueError('callback capture incomplete')
    return dict(schema=1, classification='OTS_HYPOTHESIS_ONLY',sources=sources,bindings=sorted(bindings,key=lambda b:b['position']))

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    expected=json.dumps(document(),indent=2,sort_keys=True)+'\n'
    if args.check:
        if OUTPUT.read_text()!=expected:raise SystemExit('callback capture requires regeneration')
    else:OUTPUT.write_text(expected)
    print('9 exact native callback anchors verified')
