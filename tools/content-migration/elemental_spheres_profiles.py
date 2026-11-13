"""Pinned Elemental Spheres soils, altars and corpse native counterparts."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import rafzane_elevator_profiles as pinned

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/elemental-spheres-item-profiles.json'
SOILS = {940, 941, 944, 945}
CORPSES = {8106, 8139, 8140, 8141}
# The existing ordinary-field cohort already owns energy field 2126.
IDS = SOILS | CORPSES | {7737, 7738, 7739, 7740, 7741, 7742, 7743, 7744, 7745, 7746, 7748}
SCHEMA = 'OTERYN_NATIVE_ELEMENTAL_SPHERES_PROFILES/v1'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'


def capture(xml_path):
    raw = xml_path.read_bytes()
    bank = (ROOT / pinned.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != pinned.XML_SHA or hashlib.sha256(bank).hexdigest() != Path(pinned.CLIENT).stem.removeprefix('appearances-'):
        raise ValueError('ELEMENTAL_SPHERES_SOURCE_PIN')
    native = pinned.source.source
    xml = native.xml_index(raw)
    objects = native.engine_items.load_appearance_objects(bank)
    physical = {r['server_item_id']: r for r in native.policies(bank, raw, IDS)}
    bindings = native.binding_index((ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())
    records = []
    for item in sorted(IDS):
        row = native.profile(item, xml[item], objects[item], physical[item], bindings, None)
        # Crystal movements_summon_overlord.lua consumes these movable corpses
        # through onAddItem. They remain Ground-only, never inventory-admitted.
        if item not in SOILS | CORPSES:
            if row['admission'] != {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}:
                raise ValueError('ELEMENTAL_SPHERES_GROUND_ONLY')
            row['admission']['materializable'] = False
        records.append(native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]}, bindings, {item})['records'][0])
    return {'schema': SCHEMA, 'source_revision': REVISION,
            'source_xml_sha256': pinned.XML_SHA, 'client_sha256': hashlib.sha256(bank).hexdigest(),
            'source_lines': pinned.source_lines(raw, IDS), 'records': records}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = {r['production_definition']['production_key'] for r in records}
    if (overlay['schema'] != SCHEMA or overlay['source_revision'] != REVISION
            or overlay['source_xml_sha256'] != pinned.XML_SHA
            or overlay['client_sha256'] != Path(pinned.CLIENT).stem.removeprefix('appearances-')
            or keys != {f'oteryn:item.tibia.i{i}' for i in IDS} or len(records) != len(IDS)
            or set(overlay['source_lines']) != {str(i) for i in IDS}):
        raise ValueError('ELEMENTAL_SPHERES_PROFILE_SET')
    for record in records:
        soil = record['production_definition']['production_key'] in {f'oteryn:item.tibia.i{i}' for i in SOILS}
        corpse = record['production_definition']['production_key'] in {f'oteryn:item.tibia.i{i}' for i in CORPSES}
        expected = {'materializable': soil or corpse, 'stack_class': 'StackCapable' if soil else 'NonStackable',
                    'legal_destinations': ['CharacterInventory', 'Ground'] if soil else ['Ground']}
        if record['admission'] != expected:
            raise ValueError('ELEMENTAL_SPHERES_ADMISSION')
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    for record in records:
        key = record['production_definition']['production_key']
        if key in existing:
            if existing[key] != record:
                raise ValueError('ELEMENTAL_SPHERES_PROFILE_CONFLICT')
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
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
            raise SystemExit('ELEMENTAL_SPHERES_PROFILE_DRIFT')
    else:
        (ROOT / PATH).write_bytes(data)
