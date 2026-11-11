"""Native physical profiles for finite source quest floor puzzles and carried operands."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

import grizzly_shop_profiles as source
from native_profile_composition import encode_native_profile

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/floor-assembly-item-profiles.json'
PROJECTION_IDS = {430, 431, 1772, 1791, 1947, 1948, 2772, 2773, 9125, 9126}
IDS = set(range(3222, 3231)) | set(range(3235, 3242)) | PROJECTION_IDS
XML_SHA = '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'
CLIENT = 'content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'


def source_lines(raw, ids):
    """Pin the defining XML line, including explicit source fromid/toid ranges."""
    found = {}
    for number, line in enumerate(raw.decode().splitlines(), 1):
        match = re.search(r'<item\s+([^>]+)>', line)
        if not match:
            continue
        attributes = dict(re.findall(r'(\w+)="([^"]*)"', match.group(1)))
        if 'id' in attributes:
            first = last = int(attributes['id'])
        elif 'fromid' in attributes and 'toid' in attributes:
            first, last = int(attributes['fromid']), int(attributes['toid'])
        else:
            continue
        for item in ids:
            if first <= item <= last:
                if str(item) in found:
                    raise ValueError('FLOOR_ASSEMBLY_DUPLICATE_SOURCE_LINE')
                found[str(item)] = number
    if set(found) != {str(item) for item in ids}:
        raise ValueError('FLOOR_ASSEMBLY_MISSING_SOURCE_LINE')
    return {str(item): found[str(item)] for item in sorted(ids)}


def capture(xml_path):
    raw = xml_path.read_bytes()
    bank = (ROOT / CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA or hashlib.sha256(bank).hexdigest() != Path(CLIENT).stem.removeprefix('appearances-'):
        raise ValueError('FLOOR_ASSEMBLY_SOURCE_PIN')
    native = source.source
    xml = native.xml_index(raw)
    objects = native.engine_items.load_appearance_objects(bank)
    physical = {r['server_item_id']: r for r in native.policies(bank, raw, IDS)}
    bindings = native.binding_index((ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())
    keys = {f'oteryn:item.tibia.i{i}' for i in IDS}
    definitions = {}
    for path in (ROOT / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in keys:
                definitions[definition['identity']['key']] = definition
    records = []
    for item in sorted(IDS):
        definition = definitions[f'oteryn:item.tibia.i{item}']
        row = native.profile(item, xml[item], objects[item], physical[item], bindings, None)
        if item in PROJECTION_IDS:
            # These canonical client stubs are stationary World operands, not MINT
            # authority. Keep their nonmaterializable admission while qualifying
            # exact collision/appearance facts for native map projections.
            if (definition.get('materializable') is not False
                    or row['admission'] != {'materializable': True,
                        'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}):
                raise ValueError('FLOOR_PROJECTION_MUST_REMAIN_STATIONARY')
            row['admission']['materializable'] = False
        else:
            for group in ('physical', 'stack', 'container'):
                if source.known_conflict(definition['semantics'][group], row['semantics'][group]):
                    raise ValueError('FLOOR_ASSEMBLY_CANONICAL_CONFLICT')
            row['semantics'] = source.accepted_first(definition['semantics'], row['semantics'])
        records.append(native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]}, bindings, {item})['records'][0])
    return {'schema': 'OTERYN_NATIVE_FLOOR_ASSEMBLY_PROFILES/v1',
            'source_revision': '201189f871d23597b82caf88fd611454a2ddefb8',
            'source_xml_sha256': XML_SHA, 'client_sha256': hashlib.sha256(bank).hexdigest(),
            'source_lines': source_lines(raw, IDS),
            'records': records}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = {r['production_definition']['production_key'] for r in records}
    if (overlay['source_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8' or overlay['source_xml_sha256'] != XML_SHA
            or overlay['schema'] != 'OTERYN_NATIVE_FLOOR_ASSEMBLY_PROFILES/v1') or keys != {f'oteryn:item.tibia.i{i}' for i in IDS} or len(records) != len(IDS):
        raise ValueError('FLOOR_ASSEMBLY_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    for record in records:
        key = record['production_definition']['production_key']
        if key in existing:
            if existing[key] != record:
                raise ValueError('FLOOR_ASSEMBLY_PROFILE_CONFLICT')
        else:
            document['records'].append(record)
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
    if args.check:
        if (ROOT / PATH).read_bytes() != data:
            raise SystemExit('FLOOR_ASSEMBLY_PROFILE_DRIFT')
    else:
        (ROOT / PATH).write_bytes(data)
