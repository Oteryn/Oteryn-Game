"""Capture the source level25 selector against its exact native Draconia door."""
import argparse
import hashlib
import json
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'tools/content-schema/world-authoring/samples/quest-route-level-door-bindings.json'
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SOURCE = 'data-global/startup/tables/door_level.lua'
SHA = 'd8868f675aae7c541ddc3e76b3648b6a886e0ae31a4a614747d17e1784d234fc'
BLOB = '0d1a38cf3b0309d81ce8a08c6fc495df74c86444'


def document():
    sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
    import lua_tables
    import world_region_codec as codec
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        raw = corpus.extractfile('blobs/' + BLOB).read()
    assert hashlib.sha256(raw).hexdigest() == SHA
    assert hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() == BLOB
    table = lua_tables.assignments(raw.decode(), {'LevelDoorAction'})['LevelDoorAction']
    field, = [f for f in table['fields'] if f['key'] == 1025]
    groups = lua_tables.as_python(field['value'])
    selected = [group for group in groups if group['itemId'] is False]
    assert selected == [{'itemId': False, 'itemPos': [{'x': 32804, 'y': 31583, 'z': 2}]}]
    index = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())
    region, = [r for r in index['regions'] if r['path'].endswith(codec.region_name(2, 32804 // 256, 31583 // 256))]
    native = (ROOT / region['path']).read_bytes()
    assert hashlib.sha256(native).hexdigest() == region['sha256']
    tile, = [tile for _, tiles in codec.decode_region(native)[3] for tile in tiles if tile[:2] == (32804, 31583)]
    assert tile[3] == 0
    roots = [(at, attrs) for at, (palette, depth, attrs) in enumerate(tile[-1]) if depth == 0 and index['palette'][palette]['source_item_id'] == 5102]
    assert len(roots) == 1
    at, attrs = roots[0]
    assert attrs is None
    assert at + 1 == len(tile[-1]) or tile[-1][at + 1][1] == 0
    return dict(schema=1, classification='OTS_HYPOTHESIS_ONLY', sources=[dict(repository='zimbadev/crystalserver', revision=REVISION, path=SOURCE, sha256=SHA, git_blob_sha1=BLOB)], bindings=[dict(position=[32804,31583,2], item=5102, attribute='action', value=1025, source_line=field['line'], map_path=region['path'], map_sha256=region['sha256'])])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    text = json.dumps(document(), indent=2, sort_keys=True) + '\n'
    if args.check:
        assert OUTPUT.read_text() == text, 'level gate capture drift'
    else:
        OUTPUT.write_text(text)
    print('one exact source level25 door binding verified')


if __name__ == '__main__':
    main()
