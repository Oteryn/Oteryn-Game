"""Capture source-owned empty-container stock without changing binary map assets."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tarfile

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'tools/content-schema/world-authoring/samples/quest-container-stock.json'
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'


def document(source_directory):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
    import lua_tables
    sources = []
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))

        def source(path):
            row, = [r for r in manifest['files'] if r['path'] == path and r['revision'] == REVISION]
            raw = corpus.extractfile(row['cache_path']).read()
            assert hashlib.sha256(raw).hexdigest() == row['sha256']
            assert hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() == row['git_blob_sha1']
            sources.append({k: row[k] for k in ('repository', 'revision', 'path', 'sha256', 'git_blob_sha1')})
            return raw.decode()

        containers = lua_tables.assignments(source('data-global/startup/tables/container_misc.lua'), {'MapContainersMisc'})['MapContainersMisc']
        row, = [f for f in containers['fields'] if lua_tables.as_python(f['value']).get('itemPos') == dict(x=32794,y=31572,z=7)]
        assert lua_tables.as_python(row['value']) == dict(itemId=5972,itemPos=dict(x=32794,y=31572,z=7),contents=[dict(itemId=2969)])
        loader = source('data-global/startup/others/functions.lua')
        assert 'if container:getSize() > 0 then' in loader
        assert 'created = created + fillContainer(container, value.contents)' in loader
        storage = lua_tables.assignments(source('data-global/lib/core/storages.lua'), {'Storage'})['Storage']
        for name in ('Quest', 'Key'):
            storage = next(f['value'] for f in storage['fields'] if f['key'] == name)
        number, = [f['value'] for f in storage['fields'] if f['key'] == 'ID3001']
        assert number == 3001
        keys = lua_tables.assignments(source('data-global/startup/tables/door_key.lua'), {'KeyDoorAction'})['KeyDoorAction']
        key, = [f for f in keys['fields'] if isinstance(f['key'],dict) and f['key'].get('expr') == 'Storage.Quest.Key.ID3001']
        assert dict(itemId=2969,itemPos=[dict(x=32794,y=31572,z=7)]) in lua_tables.as_python(key['value'])
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    from build_tool_item_profiles import build
    profile = build(source_directory, {5972})
    semantics = profile['records'][0]['semantics']
    assert semantics['classification']['value']['item_type']['value'] == 'CONTAINER'
    capacity = semantics['container']['value']['capacity']['value']
    assert capacity == 10
    sources.extend(dict(repository='opentibiabr/canary', revision=profile['source_revision'], **row) for row in profile['source_files'])
    index = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())
    region, = [r for r in index['regions'] if r['path'].endswith(codec.region_name(7,32794//256,31572//256))]
    native = (ROOT / region['path']).read_bytes()
    assert hashlib.sha256(native).hexdigest() == region['sha256']
    tile, = [t for _, ts in codec.decode_region(native)[3] for t in ts if t[:2] == (32794,31572)]
    assert tile[3] == 0
    roots = [(i,a) for i,(p,d,a) in enumerate(tile[-1]) if d == 0 and index['palette'][p]['source_item_id'] == 5972]
    (at,attrs), = roots
    assert attrs is None
    assert at+1 == len(tile[-1]) or tile[-1][at+1][1] == 0
    ordinal = sum(d == 0 for _,d,_ in tile[-1][:at])
    assert ordinal == 1 and sum(d == 0 for _,d,_ in tile[-1]) < 64
    return dict(schema=1,classification='OTS_HYPOTHESIS_ONLY',sources=sources,bindings=[dict(position=[32794,31572,7],container=5972,ordinal=ordinal,stock_ordinal=63,item=2969,count=1,key_number=3001,capacity=capacity,source_line=row['line'],key_line=key['line'],map_path=region['path'],map_sha256=region['sha256'])])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = json.dumps(document(args.source_directory),indent=2,sort_keys=True)+'\n'
    if args.check:
        assert OUTPUT.read_text() == raw, 'container stock capture drift'
    else:
        OUTPUT.write_text(raw)
    print('one source-owned empty shared container stock verified')


if __name__ == '__main__':
    main()
