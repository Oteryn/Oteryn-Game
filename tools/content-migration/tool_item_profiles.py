"""Admit the supported terrain-tool family without activating unqualified harvests."""
import hashlib
import json

from native_profile_composition import encode_native_profile

PATH = 'tools/content-schema/native-gameplay/tool-item-profiles.json'
IDS = {593, 594, 606, 608, 609, 615, 867, 868, 1791, 3308, 3457, 3483, 3492, 3695, 3696, 3701, 3702, 4597}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_TOOL_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('TOOL_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    additions = []
    for record in records:
        key = record['production_definition']['production_key']
        if key in existing:
            from questpc2_ladder_composition import stationary_tool
            if stationary_tool(existing[key], record) is None:
                raise ValueError('TOOL_ITEM_PROFILE_DUPLICATE')
        else:
            additions.append(record)
    document['records'] = sorted(document['records'] + additions, key=order)
    data = encode_native_profile(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
