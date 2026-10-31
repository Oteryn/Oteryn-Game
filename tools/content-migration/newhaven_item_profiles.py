"""Carry source-qualified Newhaven operands into the existing native Item provider."""
import copy
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/newhaven-item-profiles.json'
IDS = {
    3003, 3049, 3052, 3059, 3066, 3074, 3272, 3274, 3285, 3286, 3338, 3350,
    3355, 3375, 3409, 3462, 3552, 3559, 3561, 3571, 3723, 7991, 7992, 8095,
    21348, 21350, 21351, 21400, 34017, 35562, 50166, 50257, 51119, 51760,
}
# Existing spell/ranged providers already carry these three operands. Their canonical
# stats are preserved by the producer; the overlay fills source-backed equipment facts.
EXISTING = {3350, 21351, 35562}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [r['production_definition']['production_key'] for r in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_NEWHAVEN_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('NEWHAVEN_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    present = {r['production_definition']['production_key'] for r in document['records']}
    if present & set(keys) != {f'oteryn:item.tibia.i{i}' for i in EXISTING}:
        raise ValueError('NEWHAVEN_ITEM_PROFILE_CONFLICT')
    # Validate the whole overlay before changing generated output.
    for row in records:
        physical = row['semantics']['physical']['value']
        if not row['admission']['materializable'] or any(
                physical[field] != {'state': 'KNOWN', 'value': True}
                for field in ('movable', 'pickupable')):
            raise ValueError('NEWHAVEN_ITEM_PHYSICAL_FACTS')
    old = {r['production_definition']['production_key']: r for r in document['records']}
    for row in records:
        key = row['production_definition']['production_key']
        if key in old:
            row['authoring'] = copy.deepcopy(old[key]['authoring'])
        if key == 'oteryn:item.tibia.i35562':
            # QUICKLOOT owns both quiver equipment patterns, including its Extra slot.
            row['semantics'] = copy.deepcopy(old[key]['semantics'])
    document['records'] = sorted([r for r in document['records']
                                  if r['production_definition']['production_key'] not in keys] + records,
                                 key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[profile_path] = data
    generated[manifest_path] = canonical(manifest)
