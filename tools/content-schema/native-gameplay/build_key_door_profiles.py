#!/usr/bin/env python3
"""Source-derived profiles for the M06 key-door and level-door playable path.

Crystal 201189f data/libs/tables/doors.lua:4,9,265 supplies the identities.
The existing pinned importer supplies physics and classifications without overrides.
"""
import argparse
import json
from pathlib import Path

from build_tool_item_profiles import build as source_profiles

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/native-gameplay/key-door-item-profiles.json'
IDS = {1628, 1646, 1647, 2967, 2968}


def build(directory):
    document = source_profiles(directory, IDS)
    document['schema'] = 'OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1'
    document['producer'] = 'tools/content-schema/native-gameplay/build_key_door_profiles.py'
    return document


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != raw:
            raise SystemExit('KEY_DOOR_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(raw)
