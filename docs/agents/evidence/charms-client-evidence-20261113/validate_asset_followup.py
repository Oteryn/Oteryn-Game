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
        print('PASS: two creator samples, projection arithmetic, six query windows and source receipts')
