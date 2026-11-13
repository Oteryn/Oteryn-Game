"""Check sample/projection arithmetic and guard against modern-rule overclaims."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

HERE = Path(__file__).resolve().parent


def load():
    return json.loads((HERE / 'asset-test-followup.json').read_text(encoding='utf-8'))


def validate_gabriel(data):
    assert data['isolated_mechanics_or_combat_parity_verified'] is False
    assert data['exact_charm_stages_verified'] is False
    assert data['race_assignments_verified'] is False
    b, d = data['blow'], data['divine']
    assert d['initial_analyser_is_prior_run'] is True
    assert d['reset_checkpoint']['raw_xp'] == d['reset_checkpoint']['damage'] == 0
    assert b['session'] == d['session'] == '00:30h'
    assert d['raw_xp'] - b['raw_xp'] == 58528
    assert d['damage'] - b['damage'] == 9299
    assert d['supplies'] - b['supplies'] == -264368
    assert sum(d['kills'].values()) - sum(b['kills'].values()) == 10
    assert round((d['raw_xp'] / b['raw_xp'] - 1) * 100, 4) == 1.8601
    assert round((d['supplies'] / b['supplies'] - 1) * 100, 4) == -61.9185


def validate_qualification(data):
    assert not data['runtime_edits_in_this_pr'] and not data['game_actions_in_this_batch']
    assert not data['scope_exhaustive'] and len(data['rows']) == 12
    candidates = [r['topic'] for r in data['rows'] if r['decision'] == 'NARROW_FIX_ELIGIBLE']
    assert candidates == ['full_reset_fee']
    assert data['rows'][1]['topic'] == 'Cleanse_Agony'
    assert data['rows'][1]['decision'] == 'CLOSED_ALREADY_CORRECT'
    examples = data['new_source_reads'][0]['examples']
    fee = lambda level: 100000 + 11000 * max(level - 100, 0)
    assert all(fee(r['level']) == r['gold'] for r in examples)
    assert fee(100) == 100000 and fee(101) == 111000
    assert fee(757) * 75 // 100 == 5495250
    assert any(r['date'] == '2026-09-08' and r['method'] == 'DIRECT_OFFICIAL_BROWSER_UI'
               for r in data['new_source_reads'])


def validate_triage(data):
    assert len(data['topics']) == 12 and not data['runtime_changes_in_evidence_branch']
    assert not data['game_actions'] and not data['all_discord_research_complete']
    assert data['implementation_pr']['draft'] is True
    assert data['implementation_pr']['merged_or_deployed'] is False
    states = [r['status'] for r in data['topics']]
    assert states.count('FIXED') == 1 and states.count('ALREADY_CORRECT') == 2
    assert states.count('HISTORICAL_ONLY') == 2 and states.count('NO_ACTION_SUPPORTED') == 2
    assert states.count('CONCRETE_OPEN') == 5


def validate(data, repository=None):
    v = data['venice']
    assert v['current_global_rule_verified'] is False
    assert v['armor_or_mana_isolated'] is False
    assert v['hour_rows_are_projection'] is True
    for row in [v['freeze'], v['parry']]:
        assert row['sample_minutes'] == 15 and row['projected_minutes'] == 60
        assert row['projected_activations'] == row['activations'] * 4
        assert row['projected_damage'] == row['damage'] * 4
    assert round((v['parry']['damage'] / v['freeze']['damage'] - 1) * 100, 4) == 78.3319
    k = data['kusnier']
    assert k['aoe_roll_or_rounding_verified'] is False
    g, d = k['grim_reapers'], k['draken_walls']
    assert round((1 - g['low_blow_supplies'] / g['elemental_supplies']) * 100, 4) == 20.7711
    assert round((1 - d['crit_supplies'] / d['elemental_supplies']) * 100, 4) == 49.6030
    assert len(data['queries']) == 6 and len(data['unresolved_after_batch']) == 7
    if repository:
        for receipt in data['source_receipts']:
            raw = subprocess.check_output(['git', 'show', data['source_head'] + ':' + receipt['path']], cwd=repository)
            assert hashlib.sha256(raw).hexdigest() == receipt['sha256']


class BoundaryTests(unittest.TestCase):
    def test_draft_is_not_deployed_fix(self):
        data = json.loads((HERE / 'current-action-triage.json').read_text(encoding='utf-8'))
        data['implementation_pr']['merged_or_deployed'] = True
        with self.assertRaises(AssertionError):
            validate_triage(data)

    def test_historical_runtime_promotion(self):
        data = json.loads((HERE / 'fix-qualification-matrix.json').read_text(encoding='utf-8'))
        data['rows'][4]['decision'] = 'NARROW_FIX_ELIGIBLE'
        with self.assertRaises(AssertionError):
            validate_qualification(data)

    def test_prior_counter_substitution(self):
        data = json.loads((HERE / 'gabriel-video-cases.json').read_text(encoding='utf-8'))
        data['divine']['raw_xp'] = data['divine']['initial_analyser']['raw_xp']
        with self.assertRaises(AssertionError):
            validate_gabriel(data)

    def test_hour_projection_substitution(self):
        data = copy.deepcopy(load())
        data['venice']['hour_rows_are_projection'] = False
        with self.assertRaises(AssertionError):
            validate(data)

    def test_modern_rule_promotion(self):
        data = copy.deepcopy(load())
        data['venice']['current_global_rule_verified'] = True
        with self.assertRaises(AssertionError):
            validate(data)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repository')
    parser.add_argument('--test', action='store_true')
    args = parser.parse_args()
    if args.test:
        unittest.main(argv=['validate_asset_followup'])
    else:
        validate(load(), args.repository)
        validate_gabriel(json.loads((HERE / 'gabriel-video-cases.json').read_text(encoding='utf-8')))
        validate_qualification(json.loads((HERE / 'fix-qualification-matrix.json').read_text(encoding='utf-8')))
        validate_triage(json.loads((HERE / 'current-action-triage.json').read_text(encoding='utf-8')))
        print('PASS: two creator samples, projection arithmetic, six query windows and source receipts')
