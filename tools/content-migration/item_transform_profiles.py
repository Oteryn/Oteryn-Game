"""Bounded wooden-chair rotation profiles, derived with the existing native Item importer."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles

PATH = 'tools/content-schema/native-gameplay/item-transform-profiles.json'
IDS = (2358, 2359, 2360, 2361)
KINDS = ('ROTATE', 'WRAP', 'USE', 'EQUIP', 'DEEQUIP', 'MALE', 'FEMALE', 'DESTROY', 'DECAY', 'WRITE_ONCE')


def capture(canary, crystal):
    xml_raw = (canary / 'data/items/items.xml').read_bytes()
    appearance_raw = (canary / 'data/items/appearances.dat').read_bytes()
    other_raw = (crystal / 'data/items/items.xml').read_bytes()
    xml, other = source.xml_index(xml_raw), source.xml_index(other_raw)
    objects = source.engine_items.load_appearance_objects(appearance_raw)
    physical = {r['server_item_id']: r for r in source.policies(appearance_raw, xml_raw, set(IDS))}
    bindings = binding_index(BINDINGS.read_bytes())
    records = []
    for i, target in zip(IDS, IDS[1:] + IDS[:1]):
        for index in (xml, other):
            attrs = {a.get('key', '').lower(): a.get('value') for a in index[i]}
            if attrs.get('rotateto') != str(target):
                raise ValueError('CHAIR_ROTATION_SOURCE_DRIFT')
        record = source.profile(i, xml[i], objects[i], physical[i], bindings, None)
        record['semantics']['use_transform'] = source.K({'targets': [
            {'kind': kind, 'target': source.K({k: bindings[str(target)]['target'][k] for k in ('key', 'revision')})
             if kind == 'ROTATE' else source.U}
            for kind in KINDS]})
        records.append(record)
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, set(IDS))
    return {'schema': 'OTERYN_NATIVE_ITEM_TRANSFORM_PROFILES/v1',
            'producer': 'tools/content-migration/item_transform_profiles.py',
            'sources': [{'reference': name, 'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                        for name, path, raw in (
                            ('canary-47dfd51f', 'data/items/items.xml', xml_raw),
                            ('canary-47dfd51f', 'data/items/appearances.dat', appearance_raw),
                            ('crystal-ff7ede59', 'data/items/items.xml', other_raw))],
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [f'oteryn:item.tibia.i{i}' for i in IDS]
    if (overlay['schema'] != 'OTERYN_NATIVE_ITEM_TRANSFORM_PROFILES/v1'
            or [r['production_definition']['production_key'] for r in records] != keys):
        raise ValueError('CHAIR_ROTATION_PROFILE_SET')
    document = json.loads(generated[profile_path])
    if set(keys) & {r['production_definition']['production_key'] for r in document['records']}:
        raise ValueError('CHAIR_ROTATION_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
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
            raise SystemExit('CHAIR_ROTATION_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
