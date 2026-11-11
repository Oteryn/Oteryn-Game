"""M35 can retain reward evidence only for its exact completion-owner split."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import qready1_reward_use_proofs as proof
from quest_reward_use_proofs import digest

ROOT = Path(__file__).resolve().parents[3]
PROOF = ROOT / 'tools/content-schema/quest-authoring/samples/completion-matrix/reward-use-proofs/qready1.json'


class M35RewardUseRebindTests(unittest.TestCase):
    def legacy_evidence(self):
        previous = json.loads(PROOF.read_bytes())
        previous.pop('completion_owner_rebind', None)
        claims = proof.load_reward_claims(ROOT)
        for case in previous['cases']:
            key = case['claim_key']
            if key in {'oteryn:reward-claim.kv.circleroom1', 'oteryn:reward-claim.kv.circleroom2'}:
                old = copy.deepcopy(claims[key])
                old['placements'][0]['quest_transition'] = 'oteryn:quest-transition/kv/' + key.rsplit('.', 1)[1] + '/chest'
                case['claim_sha256'] = digest(old)
        return previous

    def test_only_completion_reference_changes_and_recorded_pg_evidence_is_retained(self):
        previous = self.legacy_evidence()
        current = proof.rebind_m35_completion(ROOT, previous)
        self.assertEqual(current['test_sha256'], previous['test_sha256'])
        for before, after in zip(previous['cases'], current['cases']):
            self.assertEqual({k: v for k, v in before.items() if k != 'claim_sha256'},
                             {k: v for k, v in after.items() if k != 'claim_sha256'})
        self.assertEqual(len(current['completion_owner_rebind']['claims']), 2)
        self.assertEqual(proof.rebind_m35_completion(ROOT, current), current)

    def test_changed_reward_or_placement_needs_new_pg_evidence(self):
        previous = self.legacy_evidence()
        for field in ('reward', 'placement', 'repeat'):
            claims = proof.load_reward_claims(ROOT)
            claim = claims['oteryn:reward-claim.kv.circleroom1']
            if field == 'reward':
                claim['placements'][0]['reward']['items'][0]['count'] += 1
            elif field == 'placement':
                claim['placements'][0]['source_binding']['project_position']['x'] += 1
            else:
                claim['claim']['repeat']['kind'] = 'daily'
            with self.subTest(field=field), patch.object(proof, 'load_reward_claims', return_value=claims):
                with self.assertRaisesRegex(ValueError, 'changes reward USE facts'):
                    proof.rebind_m35_completion(ROOT, previous)

    def test_changed_test_or_skipped_proof_is_rejected(self):
        for field in ('test_sha256', 'duration_seconds'):
            previous = self.legacy_evidence()
            if field == 'test_sha256':
                previous[field] = '0' * 64
            else:
                previous['cases'][0][field] = 0
            with self.subTest(field=field), self.assertRaises(ValueError):
                proof.rebind_m35_completion(ROOT, previous)
