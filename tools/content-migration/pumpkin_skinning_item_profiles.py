"""SRV7 four Halloween skinning reward Items from pinned Crystal XML/appearance facts."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CRYSTAL = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = {8032,8178,8177,3594}
PROFILE = 'tools/content-schema/native-gameplay/pumpkin-skinning-item-profiles.json'
HASHES = {'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
          'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'}


def capture(crystal):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
    raw = {name: (crystal / name).read_bytes() for name in HASHES}
    if any(hashlib.sha256(raw[name]).hexdigest() != expected for name, expected in HASHES.items()):
        raise ValueError('PUMPKIN_SKINNING_ITEM_CRYSTAL_SOURCE_DIGEST')
    xml = source.xml_index(raw['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    bindings = binding_index(BINDINGS.read_bytes())
    physical = {row['server_item_id']: row for row in source.policies(raw['data/items/appearances.dat'], raw['data/items/items.xml'], IDS)}
    records = [source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS,
                          independent_crystal_source=(CRYSTAL, raw['data/items/items.xml']))
    # Current retained Wiki facts supersede donor worm 7*12s: current worm is 12s.
    food_raw = (ROOT / 'docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json').read_bytes()
    if hashlib.sha256(food_raw).hexdigest() != 'd13625eadd669ca491964e4a0cd1a76ec0e92999c613a55c9b8964c01687e5e6':
        raise ValueError('PUMPKIN_CURRENT_FOOD_SOURCE')
    food = json.loads(food_raw)
    for identifier, seconds in ((3594, 204), (8177, 12)):
        key = f'oteryn:item.tibia.i{identifier}'
        row = next(row for row in bound['records'] if row['production_definition']['production_key'] == key)
        facts = [fact for fact in food['rows'] if fact['target']['key'] == key]
        expected = {'family': 'Item', 'key': key, 'revision': row['production_definition']['revision_ref']}
        if len(facts) != 1 or facts[0]['target'] != expected or facts[0]['consumption'] != {'kind': 'FOOD', 'value': {'regeneration_seconds': seconds}}:
            raise ValueError('PUMPKIN_CURRENT_FOOD_IDENTITY')
        row['semantics']['consumption'] = {'state': 'KNOWN', 'value': facts[0]['consumption']}
    return {'schema': 'OTERYN_NATIVE_PUMPKIN_SKINNING_ITEM_PROFILES/v1', 'source_repository': 'zimbadev/crystalserver',
            'source_revision': CRYSTAL, 'sources': [{'path': p, 'sha256': h} for p, h in HASHES.items()],
            'source_scope': 'Exact pinned Crystal reward item physics; caller source table is skinning.lua:89-104,271-286.',
            'consumption_source': {'path': 'docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json',
                                   'sha256': hashlib.sha256(food_raw).hexdigest(),
                                   'scope': 'Current Wiki Pumpkin 204s, Yummy Gummy Worm 12s; retained source admission overrides donor worm84s.'},
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PROFILE).read_bytes())
    if overlay['schema'] != 'OTERYN_NATIVE_PUMPKIN_SKINNING_ITEM_PROFILES/v1' or overlay['source_revision'] != CRYSTAL:
        raise ValueError('PUMPKIN_SKINNING_ITEM_PROFILE_SOURCE')
    if {int(row['production_definition']['production_key'].rsplit('i', 1)[1]) for row in overlay['records']} != IDS:
        raise ValueError('PUMPKIN_SKINNING_ITEM_PROFILE_COVERAGE')
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
            raise ValueError('PUMPKIN_SKINNING_ITEM_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
