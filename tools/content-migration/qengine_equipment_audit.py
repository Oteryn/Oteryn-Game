"""Refresh the QENGINE2 equipment cohort count from its pinned source producer.

All production attribute assertions remain handwritten tests. Only the derived
cohort cardinality is refreshed; the producer's explicit source IDs must exactly
match the generated proof before this tool may update its expected count.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import re


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('content', nargs='?', default='content')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path(args.content).resolve().parent
    path = root / 'tools/content-migration/qengine_reward_item_profiles.py'
    spec = importlib.util.spec_from_file_location('equipment_qengine_source', path)
    source = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(source)
    proof = json.loads((root / source.PROOF).read_bytes())
    keys = [row['production_definition']['production_key'] for row in proof['records']]
    expected = {f'oteryn:item.tibia.i{item}' for item in source.IDS}
    if len(keys) != len(set(keys)) or set(keys) != expected:
        raise SystemExit('QENGINE_EQUIPMENT_SOURCE_SET: regenerate native profiles first')
    if {row['sha256'] for row in proof['sources']} != {source.XML_SHA, source.CLIENT_SHA}:
        raise SystemExit('QENGINE_EQUIPMENT_SOURCE_PINS')
    test = root / 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
    before = test.read_text()
    after, count = re.subn(r'assert_eq!\(added_keys\.len\(\), [0-9]+\);',
                         f'assert_eq!(added_keys.len(), {len(expected)});', before)
    if count != 1:
        raise SystemExit('QENGINE_EQUIPMENT_AUDIT_ANCHOR')
    after, count = re.subn(r'QENGINE2 (?:captures 53 shared Items plus seven portable QUESTPC4 Items|adds exactly the [0-9]+ independently captured physical quest Items)\.',
                         f'QENGINE2 adds exactly the {len(expected)} independently captured physical quest Items.', after)
    if count != 1:
        raise SystemExit('QENGINE_EQUIPMENT_AUDIT_COMMENT')
    after, count = re.subn(r'(\("qengine-reward-item-profiles\.json", )[0-9]+(\),)',
                         lambda match: match[1] + str(len(expected)) + match[2], after)
    if count != 1:
        raise SystemExit('QENGINE_EQUIPMENT_COHORT_ANCHOR')
    if args.check and after != before:
        raise SystemExit('QENGINE_EQUIPMENT_COUNT_STALE: run tools/content-migration/qengine_equipment_audit.py content')
    if not args.check and after != before:
        test.write_text(after)
    print(f'QENGINE_EQUIPMENT_SOURCE_COUNT_GREEN expected={len(expected)} check={args.check}')


if __name__ == '__main__':
    main()
