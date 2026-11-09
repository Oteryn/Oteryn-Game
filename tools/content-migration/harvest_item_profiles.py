"""M08/M09 harvest, regrowth and sculpture operands missing from the base overlays.

Crystal 201189f8: register_actions.lua scythe/crowbar rows, tools/sickle.lua,
and tools/skinning.lua marble tiers. Keep the existing tool/gathering packets intact.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as native
from build_spell_item_identities import PRE_CAKE_BINDINGS_SHA, historical_binding_bytes

PATH = 'tools/content-schema/native-gameplay/harvest-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = {953, 3134, 3651, 3652, 3653, 4609, 4848, 5462, 5463, 5464, 5465, 5466, 5470,
       9307, 9594, 9596, 9598, 10426, 10427, 10428, 10429, 17151, 17153, 21341, 21342}


def build(checkout):
    if subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip() != REVISION:
        raise ValueError('HARVEST_DONOR_REVISION')
    paths = ('data/items/items.xml', 'data/items/appearances.dat')
    if subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain', '--', *paths], text=True).strip():
        raise ValueError('HARVEST_DONOR_DIRTY')
    sources = [(path, (checkout / path).read_bytes()) for path in paths]
    return build_sources(sources)


def build_sources(sources):
    """Replay the importer from exact source bytes, including archived pinned inputs."""
    xml, appearances = [raw for _, raw in sources]
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {row['server_item_id']: row for row in native.policies(appearances, xml, IDS)}
    records = [native.profile(item, index[item], objects[item], physical[item], bindings, None)
               for item in sorted(IDS)]
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))
    return {'schema': 'OTERYN_NATIVE_HARVEST_ITEM_PROFILES/v1',
            'producer': 'tools/content-migration/harvest_item_profiles.py',
            'source_revision': REVISION,
            'source_files': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                             for path, raw in sources],
            'binding_table_sha256': hashlib.sha256(historical_binding_bytes(native.BINDINGS.read_bytes(), PRE_CAKE_BINDINGS_SHA)).hexdigest(),
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_HARVEST_ITEM_PROFILES/v1'
            or overlay['source_revision'] != REVISION
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}
            or overlay['binding_table_sha256'] != hashlib.sha256(
                historical_binding_bytes((root / 'imports/crystalserver/bindings/items.json').read_bytes(),
                                         overlay['binding_table_sha256'])).hexdigest()):
        raise ValueError('HARVEST_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('HARVEST_ITEM_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('checkout', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(build(args.checkout), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / PATH).read_bytes() != data:
            raise SystemExit('HARVEST_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / PATH).write_bytes(data)
