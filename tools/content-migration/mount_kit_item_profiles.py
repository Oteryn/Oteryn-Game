"""SRV7 taming sources, mount kit operands and reachable addon-NPC currencies and official cosmetic acquisition items from the pinned Crystal source."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
OUTPUT = 'tools/content-schema/native-gameplay/mount-kit-item-profiles.json'
SOURCE_HASHES = {'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
                 'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
                 'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
                 'src/items/items_definitions.hpp': '0eff3929fca762fada5e31aa5ad069cf8f92713ee8c047c0578bb7ea8612c4cf'}
IDS = {3374, 5890, 5902, 5878, 5896, 5897, 12305, 12319, 12800, 12801, 21897, 21898, 21899, 21901, 21902, 21905, 21906, 22516, 37397, 37398}


def official_cosmetic_items(root):
    rules = json.loads((root / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())['rules']
    return {row['item_id'] for row in rules if row['mode'] not in {'random_outfit', 'random_mount'}}


def capture(donor):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
    paths = list(SOURCE_HASHES)
    raw = {p: (donor / p).read_bytes() for p in paths}
    taming = json.loads((ROOT / 'content/cosmetics/taming.json').read_bytes())
    ids = IDS | official_cosmetic_items(ROOT) | {int(r['item_key'].rsplit('i', 1)[1]) for r in taming}
    ids |= {int(r['required_equipment'].rsplit('i', 1)[1]) for r in taming if r.get('required_equipment')}
    if any(hashlib.sha256(raw[path]).hexdigest() != SOURCE_HASHES[path] for path in paths):
        raise ValueError('MOUNT_KIT_PINNED_SOURCE_MISMATCH')
    xml = source.xml_index(raw[paths[0]])
    bindings = binding_index(BINDINGS.read_bytes())
    objects = source.engine_items.load_appearance_objects(raw[paths[1]])
    physical = {r['server_item_id']: r for r in source.policies(raw[paths[1]], raw[paths[0]], ids)}
    records = [source.profile(i, xml.get(i), objects[i], physical[i], bindings, None) for i in sorted(ids - {44048})]
    # Crystal items.cpp:211-213 applies the appearance clothes slot; item definitions
    # :425 defines slot bit0=HEAD. This exact mask has no source script level/vocation
    # restriction. The generic source reader deliberately leaves this DAT-only case unknown.
    mask = next(r for r in records if r['authoring']['item']['key'] == 'candidate:item/31369')
    if objects[31369]['flags'].get('clothes.slot') != 1 or source.attrs(xml[31369]).get('script'):
        raise ValueError('GRYPHON_MASK_HEAD_SOURCE_CHANGED')
    mask['semantics']['equipment'] = source.K({'patterns': source.K([{
        'pattern_id': 1, 'primary_slot': source.K('HEAD'),
        'additional_reserved_slots': source.K([]), 'mutually_exclusive_groups': source.K([]),
        'vocations': source.K([]), 'level': source.K(0), 'compatibility_rule': source.NA,
    }])})
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                          bindings, ids - {44048}, independent_crystal_source=(PIN, raw[paths[0]]))
    from cosmetic_item_identity import candidate, profile
    bound['records'].append(profile(candidate(donor)))
    bound['records'].sort(key=lambda r: r['production_definition']['production_key'])
    return {'schema': 'OTERYN_NATIVE_MOUNT_KIT_ITEM_PROFILES/v1', 'source_revision': PIN,
            'sources': [{'path': p, 'sha256': hashlib.sha256(raw[p]).hexdigest()} for p in paths],
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / OUTPUT).read_bytes())
    if overlay['schema'] != 'OTERYN_NATIVE_MOUNT_KIT_ITEM_PROFILES/v1' or overlay['source_revision'] != PIN:
        raise ValueError('MOUNT_KIT_PROFILE_SOURCE')
    taming = json.loads((root / 'content/cosmetics/taming.json').read_bytes())
    expected = IDS | official_cosmetic_items(root) | {int(r['item_key'].rsplit('i', 1)[1]) for r in taming}
    expected |= {int(r['required_equipment'].rsplit('i', 1)[1]) for r in taming if r.get('required_equipment')}
    if {int(r['production_definition']['production_key'].rsplit('i', 1)[1]) for r in overlay['records']} != expected:
        raise ValueError('MOUNT_KIT_PROFILE_COVERAGE')
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key'] for r in document['records']}
    document['records'] = sorted(document['records'] + [r for r in overlay['records']
        if r['production_definition']['production_key'] not in existing], key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--donor', required=True, type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = (json.dumps(capture(args.donor), indent=2) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != result:
            raise ValueError('MOUNT_KIT_GENERATED_PROFILE_DIFFERS')
    else:
        (ROOT / OUTPUT).write_bytes(result)
