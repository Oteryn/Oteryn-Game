"""Admit the bounded source-derived M06 keys, locked door and level-door pair."""
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/key-door-item-profiles.json'
IDS = {1628, 1646, 1647, 2967, 2968}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('KEY_DOOR_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('KEY_DOOR_ITEM_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
