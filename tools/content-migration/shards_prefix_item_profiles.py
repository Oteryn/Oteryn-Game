"""SRV7 four Shards prefix Items from pinned Crystal XML/appearance facts."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CRYSTAL = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = {53692, 53693, 53783, 54262}
PROFILE = 'tools/content-schema/native-gameplay/shards-prefix-item-profiles.json'
HASHES = {'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
          'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'}


def capture(crystal):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
    raw = {name: (crystal / name).read_bytes() for name in HASHES}
    if any(hashlib.sha256(raw[name]).hexdigest() != expected for name, expected in HASHES.items()):
        raise ValueError('SHARDS_PREFIX_ITEM_CRYSTAL_SOURCE_DIGEST')
    xml = source.xml_index(raw['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    bindings = binding_index(BINDINGS.read_bytes())
    physical = {row['server_item_id']: row for row in source.policies(raw['data/items/appearances.dat'], raw['data/items/items.xml'], IDS)}
    records = [source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS,
                          independent_crystal_source=(CRYSTAL, raw['data/items/items.xml']))
    return {'schema': 'OTERYN_NATIVE_SHARDS_PREFIX_ITEM_PROFILES/v1', 'source_repository': 'zimbadev/crystalserver',
            'source_revision': CRYSTAL, 'sources': [{'path': p, 'sha256': h} for p, h in HASHES.items()],
            'reference_difference': 'Pinned Canary47dfd51 XML/appearance data lacks all four newer IDs; only exact pinned Crystal facts are admitted.',
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PROFILE).read_bytes())
    if overlay['schema'] != 'OTERYN_NATIVE_SHARDS_PREFIX_ITEM_PROFILES/v1' or overlay['source_revision'] != CRYSTAL:
        raise ValueError('SHARDS_PREFIX_ITEM_PROFILE_SOURCE')
    if {int(row['production_definition']['production_key'].rsplit('i', 1)[1]) for row in overlay['records']} != IDS:
        raise ValueError('SHARDS_PREFIX_ITEM_PROFILE_COVERAGE')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key'] for row in document['records']}
    document['records'] = sorted(document['records'] + [row for row in overlay['records'] if row['production_definition']['production_key'] not in existing], key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PROFILE
    if args.check:
        if path.read_bytes() != raw:
            raise ValueError('SHARDS_PREFIX_ITEM_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
