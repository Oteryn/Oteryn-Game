"""Capture exact quest key/door startup bindings against pinned native map entries.

Reuses questpc6's quest_key_door_bindings.py source custody and native pin checks.
Only the selected capture is emitted; unrelated source routes retain their own batches.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path
import sys
import tarfile

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'tools/content-schema/world-authoring/samples/quest-stock-key-bindings.json'
DOOR_OUTPUT = ROOT / 'tools/content-schema/world-authoring/samples/quest-route-key-door-bindings.json'
DOOR_SELECTED = ('ID3001', 'ID3002', 'ID3003', 'ID3012')
CORPUS = ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SELECTED = tuple(f'ID{number}' for number in range(3003, 3008))
TABLE = 'data-global/startup/tables/door_key.lua'
STORAGES = 'data-global/lib/core/storages.lua'
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import lua_tables


def document(capture="stock"):
    selected = SELECTED if capture == "stock" else DOOR_SELECTED
    sources = []
    with tarfile.open(CORPUS) as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))

        def source(path):
            row, = [f for f in manifest['files'] if f['source'] == 'crystalserver'
                    and f['revision'] == REVISION and f['path'] == path]
            raw = archive.extractfile(row['cache_path']).read()
            if (hashlib.sha256(raw).hexdigest() != row['sha256'] or
                    hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
                    != row['git_blob_sha1']):
                raise ValueError('stock key source digest mismatch')
            sources.append({k: row[k] for k in
                            ('repository', 'revision', 'path', 'sha256', 'git_blob_sha1')})
            return raw.decode()

        table = lua_tables.assignments(source(TABLE), {'KeyDoorAction'})['KeyDoorAction']
        storage = lua_tables.assignments(source(STORAGES), {'Storage'})['Storage']
    for key in ('Quest', 'Key'):
        storage = next(f['value'] for f in storage['fields'] if f['key'] == key)
    numbers = {f['key']: (f['value'], f['line']) for f in storage['fields']}
    index = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())
    registry = (ROOT / 'crates/gameplay/world/src/map/door_registry.rs').read_text()
    triples = registry.split('const KEY_DOORS:', 1)[1].split('];', 1)[0]
    locked = {int(a) for a, _, _ in re.findall(r'\((\d+), (\d+), (\d+)\)', triples)}
    bindings = []
    for field in table['fields']:
        expression = field['key'].get('expr') if isinstance(field['key'], dict) else None
        if expression not in {'Storage.Quest.Key.' + key for key in selected}:
            continue
        number, storage_line = numbers[expression.rsplit('.', 1)[1]]
        groups = lua_tables.as_python(field['value'])
        groups = groups if isinstance(groups, list) else [groups]
        targets = [group for group in groups if
                   (group['itemId'] == 2969 if capture == 'stock' else group['itemId'] is False)]
        if len(targets) != 1:
            raise ValueError('selected source binding shape drift')
        for position in targets[0]['itemPos']:
            x, y, z = (position[k] for k in 'xyz')
            region, = [r for r in index['regions'] if
                       r['path'].endswith(codec.region_name(z, x // 256, y // 256))]
            raw = (ROOT / region['path']).read_bytes()
            if hashlib.sha256(raw).hexdigest() != region['sha256']:
                raise ValueError('stock key native map digest mismatch')
            tile, = [tile for _, tiles in codec.decode_region(raw)[3] for tile in tiles
                     if tile[:2] == (x, y)]
            if tile[3] != 0:
                raise ValueError('stock key is a House tile')
            matches = [(at, attrs) for at, (i, depth, attrs) in enumerate(tile[-1])
                       if depth == 0 and (index['palette'][i]['source_item_id'] == 2969
                           if capture == 'stock' else index['palette'][i]['source_item_id'] in locked)]
            if len(matches) != 1:
                raise ValueError('stock key native item missing or ambiguous')
            at, attrs = matches[0]
            if attrs is not None or (at + 1 < len(tile[-1]) and tile[-1][at + 1][1] > 0):
                raise ValueError('stock key has unrelated native attributes or contents')
            bindings.append(dict(position=[x, y, z], item=index['palette'][tile[-1][at][0]]['source_item_id'], attribute='action',
                                 value=number, source_line=field['line'],
                                 storage_line=storage_line, map_path=region['path'],
                                 map_sha256=region['sha256']))
    if len(bindings) != 5 or len({(tuple(b['position']), b['item']) for b in bindings}) != 5:
        raise ValueError('quest key capture incomplete or ambiguous')
    if capture == 'stock' and len({b['value'] for b in bindings}) != len(SELECTED):
        raise ValueError('stock key source number drift')
    if capture == 'doors' and sorted(b['value'] for b in bindings) != [3001, 3002, 3003, 3012, 3012]:
        raise ValueError('key door source number drift')
    return dict(schema=1, classification='OTS_HYPOTHESIS_ONLY', sources=sources,
                bindings=sorted(bindings, key=lambda row: row['position']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--capture', choices=['stock', 'doors'], default='stock')
    args = parser.parse_args()
    output = OUTPUT if args.capture == 'stock' else DOOR_OUTPUT
    expected = json.dumps(document(args.capture), indent=2, sort_keys=True) + '\n'
    if args.check:
        if not output.exists() or output.read_text() != expected:
            raise SystemExit('stock key capture requires regeneration')
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(expected)
    print(f'5 source {args.capture} key bindings verified')


if __name__ == '__main__':
    main()
