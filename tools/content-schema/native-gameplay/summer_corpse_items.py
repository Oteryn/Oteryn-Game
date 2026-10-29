#!/usr/bin/env python3
"""SUMMER corpse admission on the existing Item/WorldObject contracts.

Crystal 00ce02a57ca5a12e48f32a3476e37471167e4c3f data/items/items.xml;
capacity 30 is owner decision 20. Loot stack/pickup flags use that same
appearances.dat; the existing engine stack ceiling is 100. Timings/edges not verified against Global.
owner decision 2026-10-29: 51442/51443 admitted as plain loot items; imbuement use stays blocked.
Utility weights (250/150 hundredths of an ounce) and Etcher stacking are from
imports/tibiawiki/facts/items-stats.json. Blank scroll stacking is unsourced;
use the existing nonstackable default. Stackable utilities use the engine ceiling 100.
WorldObject appearance/collision and Item pointers remain their existing owners.
Imbuement/refinement still owns utility use behaviour; no use is admitted here.
"""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
LOOT_IDS = frozenset({54595, 54362, 54357, 54599, 35848, 54359, 54593,
                       54356, 54597, 54358, 54591, 54355, 54601, 54360})
UTILITY_IDS = frozenset({51442, 51443})

CHAINS = ((54678, 54679, 54680, 54681), (54407, 54406, 54405, 54404),
          (54399, 54398, 54397, 54396), (54415, 54414, 54413, 54412))


def apply_admissions(definitions, root=ROOT):
    template = copy.deepcopy(next(d for d in definitions
                                 if d['identity']['key'] == 'oteryn:item.tibia.i5964'))
    policies = {f'oteryn:item.tibia.i{item}': (stage, chain) for chain in CHAINS for stage, item in enumerate(chain)}
    objects = json.loads((root / 'content/world/objects/objects-12782-12876.json').read_bytes())
    pointers = {r['provenance']['item_pointer']['key']: r for r in objects['records']
                if r.get('provenance', {}).get('item_pointer')}
    for key in policies:
        world_object = pointers[key]
        if (world_object['kind'] != {'state': 'KNOWN', 'value': 'corpse'}
                or world_object['corpse']['corpse'] != {'state': 'KNOWN', 'value': True}
                or world_object['provenance']['item_pointer']['revision'] != 'definition-r1'):
            raise ValueError('Summer Item/WorldObject corpse binding mismatch: ' + key)
    found = set()
    found_loot = set()
    for definition in definitions:
        identity = definition['identity']
        item_id = int(identity['key'].rsplit('i', 1)[1])
        if item_id in LOOT_IDS | UTILITY_IDS:
            found_loot.add(item_id)
            definition['materializable'] = True
            stackable = item_id not in {35848, 51442}
            definition['stack_class'] = 'StackCapable' if stackable else 'NonStackable'
            semantics = definition['semantics']
            semantics['stack'] = {'state': 'KNOWN', 'value': {
                'stackable': {'state': 'KNOWN', 'value': stackable},
                'stack_max': {'state': 'KNOWN', 'value': 100} if stackable else {'state': 'NOT_APPLICABLE'}}}
            semantics['physical']['value']['movable'] = {'state': 'KNOWN', 'value': True}
        if identity['key'] not in policies:
            continue
        found.add(identity['key'])
        stage, chain = policies[identity['key']]
        replacement = copy.deepcopy(template)
        replacement['identity'] = identity
        replacement['semantics']['container']['value']['capacity']['value'] = 30
        temporal = replacement['semantics']['temporal']['value']
        temporal['duration']['value'] = (10_000, 300_000, 300_000, 60_000)[stage]
        temporal['decay_target'] = ({'state': 'KNOWN', 'value': {'key': f'oteryn:item.tibia.i{chain[stage+1]}', 'revision': 'definition-r1'}}
                                    if stage < 3 else {'state': 'NOT_APPLICABLE'})
        definition.clear()
        definition.update(replacement)
    if found != set(policies) or found_loot != LOOT_IDS | UTILITY_IDS:
        raise ValueError('Summer corpse Item closure incomplete')


def outputs():
    shards = {p: json.loads(p.read_bytes()) for p in (ROOT / 'content/items/definitions').glob('items-*.json')}
    apply_admissions([r['definition'] for shard in shards.values() for r in shard['records']])
    owned = LOOT_IDS | UTILITY_IDS | {item for chain in CHAINS for item in chain}
    return {path: (json.dumps(shard, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            for path, shard in shards.items()
            if any(int(r['definition']['identity']['key'].rsplit('i', 1)[1]) in owned
                   for r in shard['records'])}


if __name__ == '__main__':
    import sys
    for path, raw in outputs().items():
        if '--check' in sys.argv:
            if path.read_bytes() != raw:
                raise SystemExit(f'stale Summer corpse Items: {path.relative_to(ROOT)}')
        else:
            path.write_bytes(raw)
