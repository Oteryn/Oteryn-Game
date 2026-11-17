"""Add only the pinned native Item closure needed by source party-rite geometry."""
import hashlib
import json

from native_profile_composition import encode_native_profile

PATH = 'tools/content-schema/native-gameplay/party-rite-item-profiles.json'
IDS = {1841, 2114, 2153, 3656}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_PARTY_RITE_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('PARTY_RITE_ITEM_PROFILE_SET')
    proof = json.loads((root / 'tools/content-schema/native-gameplay/canary-ground-proof.json').read_bytes())
    expected = {row['path']: row['sha256'] for row in proof['sources']}
    required = {'data/items/items.xml', 'data/items/appearances.dat'}
    if (len(overlay['source_files']) != 2
            or {row['path'] for row in overlay['source_files']} != required
            or any(expected.get(row['path']) != row['sha256'] for row in overlay['source_files'])
            or set(map(int, overlay['source_lines'])) != IDS):
        raise ValueError('PARTY_RITE_ITEM_SOURCE_PIN')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('PARTY_RITE_ITEM_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
