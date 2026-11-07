"""Close selected encounter Item facts through the ordinary pinned Item contracts.

The profile capture is produced by build_quest_encounter_profiles.py. Existing
native policies remain authoritative; only absent decay stages are appended.
Canonical identity-only records gain their own exact captured semantics. No
creature, loot probability, reward quantity or WorldObject is rewritten.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/quest-encounter-item-profiles.json'
IDS = {3060, 3063, 3306, 3364, 3366, 3414, 4097, 4098, 4099, 5954, 5995, 7368, 7382, 7393}


def profiles(root=ROOT):
    capture = json.loads((root / PATH).read_bytes())
    rows = capture['records']
    expected_sources = {
        'data/items/items.xml': 'bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85',
        'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    }
    if {row['path']: row['sha256'] for row in capture['source_files']} != expected_sources:
        raise ValueError('QUEST_ENCOUNTER_SOURCE_PIN')
    keys = [r['production_definition']['production_key'] for r in rows]
    if (capture['schema'] != 'OTERYN_NATIVE_QUEST_ENCOUNTER_ITEM_PROFILES/v1'
            or capture['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}
            or any(r['production_definition']['family'] != 'Item'
                   or r['production_definition']['revision_ref'] != 'definition-r1'
                   or r['admission']['materializable'] is not True for r in rows)):
        raise ValueError('QUEST_ENCOUNTER_PROFILE_SET')
    return {r['production_definition']['production_key']: r for r in rows}


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('QUEST_ENCOUNTER_DUPLICATE_CANONICAL_IDENTITY')
    replacements = []
    for key, profile in profiles(root).items():
        definition = indexed[key]
        if definition['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}:
            raise ValueError('QUEST_ENCOUNTER_CANONICAL_IDENTITY')
        replacement = copy.deepcopy(definition)
        replacement.update(materializable=True, stack_class=profile['admission']['stack_class'],
                           semantics=copy.deepcopy(profile['semantics']))
        if definition['materializable'] and definition != replacement:
            raise ValueError('QUEST_ENCOUNTER_CANONICAL_CONFLICT:' + key)
        replacements.append((definition, replacement))
    # Validate the complete closure before changing any canonical fact.
    for definition, replacement in replacements:
        definition.clear()
        definition.update(replacement)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    captured = profiles(root)
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    for key, row in captured.items():
        if key in existing:
            for group in ('physical', 'stack', 'container', 'temporal'):
                if existing[key]['semantics'][group] != row['semantics'][group]:
                    raise ValueError('QUEST_ENCOUNTER_NATIVE_CONFLICT:' + key + ':' + group)
        else:
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
