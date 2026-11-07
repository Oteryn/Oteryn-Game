#!/usr/bin/env python3
"""Tutorial corpse/food admission on existing Item and ordinary loot owners.

Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b pinned items.xml supplies
capacities, milliseconds after conversion, decay targets and explicit weights.
Crystal201189f appearances.dat sha256 below supplies container/cumulative,
unmove/take/corpse flags; raw bytes remain reference-only outside the checkout.
Zero omitted weights/non-pausing decay reuse the source defaults already proved
by primal_corpse_items.py. All unrelated semantics remain UNKNOWN.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
XML = 'imports/canary/items-xml/items.xml'
XML_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
APPEARANCES_SHA256 = '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'
ROOTS = (7763, 6017, 5970)
CORPSES = frozenset({7763, 6017, 4301, 4302, 4303, 5970, 4016, 4017, 4018})
PORTABLE = {7882: (10, False), 3577: (1300, True), 3582: (2000, True),
            10297: (90, True)}  # Deer also drops antlers (Crystal deer.lua:80).
# Exact captured appearance flags: container, movable, pickupable, corpse.
FLAGS = {7763: (True, False, False, True), 6017: (True, False, False, True),
         5970: (True, False, False, True), 4301: (True, True, True, False),
         4302: (False, True, True, False), 4303: (False, True, True, False),
         4016: (True, True, True, True), 4017: (True, True, True, True),
         4018: (True, True, True, True)}
OBJECT_SHARDS = ('objects-00500-00999.json', 'objects-02000-02499.json',
                 'objects-01500-01999.json')


def source_policies(root=ROOT):
    raw = (root / XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA256:
        raise ValueError('Tutorial source XML digest changed')
    indexed = {}
    for node in ET.fromstring(raw).findall('item'):
        if 'id' in node.attrib:
            indexed.setdefault(int(node.attrib['id']), []).append(node)
    policies = {}
    for first in ROOTS:
        item, seen = first, set()
        while item:
            if item in seen or len(seen) >= 8:
                raise ValueError('Tutorial corpse decay cycle or unbounded chain')
            seen.add(item)
            nodes = indexed.get(item, [])
            if len(nodes) != 1:
                raise ValueError(f'Tutorial source Item absent/duplicated: {item}')
            attrs = {}
            for a in nodes[0].findall('attribute'):
                key = a.attrib['key'].lower()
                if key in attrs:
                    raise ValueError(f'Duplicate tutorial corpse attribute: {item}/{key}')
                attrs[key] = a.attrib['value']
            if any(k in attrs for k in ('stopduration', 'pickupable', 'allowpickupable')):
                raise ValueError('Tutorial corpse overrides selected source defaults')
            capacity = int(attrs['containersize']) if 'containersize' in attrs else None
            duration, target = int(attrs['duration']) * 1000, int(attrs['decayto']) or None
            weight = int(attrs.get('weight', '0'))
            if (capacity is not None and not 1 <= capacity <= 65535) or not 1 <= duration <= 2**63-1 or not 0 <= weight <= 2**32-1:
                raise ValueError('Tutorial corpse quantity outside its typed field')
            if (capacity is not None) != FLAGS[item][0]:
                raise ValueError('Tutorial container XML and appearance flags disagree')
            policy = capacity, duration, target, weight
            if item in policies and policies[item] != policy:
                raise ValueError('Conflicting tutorial corpse decay paths')
            policies[item] = policy
            item = target
    if set(policies) != CORPSES:
        raise ValueError('Tutorial corpse source closure changed')
    return policies


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('Duplicate tutorial Item definition')
    owned = {f'oteryn:item.tibia.i{i}' for i in CORPSES | PORTABLE.keys()}
    if not owned.intersection(indexed):
        return
    if not owned.issubset(indexed):
        raise ValueError('Tutorial Item admission requires its complete source closure')
    policies = source_policies(root)
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    path = root / 'tools/content-schema/native-gameplay/primal_corpse_items.py'
    spec = importlib.util.spec_from_file_location('tutorial_corpse_definition', path)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    pointers = {}
    for shard in OBJECT_SHARDS:
        for obj in json.loads((root / 'content/world/objects' / shard).read_bytes())['records']:
            pointer = obj.get('provenance', {}).get('item_pointer')
            if pointer and pointer['key'] in {f'oteryn:item.tibia.i{i}' for i in CORPSES}:
                if pointer['key'] in pointers:
                    raise ValueError('Duplicate tutorial WorldObject pointer')
                pointers[pointer['key']] = obj
    replacements = []
    for item in sorted(CORPSES | PORTABLE.keys()):
        key = f'oteryn:item.tibia.i{item}'
        d = indexed[key]
        exact = [b for b in bindings if b['target'] == d['identity']]
        if len(exact) != 1 or exact[0]['disposition'] != 'EXACT' or exact[0]['external_id'] != str(item):
            raise ValueError(f'Tutorial Item lacks an exact source binding: {key}')
        if item in CORPSES:
            capacity, duration, target, weight = policies[item]
            _, movable, pickupable, corpse = FLAGS[item]
            obj = pointers.get(key)
            if item in ROOTS and obj is None:
                raise ValueError('Initial tutorial corpse lacks existing WorldObject')
            if obj is not None and (obj['provenance']['item_pointer'] != d['identity']
                                    or obj['movable'] != {'state': 'KNOWN', 'value': movable}
                                    or (corpse and obj['corpse']['corpse'] != {'state': 'KNOWN', 'value': True})):
                raise ValueError('Tutorial corpse existing WorldObject facts disagree')
            replacement = helper.corpse_definition(d, capacity, duration, target, movable)
            physical = replacement['semantics']['physical']['value']
            physical['weight']['value'] = weight
            physical['pickupable']['value'] = pickupable
        else:
            import copy
            replacement = copy.deepcopy(d)
            weight, stackable = PORTABLE[item]
            known = lambda value: {'state': 'KNOWN', 'value': value}
            stack = d['semantics']['stack']
            stack_agrees = (stack == {'state': 'UNKNOWN'}
                            or (stack.get('state') == 'KNOWN'
                                and stack['value']['stackable'] == known(stackable)))
            if (d['semantics']['physical'] != known({'weight': known(weight), 'movable': known(True), 'pickupable': known(True)})
                    or not stack_agrees):
                raise ValueError('Tutorial portable Item core facts disagree')
            replacement['materializable'] = True
            replacement['stack_class'] = 'StackCapable' if stackable else 'NonStackable'
            replacement['semantics']['stack'] = known({
                'stackable': known(stackable),
                'stack_max': known(100) if stackable else {'state': 'NOT_APPLICABLE'}})
        replacements.append((d, replacement))
    for d, replacement in replacements:
        d.clear()
        d.update(replacement)


def outputs(root=ROOT):
    shards = {p: json.loads(p.read_bytes()) for p in (root / 'content/items/definitions').glob('items-*.json')}
    apply_admissions([r['definition'] for s in shards.values() for r in s['records']], root)
    keys = {f'oteryn:item.tibia.i{i}' for i in CORPSES | PORTABLE.keys()}
    return {p: (json.dumps(s, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            for p, s in shards.items() if any(r['definition']['identity']['key'] in keys for r in s['records'])}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in outputs().items():
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit('Tutorial Item admission drift: ' + str(path.relative_to(ROOT)))
        else:
            path.write_bytes(raw)
