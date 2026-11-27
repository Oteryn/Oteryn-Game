"""Qualify the third slice of pinned creature loot through existing Item custody."""

import argparse
import json
from pathlib import Path
import subprocess

import boss_corpse_profiles as crystal
from build_full_spell_artifact import native_corpse_capacity, native_loot_yield_facts
from grizzly_shop_profiles import accepted_first
from import_cyclopedia_global import load_weights

ROOT = crystal.ROOT
BASE = '3e09916423181c795c829eca5e5414dde3f29a3e'
PREFIX = 'tools/content-schema/native-gameplay/'
CAPTURE = PREFIX + 'cloot3-item-profiles.json'
CORPSE_CAPTURE = PREFIX + 'zamulosh-corpse-item-profile.json'
BINDINGS = PREFIX + 'cloot3-item-bindings.json'
COHORT = PREFIX + 'cloot3-creatures.txt'
LOOT = PREFIX + 'loot-tables.json'
ITEMS = 'content/items/definitions/spell-native-profiles.json'
CRYSTAL_BINDINGS = 'imports/crystalserver/bindings/items.json'
ZAMULOSH_WALL = 22555
ZAMULOSH_CORPSE = 22495
# The three selected tables' donor Lua and Wiki list no i51302 yield. The client
# appearance is a fixed unclassified WorldObject (qualified-world-wiki46-fixed3).
INVALID_WORLD_REF = 51302
ICE_CUBE_VISUAL = 7741
PORTABLE_ICE_CUBE = 7441
# The Wiki names the unenchanted forms as the three-hour discharge targets.
AMULET_DISCHARGE = {
    53197: 54510, 53199: 54511, 53201: 54512, 53203: 54513, 53205: 54514,
}
# Exact Lua/DAT Items absent from the older Crystal binding inventory. Spirited
# Soil and each amulet have matching 15.30 Cyclopedia and Wiki own-item IDs.
NEW_BINDINGS = {51276, *AMULET_DISCHARGE, *AMULET_DISCHARGE.values()}


def item_ref(identifier):
    return {'family': 'Item', 'key': f'oteryn:item.tibia.i{identifier}', 'revision': 'definition-r1'}


def entries(rows):
    for row in rows:
        yield row
        yield from entries(row.get('children', ()))


def creatures(root):
    return set((root / COHORT).read_text().splitlines())


def has_native_custody(reference, row):
    if reference == item_ref(ZAMULOSH_CORPSE):
        return (row['admission'] == crystal.corpse_admission(ZAMULOSH_CORPSE)
                and native_corpse_capacity(reference, row, reward_boss=True) is not None)
    return native_loot_yield_facts(reference, row) is not None


def encode_carried_records(records):
    return (b'{"source_revision":"' + crystal.PIN.encode() + b'","records":[\n'
            + b',\n'.join(crystal.encode(row).rstrip(b'\n') for row in records) + b'\n]}\n')


def capture(root, source_cache):
    def base_document(path):
        return json.loads(subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=root))

    loot = base_document(LOOT)
    facts = {row['item']['key']: row for row in loot['items']}
    tables = {row['identity']['key']: row['entries'] for row in loot['tables']}
    gaps = {}
    for row in loot['creature_loot']:
        if row['loot'] is None:
            continue
        missing = {entry['item']['key'] for entry in entries(tables[row['loot']['key']])
                   if not facts[entry['item']['key']]['materializable']}
        if missing:
            gaps[row['creature']['key']] = missing
    if len(gaps) != 967:
        raise ValueError('CLOOT3_BASE_SCOPE')
    cohort = sorted(gaps)[646:]
    existing = {record['production_definition']['production_key']: record
                for path in (ITEMS, 'content/items/definitions/srx1-item-updates.json')
                for record in base_document(path)['records']}
    keys = set().union(*(gaps[creature] for creature in cohort))
    pending = {int(key.rsplit('.i', 1)[1]) for key in keys
               if native_loot_yield_facts(facts[key]['item'], existing.get(key, {})) is None}
    pending -= {ZAMULOSH_WALL, INVALID_WORLD_REF, ICE_CUBE_VISUAL}
    pending.update(AMULET_DISCHARGE.values())
    pending.add(ZAMULOSH_CORPSE)
    source = crystal.source
    blobs = crystal.load_source_blobs(source_cache)
    xml_raw = blobs['data/items/items.xml']
    dat = blobs['data/items/appearances.dat']
    xml = source.xml_index(xml_raw)
    appearances = source.engine_items.load_appearance_objects(dat)
    policies = {row['server_item_id']: row for row in source.policies(dat, xml_raw, pending)}
    bindings = crystal.binding_index((root / CRYSTAL_BINDINGS).read_bytes())
    extra = [{
        'source_key': 'oteryn:source.crystalserver', 'source_revision': crystal.PIN,
        'identity_namespace': 'ots/item_server_id', 'external_id': str(identifier),
        'target': item_ref(identifier), 'disposition': 'EXACT',
    } for identifier in sorted(NEW_BINDINGS)]
    for binding in extra:
        external = binding['external_id']
        if external in bindings:
            raise ValueError(f'CLOOT3_BINDING_COLLISION:{external}')
        bindings[external] = binding
    weights = load_weights(root)
    records = []
    corpse = None
    for identifier in sorted(pending):
        node = xml.get(identifier)
        if node is None and identifier not in NEW_BINDINGS:
            raise ValueError(f'CLOOT3_XML_MEMBERSHIP:{identifier}')
        policy = policies[identifier]
        appearance = appearances[identifier]
        row = source.profile(identifier, node, appearance, policy, bindings, None)
        # Custody needs the Item name and physical facts, not flavor text.
        row['semantics']['presentation']['value']['description'] = source.U
        crystal.complete_source_top_order(row, appearance)
        binding = bindings[str(identifier)]
        reference = item_ref(identifier)
        if binding['target'] != reference:
            raise ValueError(f'CLOOT3_EXACT_BINDING:{identifier}')
        row['authoring']['item']['revision'] = 'cloot3-crystal-201189f'
        row['production_binding'] = binding
        row['production_definition'] = dict(family='Item', production_key=reference['key'],
                                            revision_ref=reference['revision'])
        if node is None:
            row['semantics']['physical']['value']['weight'] = source.K(weights[identifier])
        if identifier in AMULET_DISCHARGE:
            # Retained Cyclopedia and TibiaWiki specify a three-hour equip budget.
            target = item_ref(AMULET_DISCHARGE[identifier])
            row['semantics']['temporal'] = source.K({
                'consumption_mode': source.K('AUTHORITATIVE_ACTIVE_TIME_BUDGET'),
                'duration': source.K(10_800_000),
                'stop_duration': source.K(True),
                'decay_target': source.K({key: target[key] for key in ('key', 'revision')}),
            })
        if not has_native_custody(reference, row):
            raise ValueError(f'CLOOT3_DONOR_CUSTODY:{identifier}')
        if identifier == ZAMULOSH_CORPSE:
            corpse = row
        else:
            records.append(row)
    if corpse is None:
        raise ValueError('ZAMULOSH_CORPSE_MISSING')
    return {
        COHORT: ''.join(creature + '\n' for creature in cohort).encode(),
        CAPTURE: encode_carried_records(records),
        CORPSE_CAPTURE: crystal.encode({'source_revision': crystal.PIN, 'record': corpse}),
        BINDINGS: json.dumps(extra, indent=2).encode() + b'\n',
    }


def complete_records(root, records, order):
    packet = json.loads((root / CAPTURE).read_bytes())
    corpse = json.loads((root / CORPSE_CAPTURE).read_bytes())
    if packet['source_revision'] != crystal.PIN or corpse['source_revision'] != crystal.PIN:
        raise ValueError('CLOOT3_SOURCE_REVISION')
    corpse_ref = item_ref(ZAMULOSH_CORPSE)
    if (corpse['record']['production_binding']['target'] != corpse_ref
            or any(row['production_binding']['target'] == corpse_ref for row in packet['records'])
            or not has_native_custody(corpse_ref, corpse['record'])):
        raise ValueError('ZAMULOSH_CORPSE_CUSTODY')
    accepted = crystal.binding_index((root / CRYSTAL_BINDINGS).read_bytes())
    accepted.update((row['external_id'], row) for row in json.loads((root / BINDINGS).read_bytes()))
    current = {row['production_definition']['production_key']: row for row in records}
    qualified = set()
    for observed in [*packet['records'], corpse['record']]:
        binding = observed['production_binding']
        reference = binding['target']
        key = reference['key']
        if accepted.get(binding['external_id']) != binding:
            raise ValueError(f'CLOOT3_EXACT_BINDING:{key}')
        row = current.get(key)
        if row is None:
            current[key] = observed
        elif not has_native_custody(reference, row):
            if row['production_definition'] != observed['production_definition'] or row['production_binding'] != binding:
                raise ValueError(f'CLOOT3_NATIVE_IDENTITY:{key}')
            # Keep known use, text, equipment and custody facts from other owners.
            for group in ('classification', 'physical', 'stack', 'container', 'charges'):
                row['semantics'][group] = accepted_first(row['semantics'][group], observed['semantics'][group])
            row['admission'] = observed['admission']
            if not has_native_custody(reference, row):
                raise ValueError(f'CLOOT3_NATIVE_CUSTODY:{key}')
        qualified.add(key)
    records[:] = sorted(current.values(), key=order)
    return qualified


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-cache', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in capture(ROOT, args.source_cache).items():
        if args.check:
            if (ROOT / path).read_bytes() != raw:
                raise SystemExit(f'CLOOT3_CAPTURE_DRIFT:{path}')
        else:
            (ROOT / path).write_bytes(raw)
