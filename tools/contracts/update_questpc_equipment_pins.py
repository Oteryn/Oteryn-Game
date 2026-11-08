#!/usr/bin/env python3
"""Refresh finite questpc equipment-count pins from accepted source cohorts."""
import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
INPUT = ROOT / 'tools/content-schema/native-gameplay/questpc-equipment-profile-cohorts.json'
TARGET = ROOT / 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
BEGIN = '    // BEGIN generated questpc native profile cohorts.\n'
END = '    // END generated questpc native profile cohorts.\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    cohorts = json.loads(INPUT.read_text())['cohorts']
    native = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_text())
    profiles = {r['production_definition']['production_key']: r for r in native['records']}
    seen = set()
    rows = []
    for cohort in cohorts:
        keys = cohort['keys']
        if not keys or len(set(keys)) != len(keys) or seen.intersection(keys):
            raise SystemExit('duplicate or empty questpc source cohort')
        if not (ROOT / cohort['producer']).is_file():
            raise SystemExit('missing source producer')
        for key in keys:
            row = profiles.get(key)
            if row is None or not row['admission']['materializable'] or row['attributes']['speed_bonus'] != 0:
                raise SystemExit(f'unqualified questpc source profile: {key}')
        seen.update(keys)
        rows.append(f'    ("{cohort["cohort"]}", {len(keys)}),\n')
    text = TARGET.read_text()
    block = BEGIN + ''.join(rows) + END
    if BEGIN in text:
        updated, count = re.subn(re.escape(BEGIN) + '.*?' + re.escape(END), lambda _: block, text, flags=re.S)
        if count != 1:
            raise SystemExit('ambiguous generated equipment pin block')
    else:
        anchor = '    // Batch 3 admits only the canonical demon helmet'
        if text.count(anchor) != 1:
            raise SystemExit('missing equipment pin insertion anchor')
        updated = text.replace(anchor, block + anchor)
    if args.check and updated != text:
        raise SystemExit('questpc equipment count pins are stale')
    if not args.check and updated != text:
        TARGET.write_text(updated)
    print(f'PASS questpc equipment pins: {len(cohorts)} source cohorts, {len(seen)} unique zero-speed profiles')


if __name__ == '__main__':
    main()
