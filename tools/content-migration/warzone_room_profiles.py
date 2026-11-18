"""Capture the stationary crystal decorations on the Count's Warzone route."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import grizzly_shop_profiles as source
from floor_assembly_profiles import CLIENT, source_lines
from native_profile_composition import encode_native_profile

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/warzone-room-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
XML_SHA = 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb'
SCHEMA = 'OTERYN_NATIVE_WARZONE_ROOM_PROFILES/v1'
# Crystal's Count portal, return teleporter destination and timeout exit.
DECORATIONS = {15853, 15857, 15858, 15860}


def capture(xml_path):
    raw = xml_path.read_bytes()
    bank = (ROOT / CLIENT).read_bytes()
    bank_sha = hashlib.sha256(bank).hexdigest()
    if hashlib.sha256(raw).hexdigest() != XML_SHA or bank_sha != client_sha():
        raise ValueError('Warzone stationary decoration source pin drift')
    native = source.source
    xml = native.xml_index(raw)
    objects = native.engine_items.load_appearance_objects(bank)
    bindings = native.binding_index((ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())
    records = []
    for physical in native.policies(bank, raw, DECORATIONS):
        item = physical['server_item_id']
        if (any(physical[key] for key in ('ground', 'block_solid', 'block_projectile',
                                        'movable', 'pickupable', 'container', 'dynamic_kind'))
                or not objects[item]['flags'].get('flags.clip')):
            raise ValueError(f'Warzone Item {item} is no longer a walkable stationary overlay')
        record = native.profile(item, xml[item], objects[item], physical, bindings, None)
        if record['admission']['stack_class'] != 'NonStackable':
            raise ValueError(f'Warzone stationary Item {item} has a stack policy')
        record['admission']['materializable'] = False
        records.append(record)
    document = native.bind_profiles(
        {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
        bindings, DECORATIONS,
    )
    return {
        'schema': SCHEMA,
        'source_revision': REVISION,
        'source_xml_sha256': XML_SHA,
        'client_sha256': bank_sha,
        'source_lines': source_lines(raw, DECORATIONS),
        'records': document['records'],
    }


def client_sha():
    return Path(CLIENT).stem.removeprefix('appearances-')


def admit(root, generated, profile_path, manifest_path, order, canonical):
    capture = json.loads((root / PATH).read_bytes())
    records = capture['records']
    keys = {r['production_definition']['production_key'] for r in records}
    if (capture['schema'] != SCHEMA or capture['source_revision'] != REVISION
            or capture['source_xml_sha256'] != XML_SHA or capture['client_sha256'] != client_sha()
            or keys != {f'oteryn:item.tibia.i{item}' for item in DECORATIONS}
            or len(records) != len(DECORATIONS)):
        raise ValueError('Warzone stationary decoration capture drift')

    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key'] for r in document['records']}
    if keys & existing:
        raise ValueError(f'Warzone stationary decoration profile already registered: {keys & existing}')
    stationary = set()
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in keys and definition.get('materializable') is False:
                stationary.add(definition['identity']['key'])
    if stationary != keys:
        raise ValueError('Warzone stationary decorations must retain nonmaterializable identities')
    for record in records:
        if (record['admission'] != {'materializable': False, 'stack_class': 'NonStackable',
                                    'legal_destinations': ['Ground']}
                or record['attributes']['blocks_movement'] is not False
                or record['attributes']['blocks_projectile'] is not False):
            raise ValueError('Warzone stationary decoration policy drift')
    document['records'].extend(records)
    document['records'].sort(key=order)
    data = encode_native_profile(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_xml', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(capture(args.source_xml), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != data:
            raise SystemExit('Warzone stationary decoration capture drift')
    else:
        path.write_bytes(data)
