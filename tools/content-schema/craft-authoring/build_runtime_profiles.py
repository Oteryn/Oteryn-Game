#!/usr/bin/env python3
"""Extract basic cooking policies using the existing pinned native Item importer.

Only normalized Item facts are retained, never original XML/DAT or client assets.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as native


def build(directory):
    xml = (directory / 'items.xml').read_bytes()
    appearances = (directory / 'appearances.dat').read_bytes()
    proof = json.loads((ROOT / 'tools/content-schema/native-gameplay/canary-ground-proof.json').read_bytes())
    hashes = {row['path']: row['sha256'] for row in proof['sources']}
    if (proof['revision'] != native.CANARY_PIN
            or hashlib.sha256(xml).hexdigest() != hashes['data/items/items.xml']
            or hashlib.sha256(appearances).hexdigest() != hashes['data/items/appearances.dat']):
        raise ValueError('COOKING_SOURCE_PIN')
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    rows = json.loads((ROOT / 'content/craft/recipes.json').read_bytes())['records']
    wanted = set()
    for row in rows:
        recipe = row['recipe']
        if (recipe['tool'] != 'none' or recipe['target']['kind'] != 'map_item'
                or len(recipe['operands']) != 1 or recipe['outputs']['kind'] != 'fixed'):
            continue
        references = [recipe['input_item'], recipe['target']['definition'],
                      recipe['operands'][0]['output_item_id']]
        references += [output['item_id'] for output in recipe['outputs']['entries']]
        wanted.update(int(ref['key'].rsplit('.i', 1)[1]) for ref in references)
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {row['server_item_id']: row for row in native.policies(appearances, xml, wanted)}
    records = [native.profile(item, index[item], objects[item], physical[item], bindings, None)
               for item in sorted(wanted)]
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))
    return {
        'schema': 'OTERYN_NATIVE_COOKING_ITEM_PROFILES/v1',
        'producer': 'tools/content-schema/craft-authoring/build_runtime_profiles.py',
        'source_revision': native.CANARY_PIN,
        'source_files': [{'path': name, 'sha256': hashlib.sha256(raw).hexdigest()}
                         for name, raw in [('data/items/items.xml', xml),
                                           ('data/items/appearances.dat', appearances)]],
        'binding_table_sha256': hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),
        'records': bound['records'],
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    args = parser.parse_args()
    output = ROOT / 'content/craft/runtime-item-profiles.json'
    output.write_text(json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n')
