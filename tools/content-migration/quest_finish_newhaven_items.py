"""Reconcile Newhaven's named rewards against exact Crystal XML and client flags.

The disabled donor callback retains stale numeric IDs. Its named rewards are
resolved by unique exact names in the same revision's XML, never by substitutes.
Only existing canonical identities are qualified; accepted facts win.
"""
import argparse
import copy
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import grizzly_shop_profiles as accepted
import quests_2_emperor_item_profiles as emperor

source = accepted.source
ROOT = accepted.ROOT
PATH = 'docs/agents/evidence/quest-finish-newhaven-items.json'
REVISION = accepted.REVISION
SCHEMA = 'OTERYN_QUEST_FINISH_NEWHAVEN_ITEMS/v1'
# Generated from capture()'s complete exact-source packet, using semantic_digest().
CAPTURE_SEMANTIC_SHA256 = '2e402f7ac7119f6a11c46837eeb19ae91ab942e77c4b1bd1e64f7c7611f74751'
IDS = (3066, 3074, 3285, 3359, 3375, 3571, 7991, 50195, 51119, 51760)
NAMES = ('snakebite rod', 'wand of vortex', 'longsword', 'brass armor',
         'soldier helmet', "ranger's cloak", "magician's robe", 'harmony amulet',
         'simple jo staff', 'refined bow')
RECONCILIATIONS = ((10321, 3571), (8871, 7991), (48847, 50195),
                   (48845, 51119), (48843, 51760))
LOOT = 'data-global/scripts/quests/newhaven/#ondroploot_the_corruptor.lua'
TARGET = 'content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'
HASHES = dict(emperor.SOURCE_HASHES)
HASHES['src/items/functions/item/item_parse.cpp'] = '08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7'
HASHES[LOOT] = '916a8de4e45ba55e3e1daa22be1910aa841fbc312db7cf795055a12ead1d1683'
encode = emperor.encode
digest = emperor.digest


def semantic_digest(value):
    return digest(json.dumps(value, sort_keys=True, ensure_ascii=False,
                             separators=(',', ':')).encode())


def definitions(root=ROOT):
    wanted = {f'oteryn:item.tibia.i{i}' for i in IDS}
    rows = [r['definition'] for p in sorted((root / 'content/items/definitions').glob('items-*.json'))
            for r in json.loads(p.read_bytes())['records'] if r['definition']['identity']['key'] in wanted]
    if len(rows) != len(IDS) or {d['identity']['key'] for d in rows} != wanted:
        raise ValueError('NEWHAVEN_CANONICAL_ITEM_SET')
    return {d['identity']['key']: d for d in rows}


def capture(crystal, root=ROOT):
    raw = {p: (crystal / p).read_bytes() for p in HASHES}
    if any(digest(raw[p]) != sha for p, sha in HASHES.items()):
        raise ValueError('NEWHAVEN_SOURCE_PIN')
    target = (root / TARGET).read_bytes()
    if digest(target) != TARGET.split('appearances-')[1].split('.')[0]:
        raise ValueError('NEWHAVEN_TARGET_PIN')
    xml = source.xml_index(raw['data/items/items.xml'])
    donor = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    objects = source.engine_items.load_appearance_objects(target)
    physical = {r['server_item_id']: r for r in source.policies(
        raw['data/items/appearances.dat'], raw['data/items/items.xml'], set(IDS))}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    canonical = definitions(root)
    records, witnesses = [], []
    for item, name in zip(IDS, NAMES):
        matches = [i for i, node in xml.items() if node.get('name') == name]
        if matches != [item] or objects[item]['name'] != name or donor[item]['name'] != name:
            raise ValueError('NEWHAVEN_EXACT_NAME_IDENTITY')
        # Compare custody flags across both pinned protobufs. No sprite bytes are retained.
        flags = ('flags.take', 'flags.unmove', 'flags.cumulative', 'flags.container', 'clothes.slot')
        if any(objects[item]['flags'].get(f) != donor[item]['flags'].get(f) for f in flags):
            raise ValueError('NEWHAVEN_TARGET_FLAGS')
        node = copy.deepcopy(xml[item])
        attrs = source.attrs(node)
        # ItemType defaults to SLOTP_HAND (items.hpp:324); XML slotType adds
        # SLOTP_TWO_HAND (item_parse.cpp:288-301). Weapon XML scripts may omit slot.
        if attrs.get('weapontype') in ('fist', 'distance', 'sword', 'wand'):
            script = node.find("attribute[@key='script']")
            if script is None:
                script = ET.SubElement(node, 'attribute', {'key': 'script'})
            if script.find("attribute[@key='slot']") is None:
                ET.SubElement(script, 'attribute', {'key': 'slot', 'value': 'hand'})
        row = source.profile(item, node, donor[item], physical[item], bindings, None)
        definition = canonical[f'oteryn:item.tibia.i{item}']
        if row['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                'legal_destinations': ['CharacterInventory', 'Ground']}:
            raise ValueError('NEWHAVEN_PORTABLE_LEAF')
        for group in ('physical', 'stack', 'container'):
            if accepted.known_conflict(definition['semantics'][group], row['semantics'][group]):
                raise ValueError('NEWHAVEN_ACCEPTED_CUSTODY_CONFLICT')
        row['semantics'] = accepted.accepted_first(definition['semantics'], row['semantics'])
        records.append(row)
        line = next(i for i, text in enumerate(raw['data/items/items.xml'].decode().splitlines(), 1)
                    if f'<item id="{item}"' in text)
        witnesses.append({'item': item, 'name': name, 'xml_line': line,
                          'target_flags': {f: objects[item]['flags'].get(f, False) for f in flags}})
    bound = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(IDS))
    return {'schema': SCHEMA, 'source_revision': REVISION,
            'sources': [{'path': p, 'sha256': h} for p, h in HASHES.items()],
            'target': {'path': TARGET, 'sha256': digest(target)},
            'identity_witnesses': witnesses,
            'qualification_rules': ['src/items/items.hpp:292-348 (decay, weight, capacity, hand and stack defaults)',
                'src/items/items.cpp:210-251 (protobuf custody and clothes flags)',
                'src/items/functions/item/item_parse.cpp:288-301 (hand/two-hand slot policy)',
                'data/items/items.xml (identity_witnesses give exact item lines)'],
            'accepted_difference': {'reason': 'Preserve exact named rewards; reconcile stale callback IDs to unique same-revision XML identities.',
                'callback': LOOT + ':43-65',
                'old_to_correct': [list(pair) for pair in RECONCILIATIONS]},
            'wiki_status': 'UNKNOWN: current API HTTP402; no current Wiki verification claimed',
            'records': bound['records']}


def validate_packet(packet, root=ROOT):
    if semantic_digest(packet) != CAPTURE_SEMANTIC_SHA256:
        raise ValueError('NEWHAVEN_CAPTURE_SEMANTIC_PIN')
    if (packet['schema'] != SCHEMA or packet['source_revision'] != REVISION
            or packet['sources'] != [{'path': p, 'sha256': h} for p, h in HASHES.items()]
            or packet['target'] != {'path': TARGET, 'sha256': TARGET.split('appearances-')[1].split('.')[0]}
            or [w['item'] for w in packet['identity_witnesses']] != list(IDS)
            or [w['name'] for w in packet['identity_witnesses']] != list(NAMES)
            or packet['accepted_difference']['old_to_correct'] != [list(p) for p in RECONCILIATIONS]):
        raise ValueError('NEWHAVEN_PROVENANCE')
    canonical = definitions(root)
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    if [r['production_definition']['production_key'] for r in packet['records']] != list(canonical_key(i) for i in IDS):
        raise ValueError('NEWHAVEN_PROFILE_SET')
    for item, row in zip(IDS, packet['records']):
        definition = canonical[canonical_key(item)]
        identity = definition['identity']
        if (row['production_binding'] != bindings[str(item)]
                or bindings[str(item)]['target'] != identity
                or row['production_definition'] != {'family': identity['family'],
                    'production_key': identity['key'], 'revision_ref': identity['revision']}):
            raise ValueError('NEWHAVEN_IDENTITY')
        if row['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                'legal_destinations': ['CharacterInventory', 'Ground']}:
            raise ValueError('NEWHAVEN_CUSTODY')
        for group in ('physical', 'stack', 'container'):
            if accepted.known_conflict(definition['semantics'][group], row['semantics'][group]):
                raise ValueError('NEWHAVEN_ACCEPTED_CONFLICT')


def canonical_key(item):
    return f'oteryn:item.tibia.i{item}'


def apply_admissions(definitions_to_update, root=ROOT):
    packet = json.loads((root / PATH).read_bytes())
    validate_packet(packet, root)
    records = {r['production_definition']['production_key']: r for r in packet['records']}
    found = set()
    updates = []
    for definition in definitions_to_update:
        key = definition['identity']['key']
        if key not in records:
            continue
        found.add(key)
        row = records[key]
        if definition['identity'] != row['production_binding']['target']:
            raise ValueError('NEWHAVEN_IDENTITY')
        if definition['stack_class'] not in ('Unknown', 'NonStackable'):
            raise ValueError('NEWHAVEN_STACK_CONFLICT')
        if any(accepted.known_conflict(definition['semantics'][group], row['semantics'][group])
               for group in ('physical', 'stack', 'container')):
            raise ValueError('NEWHAVEN_ACCEPTED_CONFLICT')
        effective = copy.deepcopy(definition)
        effective['materializable'] = True
        effective['stack_class'] = 'NonStackable'
        effective['semantics'] = accepted.accepted_first(definition['semantics'], row['semantics'])
        updates.append((definition, effective))
    if found != set(records):
        raise ValueError('NEWHAVEN_ADMISSION_INCOMPLETE')
    for definition, effective in updates:
        definition.update(effective)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    validate_packet(packet, root)
    document = json.loads(generated[profile_path])
    present = {r['production_definition']['production_key']: r for r in document['records']}
    for record in packet['records']:
        key = record['production_definition']['production_key']
        if key not in present:
            present[key] = copy.deepcopy(record)
            continue
        existing = present[key]
        if any(existing[f] != record[f] for f in ('production_definition', 'production_binding', 'admission')):
            raise ValueError('NEWHAVEN_NATIVE_CONFLICT')
        for group in ('physical', 'stack', 'container'):
            if accepted.known_conflict(existing['semantics'][group], record['semantics'][group]):
                raise ValueError('NEWHAVEN_NATIVE_CUSTODY_CONFLICT')
        existing['semantics'] = accepted.accepted_first(existing['semantics'], record['semantics'])
    document['records'] = sorted(present.values(), key=order)
    raw = encode(document)
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = digest(raw)
    generated[profile_path] = raw
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    packet = capture(args.crystal)
    validate_packet(packet)
    raw = encode(packet)
    if args.check:
        if (ROOT / PATH).read_bytes() != raw:
            raise SystemExit('NEWHAVEN_PACKET_DRIFT')
    else:
        (ROOT / PATH).write_bytes(raw)
