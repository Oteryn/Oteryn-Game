#!/usr/bin/env python3
"""Source-qualified encounter corpse Item admission, not quest qualification.

OTS_HYPOTHESIS_ONLY: imports/canary/items-xml/items.xml:12452-12465,16320-16324,
12095-12108,16191-16195,63217-63236,76397-76412;
Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b, SHA256 below. Read exact
capacity/duration/decay edges from that pinned donor, including48-slot Spite.
Movement/corpse identity uses the existing captured Game WorldObject owner.
Zero weight, nonstacking and no pickup reuse the existing Primal corpse policy;
these are conservative Game custody rules, not new donor-default claims.
Other semantic groups, fluid-source ownership and WorldObjects are unchanged.
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
XML = 'imports/canary/items-xml/items.xml'
SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
ROOTS = (5973, 5995, 33867, 44012)
OBJECT_SHARDS = ('objects-00500-00999.json', 'objects-01000-01499.json', 'objects-01500-01999.json',
                 'objects-10500-10999.json', 'objects-12000-12499.json')


def source_policies(root=ROOT, roots=ROOTS, *, explicit_weights=False):
    raw = (root / XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('Encounter corpse source XML differs from its selected pin')
    indexed = {}
    for node in ET.fromstring(raw).findall('item'):
        if 'id' in node.attrib:
            indexed.setdefault(int(node.attrib['id']), []).append(node)
    policies = {}
    for first in roots:
        item = first
        seen = set()
        while item:
            if item in seen or len(seen) >= 8:
                raise ValueError('Encounter corpse decay chain is cyclic or unbounded')
            seen.add(item)
            nodes = indexed.get(item, [])
            if len(nodes) != 1:
                raise ValueError(f'Encounter corpse source identity absent/duplicated: {item}')
            attributes = {}
            for attribute in nodes[0].findall('attribute'):
                key = attribute.attrib['key'].lower()
                if key in attributes:
                    raise ValueError(f'Encounter corpse source attribute duplicated: {item}/{key}')
                attributes[key] = attribute.attrib['value']
            if (any(key in attributes for key in ('stopduration', 'pickupable', 'allowpickupable'))
                    or ('weight' in attributes and not explicit_weights)):
                raise ValueError('Encounter corpse source overrides the selected Game policy')
            capacity = int(attributes['containersize']) if 'containersize' in attributes else None
            duration = int(attributes['duration']) * 1000
            target = int(attributes['decayto']) or None
            if (capacity is not None and not 1 <= capacity <= 65535) or not 1 <= duration <= 2**63-1:
                raise ValueError('Encounter corpse source quantity is outside its typed field')
            policy = capacity, duration, target
            if item in policies and policies[item] != policy:
                raise ValueError('Encounter corpse decay paths disagree')
            policies[item] = policy
            item = target
    return policies


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    indexed = {definition['identity']['key']: definition for definition in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('Encounter Item input contains duplicate identities')
    policies = source_policies(root)
    pointers = {}
    for shard in OBJECT_SHARDS:
        for record in json.loads((root / 'content/world/objects' / shard).read_bytes())['records']:
            pointer = record.get('provenance', {}).get('item_pointer')
            if pointer:
                if pointer['key'] in pointers:
                    raise ValueError('Encounter WorldObject Item pointer duplicated')
                pointers[pointer['key']] = record
    path = Path(__file__).with_name('primal_corpse_items.py')
    spec = importlib.util.spec_from_file_location('encounter_corpse_definition', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    replacements = []
    for item, (capacity, duration, target) in policies.items():
        key = f'oteryn:item.tibia.i{item}'
        definition, world_object = indexed[key], pointers[key]
        movement = world_object['movable']
        if (world_object['kind'] != {'state': 'KNOWN', 'value': 'corpse'}
                or world_object['corpse']['corpse'] != {'state': 'KNOWN', 'value': True}
                or world_object['corpse']['player_corpse'] != {'state': 'KNOWN', 'value': False}
                or world_object['provenance']['item_pointer'] != definition['identity']
                or movement.get('state') != 'KNOWN' or type(movement.get('value')) is not bool):
            raise ValueError('Encounter Item/WorldObject corpse binding differs: ' + key)
        replacements.append((definition, module.corpse_definition(
            definition, capacity, duration, target, movement['value'])))
    # All source and pointer proofs precede any mutation.
    for definition, replacement in replacements:
        definition.clear()
        definition.update(replacement)


def outputs(root=ROOT):
    shards = {path: json.loads(path.read_bytes())
              for path in (root / 'content/items/definitions').glob('items-*.json')}
    owned = {f'oteryn:item.tibia.i{item}' for item in source_policies(root)}
    apply_admissions([row['definition'] for shard in shards.values() for row in shard['records']], root)
    return {path: (json.dumps(shard, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            for path, shard in shards.items()
            if any(row['definition']['identity']['key'] in owned for row in shard['records'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in outputs().items():
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f'stale encounter corpse Items: {path.relative_to(ROOT)}')
        else:
            path.write_bytes(raw)


if __name__ == '__main__':
    main()
