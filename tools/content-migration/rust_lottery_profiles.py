"""W19E: source-derived native profiles for rust removal and lottery tickets."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles

PATH = 'tools/content-schema/native-gameplay/rust-lottery-profiles.json'
IDS = {3357, 3358, 3359, 3360, 3362, 3364, 3370, 3371, 3372, 3377,
       3381, 3382, 3557, 3558, 5957, 5958, 8063, 8894, 8895, 8896,
       8897, 8898, 8899, 9016}


def capture(canary, crystal):
    xml_raw = (canary / 'data/items/items.xml').read_bytes()
    appearance_raw = (canary / 'data/items/appearances.dat').read_bytes()
    crystal_raw = (crystal / 'data/items/items.xml').read_bytes()
    xml, other = source.xml_index(xml_raw), source.xml_index(crystal_raw)
    objects = source.engine_items.load_appearance_objects(appearance_raw)
    physical = {row['server_item_id']: row
                for row in source.policies(appearance_raw, xml_raw, IDS)}
    bindings = binding_index(BINDINGS.read_bytes())
    records = []
    for item_id in sorted(IDS):
        if source.attrs(xml[item_id]).get('weight') != source.attrs(other[item_id]).get('weight'):
            raise ValueError('RUST_LOTTERY_SOURCE_WEIGHT_DRIFT')
        records.append(source.profile(item_id, xml[item_id], objects[item_id],
                                      physical[item_id], bindings, None))
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                          bindings, IDS)
    return {'schema': 'OTERYN_NATIVE_RUST_LOTTERY_PROFILES/v1',
            'sources': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                        for path, raw in [('canary-47dfd51f/data/items/items.xml', xml_raw),
                                          ('canary-47dfd51f/data/items/appearances.dat', appearance_raw),
                                          ('crystal-201189f/data/items/items.xml', crystal_raw)]],
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = {f'oteryn:item.tibia.i{i}' for i in IDS}
    if (overlay['schema'] != 'OTERYN_NATIVE_RUST_LOTTERY_PROFILES/v1'
            or len(records) != len(keys)
            or {r['production_definition']['production_key'] for r in records} != keys):
        raise ValueError('RUST_LOTTERY_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    for record in records:
        key = record['production_definition']['production_key']
        if key in existing:
            if (existing[key]['production_binding'] != record['production_binding']
                    or existing[key]['production_definition'] != record['production_definition']):
                raise ValueError('RUST_LOTTERY_PROFILE_IDENTITY_CONFLICT')
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.canary, args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('RUST_LOTTERY_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
