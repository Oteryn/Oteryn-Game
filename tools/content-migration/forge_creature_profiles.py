"""FORGECREATURE: sliver materialization through a bounded native Item profile.

Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446 and Crystal
201189f871d23597b82caf88fd611454a2ddefb8 agree on the cumulative appearance
and the items.hpp stackSize default of 100; neither XML overrides it.
The legacy per-range definition shards remain reference-only.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles

PATH = 'tools/content-schema/native-gameplay/forge-creature-item-profiles.json'
ITEM_ID = 37109
PINS = (
    ('canary', '47dfd51f45280a59a1d3e50ba7edd573d7234446'),
    ('crystal', '201189f871d23597b82caf88fd611454a2ddefb8'),
)
SOURCE_PATHS = ('data/items/items.xml', 'data/items/appearances.dat', 'src/items/items.hpp')
STACK = source.K({'stackable': source.K(True), 'stack_max': source.K(100)})
ADMISSION = {'materializable': True, 'stack_class': 'StackCapable',
             'legal_destinations': ['CharacterInventory', 'Ground']}


def capture(canary, crystal):
    sources, profiles = [], []
    bindings = binding_index(BINDINGS.read_bytes())
    for root, (name, revision) in zip((canary, crystal), PINS):
        xml_raw, appearance_raw, header = ((root / path).read_bytes() for path in SOURCE_PATHS)
        xml = source.xml_index(xml_raw)
        objects = source.engine_items.load_appearance_objects(appearance_raw)
        if (b'uint8_t stackSize = 100;' not in header
                or 'stacksize' in source.attrs(xml[ITEM_ID])
                or not objects[ITEM_ID]['flags'].get('flags.cumulative')):
            raise ValueError('FORGE_CREATURE_STACK_SOURCE')
        physical = source.policies(appearance_raw, xml_raw, {ITEM_ID})[0]
        record = source.profile(ITEM_ID, xml[ITEM_ID], objects[ITEM_ID], physical, bindings, None)
        if record['semantics']['stack'] != STACK or record['admission'] != ADMISSION:
            raise ValueError('FORGE_CREATURE_STACK_PROFILE')
        profiles.append(record)
        sources.extend({'reference': name, 'revision': revision, 'path': path,
                        'sha256': hashlib.sha256(raw).hexdigest()}
                       for path, raw in zip(SOURCE_PATHS, (xml_raw, appearance_raw, header)))
    if profiles[0] != profiles[1]:
        raise ValueError('FORGE_CREATURE_REFERENCE_DRIFT')
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': profiles[:1]},
                          bindings, {ITEM_ID})
    return {'schema': 'OTERYN_NATIVE_FORGE_CREATURE_ITEM_PROFILES/v1',
            'producer': 'tools/content-migration/forge_creature_profiles.py',
            'sources': sources, 'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    key = f'oteryn:item.tibia.i{ITEM_ID}'
    if (overlay['schema'] != 'OTERYN_NATIVE_FORGE_CREATURE_ITEM_PROFILES/v1'
            or len(records) != 1 or records[0]['production_definition']['production_key'] != key
            or records[0]['semantics']['stack'] != STACK or records[0]['admission'] != ADMISSION):
        raise ValueError('FORGE_CREATURE_PROFILE_SET')
    document = json.loads(generated[profile_path])
    if any(r['production_definition']['production_key'] == key for r in document['records']):
        raise ValueError('FORGE_CREATURE_PROFILE_DUPLICATE')
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
            raise SystemExit('FORGE_CREATURE_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
