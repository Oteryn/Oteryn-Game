#!/usr/bin/env python3
"""Isolated existing-case mutation witnesses; never edits committed content."""
import copy
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import test_itemhold_q as item_test
import test_questpc4_treasure_reward_profiles as treasure_test
import refresh_equipment_profile_pin as cli


def result(case):
    outcome = unittest.TestResult()
    case.run(outcome)
    return {'run': outcome.testsRun, 'failures': len(outcome.failures),
            'errors': len(outcome.errors), 'skipped': len(outcome.skipped),
            'details': [message[-1800:] for _, message in outcome.failures + outcome.errors]}


def item_case():
    return item_test.ItemholdQTests('test_ornamented_shield_exact_source_charge_keeps_conflicting_witness')


def treasure_case():
    return treasure_test.TreasureRewardTests('test_check_mode_rejects_a_missing_cohort_without_writing')


def main():
    item_test.ItemholdQTests.setUpClass()
    report = {'baseline_evidence': 'reviewed_batch5_execution.json', 'original': {
        'ornamented_shield': result(item_case()), 'treasure_check': result(treasure_case())}, 'mutants': {}}
    path = ROOT / 'content/interactions/reward_claims/reward-claim-variants-00000-00099.json'
    read = Path.read_bytes
    original = json.loads(read(path))
    for label in ('live_conflict_reintroduced', 'default_150_instead_of_raw_one', 'historical_witness_removed'):
        document = copy.deepcopy(original)
        claim = next(r['definition'] for r in document['records'] if r['definition']['identity']['key'].endswith('.kv.ornamentedshield'))
        if label == 'live_conflict_reintroduced':
            claim['data_holds'] = [{'category': 'source', 'code': 'SOURCE_CHARGE_MISMATCH', 'placement_index': 0, 'reward_field': 'items', 'reward_index': 2}]
        elif label == 'default_150_instead_of_raw_one':
            claim['native_contents']['placements'][0]['contents'][2]['charges_remaining'] = 150
        else:
            claim['source_variant']['charge_dispositions'] = []
        raw = json.dumps(document).encode()
        with patch.object(Path, 'read_bytes', lambda p: raw if p == path else read(p)):
            report['mutants'][label] = result(item_case())
    with patch.object(cli.questpc4_treasure_reward_equipment_pin, 'refresh', side_effect=lambda source, packet: source):
        report['mutants']['treasure_owner_omitted'] = result(treasure_case())
    original_main = cli.main
    def writes_on_check():
        try:
            return original_main()
        except SystemExit:
            cli.TEST.write_text(cli.TEST.read_text() + '\nUNAUTHORIZED_CHECK_WRITE\n')
            raise
    with patch.object(cli, 'main', writes_on_check):
        report['mutants']['check_writes_before_refusal'] = result(treasure_case())
    report['all_original_pass'] = all(r['run'] == 1 and not r['failures'] and not r['errors'] and not r['skipped'] for r in report['original'].values())
    report['all_mutants_rejected'] = all(r['run'] == 1 and r['failures'] == 1 and not r['errors'] for r in report['mutants'].values())
    (Path(__file__).with_name('stale_expectations_after.json')).write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['all_original_pass'] and report['all_mutants_rejected'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
