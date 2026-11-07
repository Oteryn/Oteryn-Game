"""Admit the bounded source-derived M06 keys, locked door and level-door pair."""
import hashlib
import json
import copy

PATH = 'tools/content-schema/native-gameplay/key-door-item-profiles.json'
IDS = {1628, 1646, 1647, 2967, 2968, 2970, 5097, 5102, 5103}


def refine_unknown(old, source):
    if isinstance(old, dict) and isinstance(source, dict):
        if old.get('state') == 'UNKNOWN':
            return copy.deepcopy(source)
        if source.get('state') == 'UNKNOWN':
            return copy.deepcopy(old)
        result = copy.deepcopy(old)
        for key, value in source.items():
            result[key] = refine_unknown(old[key], value) if key in old else copy.deepcopy(value)
        return result
    if old != source:
        raise ValueError('KEY_DOOR_ITEM_PROFILE_KNOWN_CONFLICT')
    return copy.deepcopy(old)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('KEY_DOOR_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    additions = []
    for row in records:
        key = row['production_definition']['production_key']
        if key not in existing:
            additions.append(row)
            continue
        old = existing[key]
        # A partial ordinary-item profile may acquire source-derived key/door
        # facts. Known facts and its admitted identity remain authoritative.
        if (old['semantics']['classification']['state'] != 'UNKNOWN'
                or row['semantics']['classification']['state'] != 'KNOWN'):
            raise ValueError('KEY_DOOR_ITEM_PROFILE_DUPLICATE')
        if old['production_definition'] != row['production_definition']:
            raise ValueError('KEY_DOOR_ITEM_PROFILE_KNOWN_CONFLICT')
        refined = copy.deepcopy(old)
        refined['semantics'] = refine_unknown(old['semantics'], row['semantics'])
        refined['attributes'] = refine_unknown(old['attributes'], row['attributes'])
        if old['admission'] != row['admission']:
            raise ValueError('KEY_DOOR_ITEM_PROFILE_KNOWN_CONFLICT')
        existing[key] = refined
    document['records'] = sorted(list(existing.values()) + additions, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
