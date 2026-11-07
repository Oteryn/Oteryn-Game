#!/usr/bin/env python3
"""TUTORIAL-1: derive exactly the 17 path Item profiles using the existing pinned importer.

Only --capture reads references. No original source/appearance files are redistributed.
"""
import argparse
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, CANARY_PIN, binding_index, bind_profiles
from lower_item_consumption_packet import fetch

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name('movement-item-profiles.json')
IDS = {3892, 3893, 6216, 6217, *range(6373, 6383), 6385, 19394, 37001}


def capture():
    proof = json.loads((ROOT / 'docs/reference/spells/r21-local-candidate/active-artifact/item-source-proof.json').read_bytes())
    captured = {}
    sources = []
    for row in proof['loader_sources']:
        if row['path'] not in ('data/items/appearances.dat', 'data/items/items.xml'):
            continue
        raw = fetch(f'https://raw.githubusercontent.com/opentibiabr/canary/{CANARY_PIN}/{row["path"]}')
        if hashlib.sha256(raw).hexdigest() != row['sha256']:
            raise ValueError('pinned source digest mismatch')
        captured[row['path']] = raw
        sources.append(row)
    xml = source.xml_index(captured['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(captured['data/items/appearances.dat'])
    physical = {row['server_item_id']: row for row in source.policies(captured['data/items/appearances.dat'], captured['data/items/items.xml'], IDS)}
    bindings = binding_index(BINDINGS.read_bytes())
    records = [source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    for row in records:
        if row['attributes']['blocks_movement'] is not False:
            raise ValueError('MOVEMENT_ITEM_PROFILES_NOT_WALKABLE')
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS)
    return {'schema': 'OTERYN_NATIVE_MOVEMENT_ITEM_PROFILES/v1',
            'producer': 'tools/content-schema/native-gameplay/build_tutorial_movement_profiles.py',
            'sources': sources, 'records': bound['records']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', action='store_true', required=True)
    args = parser.parse_args()
    OUTPUT.write_text(json.dumps(capture(), indent=2, ensure_ascii=False) + '\n')
