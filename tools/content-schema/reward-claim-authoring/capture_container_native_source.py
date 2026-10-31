"""Capture the exact Crystal201189f chest/handler witnesses used by W15b.

Only source tables are parsed; donor Lua is never executed. The key type witness
is the independently retrieved pinned items.xml blob, not an inferred name.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'quest-authoring'))
import lua_tables

REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
TABLE = 'data-global/startup/tables/chest.lua'
HANDLER = 'data-global/scripts/actions/system/quest_reward_common.lua'
BLOBS = {TABLE: '7b9f823542f9b8d5f99fb6960abe070324c89b8e',
         HANDLER: '709a1d76512b284933a04d707a634d8c538309ce'}


def read(root, path):
    raw = (root / path).read_bytes()
    blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    if blob != BLOBS[path]:
        raise ValueError(f'{path}: not the pinned Crystal201189f Git blob')
    return raw, raw.decode()


def capture(root):
    table_raw, table = read(root, TABLE)
    handler_raw, handler = read(root, HANDLER)
    fields = lua_tables.assignments(table, {'ChestUnique'})['ChestUnique']['fields']
    rows = [{'unique_id': f['key'], 'line': f['line'], 'setting': lua_tables.as_python(f['value'])}
            for f in fields]
    attributes = lua_tables.assignments(handler, {'AttributeTable'})['AttributeTable']['fields']
    text_entries = [{'unique_id': f['key'], 'line': f['line'],
                     'item_id': lua_tables.as_python(f['value'])['itemId'],
                     'text': lua_tables.as_python(f['value'])['text']} for f in attributes]
    leaf_ids = {row['unique_id'] for row in text_entries} - {6195}
    leaf_ids |= {6093, 6082}
    achievements = lua_tables.assignments(handler, {'achievementTable'})['achievementTable']['fields']
    return {
        'schema': 'OTERYN_CRYSTAL_CONTAINER_REWARD_SOURCE/v1',
        'classification': 'OTS_HYPOTHESIS_ONLY', 'repository': 'zimbadev/crystalserver', 'revision': REVISION,
        'source_path': TABLE, 'source_blob_sha1': BLOBS[TABLE],
        'source_sha256': hashlib.sha256(table_raw).hexdigest(),
        'handler_path': HANDLER, 'handler_blob_sha1': BLOBS[HANDLER],
        'handler_sha256': hashlib.sha256(handler_raw).hexdigest(),
        'entries': [row for row in rows if 'container' in row['setting']],
        'shared_choice_entries': [row for row in rows if row['unique_id'] in (6085, 6086, 6087)],
        'leaf_entries': [row for row in rows if row['unique_id'] in leaf_ids],
        'achievement_entries': [{'unique_id': f['key'], 'line': f['line'], 'name': f['value']} for f in achievements],
        'text_entries': text_entries,
        'key_item_evidence': {
            'path': 'data/items/items.xml', 'blob_sha1': 'ea4196beae8f77be898f05de89db81de134886b6',
            'entries': [{'item_id': 2969, 'line': 6502, 'type_line': 6504},
                        {'item_id': 2970, 'line': 6507, 'type_line': 6509},
                        {'item_id': 2971, 'line': 6512, 'type_line': 6514},
                        {'item_id': 2972, 'line': 6517, 'type_line': 6519}],
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    target = Path(__file__).with_name('container_native_source.json')
    text = json.dumps(capture(args.crystal), indent=2, sort_keys=True) + '\n'
    if args.check:
        if target.read_text() != text: parser.exit(1, 'container source capture is stale\n')
    else:
        target.write_text(text)
    print('container source capture: exact Crystal201189f table and handler')


if __name__ == '__main__':
    main()
