"""C19b: admit exactly the source-derived missing Newhaven starter Item profiles."""
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/newhaven-starter-item-profiles.json'
IDS = {34017, 21348, 21400, 21350, 50166}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_NEWHAVEN_STARTER_ITEM_PROFILES/v1'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('NEWHAVEN_STARTER_PROFILE_SET')
    if any(not row['admission']['materializable']
           or 'CharacterInventory' not in row['admission']['legal_destinations'] for row in records):
        raise ValueError('NEWHAVEN_STARTER_PROFILE_CUSTODY')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('NEWHAVEN_STARTER_PROFILE_DUPLICATE')
    # Reuse the base's separately pinned small Item overlay, keeping its large
    # historical provider byte-identical and retaining all eight legacy records.
    fragment = 'content/items/definitions/starting-loadout-profiles.json'
    document = json.loads(generated[fragment])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('NEWHAVEN_STARTER_PROFILE_DUPLICATE')
    combined = sorted(document['records'] + records, key=order)
    data = ('{\n  "schema": "OTERYN_NATIVE_ITEM_PROFILES/v1",\n  "records": [\n'
            + ',\n'.join('    ' + json.dumps(row, sort_keys=True, separators=(',', ':')) for row in combined)
            + '\n  ]\n}\n').encode()
    manifest = json.loads(generated[manifest_path])
    manifest['starting_loadout_item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[fragment] = data
    generated[manifest_path] = canonical(manifest)
