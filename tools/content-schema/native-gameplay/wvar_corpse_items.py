"""Admit only Mamma/Giant Spider and Bibby's sourced corpse chains through existing Item owners.

Crystal 201189f items.xml:12182-12197,16295-16300 supplies 10/300/300/60s
and 5977->4038->4039->4040. Source capacities 30/30/20 are retained as source
facts and are now admitted exactly. The earlier16-slot native projection is
corrected through the shared source-backed44-entry owner.
The terminal appearance is not a container. WorldObject retains collision and
appearance authority, and no other corpse is promoted.
Combat corpse retirement separately retains D3's existing fixed 60 seconds.
"""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import sys
import tarfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
XML_SHA256 = '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'
DEFAULTS_SHA256 = '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca'
APPEARANCES_SHA256 = '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'
CHAIN = ((5977, 30, 10_000, 4038), (4038, 30, 300_000, 4039),
         (4039, 20, 300_000, 4040), (4040, None, 60_000, None),
         (6008, 10, 10_000, 4148), (4148, 10, 300_000, 4149),
         (4149, 10, 300_000, 4147), (4147, None, 60_000, None))
PRIMARY = frozenset((5977, 6008))
TERMINAL = frozenset((4040, 4147))
WEIGHTS = {4148:120000,4149:120000,4147:10000}
OWNED = frozenset(row[0] for row in CHAIN)
OBJECT_SHARDS = ('content/world/objects/objects-00500-00999.json',
                 'content/world/objects/objects-01500-01999.json',
                 'content/world/objects/objects-01000-01499.json')
UNKNOWN_GROUPS = ('presentation', 'classification', 'equipment', 'weapon',
                  'protection', 'skill_modifiers', 'charges', 'imbuement',
                  'use_transform', 'trade_restrictions', 'fluid', 'readable_writeable')


def verify_source(archive):
    with tarfile.open(archive) as donor:
        def read(path):
            members = [m for m in donor.getmembers() if m.name.endswith('/' + path)]
            if len(members) != 1 or PIN not in members[0].name:
                raise ValueError('WVAR_CORPSE_SOURCE_IDENTITY')
            return donor.extractfile(members[0]).read()
        raw = read('data/items/items.xml')
        if hashlib.sha256(raw).hexdigest() != XML_SHA256:
            raise ValueError('WVAR_CORPSE_XML_HASH')
        xml = ET.fromstring(raw)
        defaults_raw = read('src/items/items.hpp')
        if hashlib.sha256(defaults_raw).hexdigest() != DEFAULTS_SHA256:
            raise ValueError('WVAR_CORPSE_DEFAULTS_HASH')
        defaults = defaults_raw.decode()
        if 'int32_t weight = 0;' not in defaults or 'bool stopTime = false;' not in defaults:
            raise ValueError('WVAR_CORPSE_SOURCE_DEFAULTS')
        for item, capacity, duration, target in CHAIN:
            row = xml.find(f'item[@id="{item}"]')
            attributes = {a.attrib['key'].lower(): a.attrib['value'] for a in row}
            expected_capacity = {5977: '30', 4038: '30', 4039: '20',6008:'10',4148:'10',4149:'10'}.get(item)
            if (attributes.get('duration'), attributes.get('decayto'), attributes.get('containersize')) != (
                    str(duration // 1000), str(target or 0), expected_capacity):
                raise ValueError('WVAR_CORPSE_XML_FIELDS')
            if (int(attributes.get('weight','0')) != WEIGHTS.get(item,0)
                    or any(key in attributes for key in ('stopduration', 'pickupable', 'allowpickupable'))):
                raise ValueError('WVAR_CORPSE_SOURCE_DEFAULT_OVERRIDE')
        raw = read('data/items/appearances.dat')
        if hashlib.sha256(raw).hexdigest() != APPEARANCES_SHA256:
            raise ValueError('WVAR_CORPSE_APPEARANCE_HASH')
    sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
    import client_map_reader as wire
    seen = set()
    for number, kind, payload in wire.fields(raw):
        if number != 1 or kind != 2:
            continue
        fields = list(wire.fields(payload))
        item = next(value for field, wire_kind, value in fields if field == 1 and wire_kind == 0)
        if item not in OWNED:
            continue
        flags = next(value for field, wire_kind, value in fields if field == 3 and wire_kind == 2)
        selected = {field: bool(value) for field, wire_kind, value in wire.fields(flags)
                    if field in (5, 6, 14, 18, 42, 43) and wire_kind == 0}
        if {field: selected.get(field, False) for field in (5, 6, 14, 18, 42, 43)} != {
                5: item not in TERMINAL, 6: False, 14: item in PRIMARY, 18: item in (4147,4148,4149), 42: True, 43: False}:
            raise ValueError('WVAR_CORPSE_APPEARANCE_FLAGS')
        if item in seen:
            raise ValueError('WVAR_CORPSE_DUPLICATE_APPEARANCE')
        seen.add(item)
    if seen != OWNED:
        raise ValueError('WVAR_CORPSE_SOURCE_CLOSURE')


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('WVAR_CORPSE_DUPLICATE_ITEM')
    objects = [r for path in OBJECT_SHARDS for r in json.loads((root / path).read_bytes())['records']]
    pointers = {r['provenance']['item_pointer']['key']: r for r in objects
                if r.get('provenance', {}).get('item_pointer')}
    replacements = []
    for item, capacity, duration, target in CHAIN:
        key = f'oteryn:item.tibia.i{item}'
        definition, obj = indexed[key], pointers[key]
        if (obj['kind'] != {'state': 'KNOWN', 'value': 'corpse'}
                or obj['corpse']['corpse'] != {'state': 'KNOWN', 'value': True}
                or obj['corpse']['player_corpse'] != {'state': 'KNOWN', 'value': False}
                or obj['movable'] != {'state': 'KNOWN', 'value': item not in PRIMARY}
                or obj['client_projection'] != {'state': 'KNOWN', 'value': {'appearance_id': item}}
                or obj['provenance']['item_pointer'] != definition['identity']):
            raise ValueError('WVAR_CORPSE_WORLD_OBJECT_BINDING')
        replacement = {'kind': 'Item', 'identity': copy.deepcopy(definition['identity']),
                       'client_projection': definition['client_projection'],
                       'materializable': True, 'stack_class': 'NonStackable',
                       'semantics': {name: {'state': 'UNKNOWN'} for name in UNKNOWN_GROUPS}}
        semantics = replacement['semantics']
        semantics['physical'] = {'state': 'KNOWN', 'value': {
            'weight': {'state': 'KNOWN', 'value': WEIGHTS.get(item,0)},
            'movable': {'state': 'KNOWN', 'value': item not in PRIMARY},
            'pickupable': {'state': 'KNOWN', 'value': item in (4147,4148,4149)}}}
        semantics['stack'] = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': False}, 'stack_max': {'state': 'NOT_APPLICABLE'}}}
        semantics['container'] = ({'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': capacity}}}
                                  if capacity is not None else {'state': 'NOT_APPLICABLE'})
        semantics['temporal'] = {'state': 'KNOWN', 'value': {
            'consumption_mode': {'state': 'KNOWN', 'value': 'DURABLE_ABSOLUTE_DEADLINE'},
            'duration': {'state': 'KNOWN', 'value': duration},
            'stop_duration': {'state': 'KNOWN', 'value': False},
            'decay_target': ({'state': 'KNOWN', 'value': {'key': f'oteryn:item.tibia.i{target}', 'revision': 'definition-r1'}}
                             if target else {'state': 'NOT_APPLICABLE'})}}
        corrected=copy.deepcopy(definition)
        if item in (5977,4038,4039) and corrected['semantics']['container']=={'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':16}}}:
            corrected['semantics']['container']=copy.deepcopy(replacement['semantics']['container'])
        if definition.get('materializable') is True and corrected != replacement:
            raise ValueError('WVAR_CORPSE_ADMITTED_POLICY_CONFLICT')
        replacements.append((definition, replacement))
    for definition, replacement in replacements:
        definition.clear()
        definition.update(replacement)


def outputs(root=ROOT):
    shards = {path: json.loads(path.read_bytes())
              for path in (root / 'content/items/definitions').glob('items-*.json')}
    apply_admissions([r['definition'] for shard in shards.values() for r in shard['records']], root)
    return {path: (json.dumps(shard, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            for path, shard in shards.items()
            if any(r['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in OWNED}
                   for r in shard['records'])}


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--crystal-archive', type=Path)
    args = parser.parse_args()
    if args.crystal_archive:
        verify_source(args.crystal_archive)
    for path, raw in outputs().items():
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f'stale WVAR corpse Item: {path.relative_to(ROOT)}')
        else:
            path.write_bytes(raw)
    print(f'WVAR_CORPSE_ITEMS_VERIFIED items={len(OWNED)} source_bytes={args.crystal_archive is not None}')
