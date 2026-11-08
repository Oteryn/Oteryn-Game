"""Extend the shared capture with exact Crystal201189f Yalahar startup facts."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

import quest_access_bindings as access

sys.path.insert(0, str(access.ROOT / 'tools/content-schema/quest-authoring'))
import lua_tables


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal-root', required=True, type=Path)
    args = parser.parse_args()
    path = 'data-global/startup/tables/item.lua'
    raw = (args.crystal_root / path).read_bytes()
    blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    if blob != '5d5323401d243df8416a38aba33c3d6e738b2d1a':
        raise ValueError('item.lua is not the pinned Crystal201189f source blob')
    capture = json.loads((access.ROOT / access.CAPTURE).read_bytes())
    capture['sources'] = [row for row in capture['sources'] if row['path'] != path]
    capture['sources'].append({'path': path, 'sha256': hashlib.sha256(raw).hexdigest(),
                               'git_blob_sha1': blob})
    capture['bindings'] = [row for row in capture['bindings'] if row['source'] != path]
    ids = {1018, 1019, *range(3071, 3075), *range(3091, 3103)}
    for field in lua_tables.assignments(raw.decode(), {'ItemUnique'})['ItemUnique']['fields']:
        if field['key'] not in ids:
            continue
        item = lua_tables.as_python(field['value'])
        capture['bindings'].append({'position': [item['itemPos'][k] for k in 'xyz'],
                                    'item': item['itemId'], 'attribute': 'unique',
                                    'value': field['key'], 'source': path, 'line': field['line']})
    target = access.ROOT / access.CAPTURE
    target.write_text(json.dumps(capture, indent=2) + '\n')
    if access.capture_sources(args.crystal_root) != capture:
        raise ValueError('full shared startup capture does not reproduce')


if __name__ == '__main__':
    main()
