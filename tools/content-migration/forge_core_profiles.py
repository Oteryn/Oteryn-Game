"""P18: admit Exalted Core through the existing source Item importer and identity.

Crystal 201189f items.xml:69440-69443 supplies weight 260; the pinned
appearance is cumulative and pickupable, and items.hpp:348 defaults stacks
to 100 with no XML override. Optional Canary comparison requires its exact
47dfd51 files, never an older predecessor merely carrying the same item id.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, bind_profiles, binding_index

PATH = 'tools/content-schema/native-gameplay/forge-core-item-profiles.json'
ITEM_ID = 37110
SCHEMA = 'OTERYN_NATIVE_FORGE_CORE_ITEM_PROFILES/v1'
SOURCE_PATHS = ('data/items/items.xml', 'data/items/appearances.dat', 'src/items/items.hpp')
SOURCES = {
    'crystal': ('201189f871d23597b82caf88fd611454a2ddefb8', (
        '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
        '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
        '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
    )),
    'canary': ('47dfd51f45280a59a1d3e50ba7edd573d7234446', (
        '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2',
        'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50',
        'b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6',
    )),
}
STACK = source.K({'stackable': source.K(True), 'stack_max': source.K(100)})
PHYSICAL = source.K({'weight': source.K(260), 'movable': source.K(True), 'pickupable': source.K(True)})
ADMISSION = {'materializable': True, 'stack_class': 'StackCapable',
             'legal_destinations': ['CharacterInventory', 'Ground']}


def provenance(name):
    revision, hashes = SOURCES[name]
    return [{'reference': name, 'revision': revision, 'path': path, 'sha256': digest}
            for path, digest in zip(SOURCE_PATHS, hashes)]


def capture(canary, crystal):
    """Capture the primary Crystal profile; compare Canary only when supplied."""
    if crystal is None:
        raise ValueError('FORGE_CORE_PRIMARY_SOURCE_REQUIRED')
    bindings = binding_index(BINDINGS.read_bytes())
    sources, records = [], []
    for name, root in [('crystal', crystal), ('canary', canary)]:
        if root is None:
            continue
        evidence = provenance(name)
        raw = [(root / path).read_bytes() for path in SOURCE_PATHS]
        if any(hashlib.sha256(data).hexdigest() != row['sha256'] for data, row in zip(raw, evidence)):
            raise ValueError('FORGE_CORE_SOURCE_DIGEST')
        xml_raw, appearance_raw, header = raw
        xml = source.xml_index(xml_raw)
        objects = source.engine_items.load_appearance_objects(appearance_raw)
        if (b'uint8_t stackSize = 100;' not in header
                or 'stacksize' in source.attrs(xml[ITEM_ID])
                or not objects[ITEM_ID]['flags'].get('flags.cumulative')):
            raise ValueError('FORGE_CORE_STACK_SOURCE')
        physical = source.policies(appearance_raw, xml_raw, {ITEM_ID})[0]
        record = source.profile(ITEM_ID, xml[ITEM_ID], objects[ITEM_ID], physical, bindings, None)
        if (record['semantics']['stack'] != STACK or record['semantics']['physical'] != PHYSICAL
                or record['admission'] != ADMISSION):
            raise ValueError('FORGE_CORE_SOURCE_PROFILE')
        records.append(record)
        sources.extend(evidence)
    if any(record != records[0] for record in records[1:]):
        raise ValueError('FORGE_CORE_REFERENCE_DRIFT')
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records[:1]},
                          bindings, {ITEM_ID})
    return {'schema': SCHEMA, 'producer': 'tools/content-migration/forge_core_profiles.py',
            'sources': sources, 'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    key = f'oteryn:item.tibia.i{ITEM_ID}'
    primary = provenance('crystal')
    if (overlay['schema'] != SCHEMA or overlay['sources'] not in [primary, primary + provenance('canary')]
            or len(records) != 1 or records[0]['production_definition']['production_key'] != key
            or records[0]['semantics']['stack'] != STACK or records[0]['semantics']['physical'] != PHYSICAL
            or records[0]['admission'] != ADMISSION):
        raise ValueError('FORGE_CORE_PROFILE_SET')
    # The existing exact identity binding remains the admission authority.
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                          binding_index(BINDINGS.read_bytes()), {ITEM_ID})
    document = json.loads(generated[profile_path])
    if any(r['production_definition']['production_key'] == key for r in document['records']):
        raise ValueError('FORGE_CORE_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + bound['records'], key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--canary', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.canary, args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('FORGE_CORE_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
