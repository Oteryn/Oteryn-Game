#!/usr/bin/env python3
"""Source-derived chest/bookcase carriers for the N--Q reward cohort.

Use the existing pinned importer unchanged. These are placed map entries, not
reward grants; their source physics must survive map boot.
"""
import argparse
import json
from pathlib import Path

from build_tool_item_profiles import build as source_profiles

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/native-gameplay/quest-chest-item-profiles.json'
IDS = {1777, 2436, 2438, 2473, 2474}


def build(directory):
    document = source_profiles(directory, IDS)
    document['schema'] = 'OTERYN_NATIVE_QUEST_CHEST_ITEM_PROFILES/v1'
    document['producer'] = 'tools/content-schema/native-gameplay/build_quest_chest_profiles.py'
    return document


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != raw:
            raise SystemExit('QUEST_CHEST_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(raw)
