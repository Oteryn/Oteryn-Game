"""Keep QREADY3's existing dispatch qualification visible in the inventory."""
import json
from pathlib import Path
import unittest

import quest_reward_use_proofs as proofs
from quest_completion_matrix import load_reward_claims

ROOT = Path(__file__).resolve().parents[3]


class Qready3CoverageTest(unittest.TestCase):
    def test_six_reward_claims_have_current_dispatch_proofs_and_coverage(self):
        keys = {'oteryn:reward-claim.kv.' + suffix for suffix in (
            'sixrubies', 'spikesword', 'honeyflower', 'bananafree',
            'bananapremium', 'throwingstar')}
        qualified = proofs.load(ROOT, load_reward_claims(ROOT))
        self.assertTrue(keys <= qualified)
        tests = {case['claim_key']: case['test']
                 for path in (ROOT / proofs.PROOFS).glob('*.json')
                 for case in json.loads(path.read_bytes())['cases']}
        coverage = json.loads((ROOT / 'docs/repository/QUEST_CALLBACK_COVERAGE.json').read_bytes())
        rows = {row['claim']: row for quest in coverage['quests'] for row in quest['chests']
                if row['claim'] in keys}
        self.assertEqual(keys, rows.keys())
        for key, row in rows.items():
            with self.subTest(claim=key):
                self.assertEqual('dispatch_pg_qualified', row['status'])
                self.assertEqual(tests[key], row['reason'])


if __name__ == '__main__':
    unittest.main()
