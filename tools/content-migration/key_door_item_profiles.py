"""Admit source-derived M06/M01c/Panpipe keys, locked doors and level-door pairs."""
import hashlib
import json

from native_profile_composition import encode_native_profile

PATH = 'tools/content-schema/native-gameplay/key-door-item-profiles.json'
IDS = {1628, 1631, 1646, 1647, 1648, 1649, 2967, 2968, 5097, 5102, 5103, 6248, 6264, 6265}


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
    data = encode_native_profile(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
