#!/usr/bin/env python3
"""Capture missing Demon encounter Items with the existing pinned native importer."""
import argparse
import json
from pathlib import Path
from build_tool_item_profiles import build as source_profiles
ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'tools/content-schema/native-gameplay/quest-encounter-item-profiles.json'
IDS = {3060, 3063, 3306, 3364, 3366, 3414, 4097, 4098, 4099, 5954, 5995, 7368, 7382, 7393}

def build(directory):
    document = source_profiles(directory, IDS)
    document['schema'] = 'OTERYN_NATIVE_QUEST_ENCOUNTER_ITEM_PROFILES/v1'
    document['producer'] = 'tools/content-schema/native-gameplay/build_quest_encounter_profiles.py'
    return document

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if OUTPUT.read_text() != raw:
            raise SystemExit('QUEST_ENCOUNTER_PROFILE_DRIFT')
    else:
        OUTPUT.write_text(raw)
