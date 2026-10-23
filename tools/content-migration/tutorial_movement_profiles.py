"""TUTORIAL-1 movement overlay; fail closed on the exact record set and walkability."""
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/movement-item-profiles.json'
IDS = {3892, 3893, 6216, 6217, *range(6373, 6383), 6385, 19394, 37001}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_MOVEMENT_ITEM_PROFILES/v1'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('MOVEMENT_ITEM_PROFILES_RECORD_SET')
    if any(row['attributes']['blocks_movement'] is not False for row in records):
        raise ValueError('MOVEMENT_ITEM_PROFILES_NOT_WALKABLE')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('MOVEMENT_ITEM_PROFILES_RECORD_SET')
    document['records'] = sorted(document['records'] + records, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
