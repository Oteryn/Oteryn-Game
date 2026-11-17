#!/usr/bin/env python3
"""Admit the four source Items placed along the Desert Dungeon party route.

Reuse the pinned native Item importer: source appearance flags decide collision,
pickup and movement, and XML supplies identity/weight. No map entry is removed
or treated as walkable merely to allow the rite. Raw sources stay outside Git.
"""
import argparse
import json
import re
from pathlib import Path

import build_tool_item_profiles as importer

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/native-gameplay/party-rite-item-profiles.json'
IDS = {1841, 2114, 2153, 3656}


def build(directory):
    result = importer.build(directory, IDS)
    result['schema'] = 'OTERYN_NATIVE_PARTY_RITE_ITEM_PROFILES/v1'
    result['producer'] = 'tools/content-schema/native-gameplay/build_party_rite_item_profiles.py'
    lines = {}
    for number, line in enumerate((directory / 'items.xml').read_text().splitlines(), 1):
        match = re.search(r'<item id="(\d+)"', line)
        if match and int(match[1]) in IDS:
            lines[match[1]] = f'data/items/items.xml:{number}'
    if set(map(int, lines)) != IDS:
        raise ValueError('PARTY_RITE_ITEM_SOURCE_LINES')
    result['source_lines'] = lines
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != raw:
            raise SystemExit('PARTY_RITE_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(raw)
