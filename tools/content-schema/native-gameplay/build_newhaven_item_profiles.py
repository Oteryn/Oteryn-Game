#!/usr/bin/env python3
"""Normalize missing starter operands, preserving already admitted wiki stats.

Crystal 201189f data-global/lib/others/newhaven.lua:3-142 selects the operands.
Only pinned source flags supply physical facts; UNKNOWN never means false.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
IDS = {
    3003, 3049, 3052, 3059, 3066, 3074, 3272, 3274, 3285, 3286, 3338, 3350,
    3355, 3375, 3409, 3462, 3552, 3559, 3561, 3571, 3723, 7991, 7992, 8095,
    21348, 21350, 21351, 21400, 34017, 35562, 50166, 50257, 51119, 51760,
}
OUTPUT = ROOT / 'tools/content-schema/native-gameplay/newhaven-item-profiles.json'

CRYSTAL_XML_SHA256 = '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'

def fill_unknown(admitted, source):
    """Keep admitted facts, including wiki defense13 over Crystal defense8 (Q11)."""
    if isinstance(admitted, dict) and admitted.get('state') == 'UNKNOWN':
        return copy.deepcopy(source)
    if isinstance(admitted, dict) and isinstance(source, dict):
        return {k: fill_unknown(v, source[k]) if k in source else copy.deepcopy(v)
                for k, v in admitted.items()}
    if isinstance(admitted, list) and isinstance(source, list) and len(admitted) == len(source):
        return [fill_unknown(a, b) for a, b in zip(admitted, source)]
    return copy.deepcopy(admitted)


def build(directory, crystal_xml):
    xml = crystal_xml.read_bytes()
    appearances = (directory / 'appearances.dat').read_bytes()
    proof = json.loads((ROOT / 'tools/content-schema/native-gameplay/canary-ground-proof.json').read_bytes())
    digest = next(r['sha256'] for r in proof['sources'] if r['path'] == 'data/items/appearances.dat')
    if hashlib.sha256(appearances).hexdigest() != digest:
        raise ValueError('NEWHAVEN_APPEARANCE_SOURCE_PIN')
    # Exact Crystal blob at the task's immutable201189f source pin.
    if hashlib.sha256(xml).hexdigest() != CRYSTAL_XML_SHA256:
        raise ValueError('NEWHAVEN_ITEM_XML_SOURCE_PIN')
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {p['server_item_id']: p for p in native.policies(appearances, xml, IDS)}
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    admitted = {}
    for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            d = row['definition']
            if d['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in IDS}:
                admitted[d['identity']['key']] = d['semantics']
    records = []
    for item in sorted(IDS):
        node = copy.deepcopy(index[item])
        # Source spellbooks shield in the same right-hand slot; the runtime uses SHIELD.
        if item == 21400:
            node.find("attribute[@key='weaponType']").set('value', 'shield')
        row = native.profile(item, node, objects[item], physical[item], bindings, None)
        semantics = fill_unknown(admitted[f'oteryn:item.tibia.i{item}'], row['semantics'])
        # Existing canonical equipment patterns own slots/reservations. Source scripts
        # declare no minimum for these starter operands except the two level1 wands.
        for pattern in semantics.get('equipment', {}).get('value', {}).get('patterns', {}).get('value', []):
            if pattern['level']['state'] == 'UNKNOWN':
                pattern['level'] = native.K(1 if item in (21348, 21350) else 0)
            if pattern['vocations']['state'] == 'UNKNOWN':
                pattern['vocations'] = native.K([])
            if pattern['compatibility_rule']['state'] == 'UNKNOWN':
                pattern['compatibility_rule'] = copy.deepcopy(native.NA)
        row['semantics'] = semantics
        records.append(row)
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, set(index))
    return {'schema': 'OTERYN_NATIVE_NEWHAVEN_ITEM_PROFILES/v1',
            'source_revision': '201189f871d23597b82caf88fd611454a2ddefb8',
            'source_files': [{'path': 'data/items/items.xml', 'sha256': hashlib.sha256(xml).hexdigest()},
                             {'path': 'data/items/appearances.dat', 'revision': native.CANARY_PIN, 'sha256': digest}],
            'known_difference': 'Owner Q11: Item21400 keeps admitted wiki defense13; Crystal items.xml42619 says8. Spellbook SHIELD classification follows source shield slot42622.',
            'records': bound['records']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('crystal_xml', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(build(args.source_directory, args.crystal_xml), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit('NEWHAVEN_ITEM_PROFILE_DRIFT')
    else:
        OUTPUT.write_bytes(data)
