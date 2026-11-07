"""Compile selected pinned startup selectors without rewriting source World regions.

Reuses QENGINE's startup-table projection. The bundle compiler owns the derived
attributes; B3 source bytes and their capture checksums remain unchanged.
"""
import argparse
import hashlib
import json
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'content/world/quest-startup-bindings.json'
CORPUS = ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import lua_tables

# Complete existing shared-access bindings plus the active L5 source objects.
REQUESTED = {
    'lever.lua': {'LeverUnique': {30004, 30005, 30025, 30034}},
    'tile.lua': {'TileUnique': {25010, 25030}, 'TileAction': {50022}},
    'teleport.lua': {'TeleportUnique': {3059, 3060, 3061}, 'TeleportAction': {50022}},
    'corpse.lua': {'CorpseUnique': {20003}},
    'chest.lua': {'ChestUnique': {5016, 5017, 6014, 6015, 6029, 6030, 6031, 6076, 6113, 6139, 6140, 6141, 6151, 6152, 6214}},
}


def document():
    sources, bindings = [], []
    with tarfile.open(CORPUS) as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        for name, groups in REQUESTED.items():
            path = 'data-global/startup/tables/' + name
            row, = [r for r in manifest['files'] if r['source'] == 'crystalserver'
                    and r['revision'] == PIN and r['path'] == path]
            raw = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
            digest = hashlib.sha256(raw).hexdigest()
            if digest != row['sha256']:
                raise ValueError('startup donor digest changed: ' + path)
            sources.append(dict(path=path, sha256=digest,
                git_blob_sha1=hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()))
            tables = lua_tables.assignments(raw.decode(), set(groups))
            for group, ids in groups.items():
                found = set()
                for field in tables[group]['fields']:
                    if not isinstance(field['key'], int) or field['key'] not in ids:
                        continue
                    found.add(field['key'])
                    value = lua_tables.as_python(field['value'])
                    for item in value if isinstance(value, list) else [value]:
                        positions = item['itemPos']
                        for position in positions if isinstance(positions, list) else [positions]:
                            bindings.append(dict(position=[position[k] for k in 'xyz'],
                                item=item['itemId'], attribute='unique' if group.endswith('Unique') else 'action',
                                value=field['key'], source=path, line=field['line']))
                if found != ids:
                    raise ValueError(f'incomplete startup source group {group}: {ids-found}')
    bindings.sort(key=lambda r: (r['position'], r['item'], r['attribute']))
    return dict(schema=1, repository='zimbadev/crystalserver', revision=PIN,
                evidence_class='OTS_HYPOTHESIS_ONLY', sources=sources, bindings=bindings)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    text = json.dumps(document(), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text() != text:
            raise ValueError('stale quest startup source bindings')
    else:
        OUTPUT.write_text(text)


if __name__ == '__main__':
    main()
