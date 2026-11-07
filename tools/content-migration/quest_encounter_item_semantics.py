"""Close selected encounter Item facts through the ordinary pinned Item contracts.

The profile capture is produced by build_quest_encounter_profiles.py. Existing
native policies remain authoritative; only absent native policies are appended.
Canonical identity-only records gain their own exact captured semantics. No
creature, loot probability, reward quantity or WorldObject is rewritten.
"""
import argparse
import copy
import hashlib
import json
from task_board_item_profiles import retain_known
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/quest-encounter-item-profiles.json'
IDS = {
    3277, 3336, 3355, 3987, 3991, 3992, 5960, 9689,
    3060, 3061, 3063, 3071, 3275, 3306, 3322, 3359, 3364, 3366, 3413, 3414, 3416,
    4013, 4025, 4026, 4027, 4057, 4058, 4097, 4098, 4099, 5877, 5920, 5954,
    5973, 5983, 5995, 7368, 7382, 7393, 7401, 7430, 11457, 11472, 11482, 44158,
}


def profiles(root=ROOT):
    capture = json.loads((root / PATH).read_bytes())
    rows = capture['records']
    expected_sources = {
        'data/items/items.xml': 'bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85',
        'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
        'crystal-current/data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
        'official/client-assets/15.30/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat': '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2',
    }
    if {row['path']: row['sha256'] for row in capture['source_files']} != expected_sources:
        raise ValueError('QUEST_ENCOUNTER_SOURCE_PIN')
    overrides = capture.get('source_overrides',[])
    if (len(overrides)!=1 or overrides[0].get('item')!=44158
            or overrides[0].get('xml_revision')!='201189f871d23597b82caf88fd611454a2ddefb8'
            or overrides[0].get('client_sha256')!='2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'):
        raise ValueError('QUEST_ENCOUNTER_CURRENT_SOURCE_OVERRIDE')
    keys = [r['production_definition']['production_key'] for r in rows]
    if (capture['schema'] != 'OTERYN_NATIVE_QUEST_ENCOUNTER_ITEM_PROFILES/v1'
            or capture['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}
            or any(r['production_definition']['family'] != 'Item'
                   or r['production_definition']['revision_ref'] != 'definition-r1'
                   or r['admission']['materializable'] is not True for r in rows)):
        raise ValueError('QUEST_ENCOUNTER_PROFILE_SET')
    return {r['production_definition']['production_key']: r for r in rows}


def legacy_definitions(root=ROOT):
    document = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    selected = set(profiles(root))
    result = {d['identity']['key']: d for d in document['records'] if d['identity']['key'] in selected}
    if set(result) != selected:
        raise ValueError('QUEST_ENCOUNTER_LEGACY_CLOSURE')
    return result


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('QUEST_ENCOUNTER_DUPLICATE_CANONICAL_IDENTITY')
    legacy = legacy_definitions(root)
    replacements = []
    for key, profile in profiles(root).items():
        definition = indexed[key]
        if definition['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}:
            raise ValueError('QUEST_ENCOUNTER_CANONICAL_IDENTITY')
        replacement = copy.deepcopy(legacy[key])
        semantics = {group: {'state':'UNKNOWN'} for group in profile['semantics']}
        semantics.update(copy.deepcopy(legacy[key].get('semantics', {})))
        # Canonical admission only needs these materialization facts. Other
        # groups retain their existing owner, including partial Known groups.
        for group in ('physical','stack','container','temporal'):
            if semantics[group].get('state') == 'UNKNOWN':
                semantics[group] = copy.deepcopy(profile['semantics'][group])
        # The canonical maximum has a separate qualified reward owner. Native
        # profiles already prove their runtime ceiling; do not promote this hold.
        stack = semantics.get('stack', {})
        if (stack.get('state') == 'KNOWN'
                and stack['value']['stackable'] == {'state':'KNOWN','value':True}
                and legacy[key].get('semantics',{}).get('stack',{}).get('value',{})
                    .get('stack_max',{'state':'UNKNOWN'}).get('state') == 'UNKNOWN'):
            stack['value']['stack_max'] = {'state':'UNKNOWN'}
        replacement.update(materializable=True, stack_class=profile['admission']['stack_class'],
                           semantics=semantics)
        # Recognize only this producer's exact earlier source-only projection
        # when repairing it. Arbitrary admitted facts still refuse unchanged.
        previous_projection = copy.deepcopy(legacy[key])
        previous_projection.update(materializable=True, stack_class=profile['admission']['stack_class'],
                                   semantics=copy.deepcopy(profile['semantics']))
        if definition['materializable']:
            if definition not in (replacement, previous_projection):
                raise ValueError('QUEST_ENCOUNTER_CANONICAL_CONFLICT:' + key)
        else:
            replacement['semantics'] = retain_known(replacement['semantics'], definition.get('semantics', {}))
        replacements.append((definition, replacement))
    for definition, replacement in replacements:
        definition.clear()
        definition.update(replacement)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    captured = profiles(root)
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    legacy = legacy_definitions(root)
    for key, row in captured.items():
        if key in existing:
            for group in ('physical', 'stack', 'container', 'temporal'):
                if existing[key]['semantics'][group] != row['semantics'][group]:
                    raise ValueError('QUEST_ENCOUNTER_NATIVE_CONFLICT:' + key + ':' + group)
            existing[key]['semantics'] = retain_known(legacy[key].get('semantics', {}), existing[key]['semantics'])
        else:
            row = copy.deepcopy(row)
            row['semantics'] = retain_known(legacy[key].get('semantics', {}), row['semantics'])
            document['records'].append(row)
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def outputs(root=ROOT):
    shards = {p: json.loads(p.read_bytes()) for p in (root / 'content/items/definitions').glob('items-*.json')}
    apply_admissions([r['definition'] for shard in shards.values() for r in shard['records']], root)
    keys = set(profiles(root))
    return {p: (json.dumps(s, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            for p, s in shards.items() if any(r['definition']['identity']['key'] in keys for r in s['records'])}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in outputs().items():
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit('QUEST_ENCOUNTER_CANONICAL_DRIFT:' + str(path))
        else:
            path.write_bytes(raw)
