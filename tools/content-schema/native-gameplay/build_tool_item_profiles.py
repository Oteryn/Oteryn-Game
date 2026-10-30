#!/usr/bin/env python3
"""Normalize the supported terrain tools using the existing pinned native Item importer.

Basic fishing: Crystal 201189f8 data-global/scripts/actions/other/fishing.lua;
Global: Fishing Rod revision 1113826, Worm 1113950, Fish 1113662.
Behaviour: Crystal 201189f8 data-global/scripts/lib/register_actions.lua:6-7,363,864.
Global: Shovel revision 1113775; Machete 1147243; Jungle Grass 1113755.
Pick piles: Crystal 201189f8 register_actions.lua:633-648; items.xml decay 1002-1004, 1029-1031, 1046-1048.
Harvest remains excluded until the durable owner can commit its yield atomically.
"""
import argparse
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/native-gameplay/tool-item-profiles.json'
IDS = {593, 594, 606, 608, 609, 615, 867, 868, 3308, 3457, 3483, 3492, 3695, 3696, 3701, 3702, 4597}


def build(directory):
    xml = (directory / 'items.xml').read_bytes()
    appearances = (directory / 'appearances.dat').read_bytes()
    proof = json.loads((ROOT / 'tools/content-schema/native-gameplay/canary-ground-proof.json').read_bytes())
    hashes = {row['path']: row['sha256'] for row in proof['sources']}
    sources = [('data/items/items.xml', xml), ('data/items/appearances.dat', appearances)]
    if proof['revision'] != native.CANARY_PIN or any(
            hashlib.sha256(raw).hexdigest() != hashes[path] for path, raw in sources):
        raise ValueError('TOOL_ITEM_SOURCE_PIN')
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {row['server_item_id']: row for row in native.policies(appearances, xml, IDS)}
    records = [native.profile(item, index[item], objects[item], physical[item], bindings, None)
               for item in sorted(IDS)]
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))
    return {
        'schema': 'OTERYN_NATIVE_TOOL_ITEM_PROFILES/v1',
        'producer': 'tools/content-schema/native-gameplay/build_tool_item_profiles.py',
        'source_revision': native.CANARY_PIN,
        'source_files': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                         for path, raw in sources],
        'binding_table_sha256': hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),
        'records': bound['records'],
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != data:
            raise SystemExit('TOOL_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(data)
