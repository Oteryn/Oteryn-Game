#!/usr/bin/env python3
"""Generate the opt-in Testserver Assistant pack; Python standard library only."""
from __future__ import annotations

import argparse
import json
import re
import unicodedata
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACK = Path('content/test-packs/test-merchant/merchant.json')
NPC = 'oteryn:npc.testserver_assistant'
REVISION = 'definition-r1'
MAX_OFFERS = 1024
# Crystal 201189f8 data-global/npc/testserver_assistant.lua:1,10,18-25.
SOURCE = 'zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/testserver_assistant.lua:1,10,18-25'
# Wheel/Gem Atelier: all five currently admitted vocations, including Monk.
GEMS = tuple(range(44602, 44614)) + (46625, 46626, 49371, 49372, 49373)


def read(root, path):
    return json.loads((root / path).read_text())


def records(root, directory):
    for path in sorted((root / directory).glob('*.json')):
        yield from json.loads(path.read_text()).get('records', [])


def item_keys(value):
    if isinstance(value, str):
        if re.fullmatch(r'oteryn:item\.[a-zA-Z0-9_.-]+', value):
            yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from item_keys(child)
    elif isinstance(value, list):
        for child in value:
            yield from item_keys(child)


def sources(root):
    loot = set()
    for row in records(root, 'content/loot'):
        definition = row.get('definition', {})
        if definition.get('identity', {}).get('key', '').startswith('oteryn:loot.creature.'):
            loot.update(item_keys(definition.get('entries', [])))
    trade = set()
    for row in records(root, 'content/services/trade'):
        trade.update(item_keys(row['declaration'].get('offers', [])))
    quest = set(item_keys(read(root, 'content/quests/chosen-reward-offers.json')))
    # The literal chosen offers describe dialogue; their native callback owns the items.
    quest.update(item_keys(read(root, 'content/quests/chosen-reward-callbacks.json')))
    packet = read(root, 'docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json')
    quest.update(row['item_key'] for row in packet['admissions'])
    return dict(loot=loot, trade=trade, quest=quest,
                wheel={f'oteryn:item.tibia.i{i}' for i in GEMS})


def reference(family, key):
    return dict(family=family, key=key, revision=REVISION)


def build(root=ROOT):
    source = sources(root)
    portable = set()
    definitions = {}
    for row in records(root, 'content/items/definitions'):
        definition = row.get('definition', {})
        if definition.get('identity', {}).get('family') != 'Item':
            continue
        definitions[definition['identity']['key']] = definition
        physical = definition.get('semantics', {}).get('physical', {})
        value = physical.get('value', {}) if physical.get('state') == 'KNOWN' else {}
        # Source membership proves obtainability. Exclude explicit physical
        # prohibitions; UNKNOWN in the reference tree is not a prohibition.
        if all(value.get(field) != dict(state='KNOWN', value=False)
               for field in ('pickupable', 'movable')):
            portable.add(definition['identity']['key'])
    keys = sorted(set.union(*source.values()) & portable)
    taxonomy = {row['target']['key']: row.get('source_taxonomy', {}).get('primary')
                for row in read(root, 'content/items/taxonomy/items.json')['records']}
    groups = defaultdict(list)
    for key in keys:
        primary = taxonomy.get(key) or 'Other Items'
        primary = unicodedata.normalize('NFKD', primary).encode('ascii', 'ignore').decode()
        keyword = re.sub(r'[^a-z0-9]+', ' ', primary.lower()).strip()
        if keyword == 'retained owner reviewed family' or keyword.startswith('client market category '):
            keyword = 'other items'
        groups[keyword].append(key)
    categories, services = [], []
    def offer(key):
        row = dict(direction='SellToPlayer', item=reference('Item', key), unit_price=1)
        semantics = definitions[key].get('semantics', {})
        charges = semantics.get('charges', {}).get('value', {}).get('count', {})
        stackable = (definitions[key].get('stack_class') == 'StackCapable'
                     or semantics.get('stack', {}).get('value', {}).get('stackable')
                     == dict(state='KNOWN', value=True))
        if key in {f'oteryn:item.tibia.i{i}' for i in (2874, 2875, 2877, 2880, 2901, 5552, 9232)}:
            row['sub_type'] = 0  # Empty native fluid container, one physical item.
        elif charges.get('state') == 'KNOWN' and not stackable:
            row['sub_type'] = charges['value']  # Existing native full-charge default.
        return row
    for keyword, items in sorted(groups.items()):
        for offset in range(0, len(items), MAX_OFFERS):
            part = keyword + (f' {offset // MAX_OFFERS + 1}' if offset else '')
            service = 'oteryn:service.trade.testserver_assistant.' + part.replace(' ', '_')
            categories.append(dict(keyword=part, service=reference('Service', service)))
            services.append(dict(kind='Service', identity=dict(key=service, revision=REVISION),
                                 fields=[], offers=[offer(key) for key in items[offset:offset + MAX_OFFERS]]))
    presentation = 'oteryn:presentation.npc.testserver_assistant'
    npc = dict(kind='NPC', identity=dict(key=NPC, revision=REVISION), fields=[],
               presentation=reference('Presentation', presentation),
               services=[category['service'] for category in categories])
    profile = dict(target=reference('Presentation', presentation),
                   data=dict(kind='Presentation', profile=dict(
                       asset_binding='canary.appearance:outfit/396', light_level=0,
                       palette_bindings=[dict(slot=slot, asset_binding='canary.appearance:palette/0')
                                         for slot in ('Head', 'Body', 'Legs', 'Feet')])))
    start = read(root, 'content/world/pins/oteryn.json')['entry_start']
    x, y, floor = start['x'] + 1, start['y'], start['floor']
    return dict(schema='OTERYN_TEST_MERCHANT/v1', source=SOURCE,
                declarations=[npc, *services], profiles=[profile], categories=categories,
                placement=dict(identity=f'oteryn:npc_placement.testserver_assistant.x{x}_y{y}_z{abs(floor)}',
                               npc=NPC, x=x, y=y, floor=floor, direction='north'),
                counts=dict(sources={name: len(items) for name, items in source.items()},
                            included_sources={name: len(items & portable) for name, items in source.items()},
                            total=len(keys)))


def encode(pack):
    return (json.dumps(pack, indent=2, ensure_ascii=False) + '\n').encode()


def inventory(pack):
    rows = ['# Generated test merchant inventory', '',
            f"Unique catalogue items: {pack['counts']['total']}", '',
            '| Source | Raw | Included |', '|---|---:|---:|']
    for name, count in pack['counts']['sources'].items():
        rows.append(f"| {name} | {count} | {pack['counts']['included_sources'][name]} |")
    rows += ['', '| Category keyword | Offers |', '|---|---:|']
    services = {row['identity']['key']: row for row in pack['declarations'][1:]}
    for category in pack['categories']:
        count = len(services[category['service']['key']]['offers'])
        rows.append(f"| {category['keyword']} | {count} |")
    return ('\n'.join(rows) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    pack = build()
    for relative, data in ((PACK, encode(pack)), (PACK.parent / 'inventory.md', inventory(pack))):
        path = ROOT / relative
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit(f'test merchant output is stale: {relative}')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print(json.dumps(pack['counts'], sort_keys=True))


if __name__ == '__main__':
    main()
