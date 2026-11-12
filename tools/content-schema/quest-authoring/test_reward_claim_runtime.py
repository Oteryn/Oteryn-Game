import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import quest_completion_matrix as matrix
from reward_claim_runtime import enabled_claims, PLACEMENTS


class RewardClaimRuntimeTest(unittest.TestCase):
    def setUp(self):
        self.claim = {'readiness': 'ready', 'placements': [{
            'appearance_tibia_id': 2469,
            'source_binding': {'project_position': dict(x=10, y=20, z=7),
                               'legacy_unique_ids': [{'server': 'crystalserver', 'unique_id': 6025}]}}]}
        self.row = dict(claim='claim', position=[10, 20, 7], appearance=2469, unique=6025)

    def evaluate(self, rows, corrupt=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw = json.dumps({'bindings': rows}).encode()
            (root / 'capture.json').write_bytes(raw)
            index = {'source': {'quest_chests': {'captures': [{
                'path': 'capture.json', 'sha256': 'wrong' if corrupt else hashlib.sha256(raw).hexdigest()}]}}}
            path = root / PLACEMENTS
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(index))
            return enabled_claims(root, {'claim': self.claim})

    def test_all_exact_source_selectors_required(self):
        self.assertEqual({'claim'}, self.evaluate([self.row]))
        for field, value in [('claim', 'other'), ('position', [11, 20, 7]),
                             ('appearance', 2472), ('unique', 6026)]:
            row = {**self.row, field: value}
            self.assertEqual(set(), self.evaluate([row]))
        self.assertEqual(set(), self.evaluate([]))

    def test_every_placement_required_and_readiness_preserved(self):
        self.claim['placements'].append(copy.deepcopy(self.claim['placements'][0]))
        self.claim['placements'][1]['source_binding']['project_position']['x'] = 11
        self.assertEqual(set(), self.evaluate([self.row]))
        self.claim['placements'].pop()
        self.claim['readiness'] = 'waiting_implementation'
        self.assertEqual(set(), self.evaluate([self.row]))

    def test_applied_capture_digest_required(self):
        with self.assertRaisesRegex(ValueError, 'digest'):
            self.evaluate([self.row], corrupt=True)

    def test_data_only_claim_stays_pending_without_runtime_binding(self):
        definition = {'kind': 'reward_only', 'claims': [{'family': 'RewardClaim', 'key': 'claim'}]}
        self.assertEqual('REWARD_CLAIM_USE_PENDING', matrix.reward_claim_owner(
            [definition], {'claim': self.claim})['state'])
        self.assertEqual('REWARD_CLAIM_USE_ENABLED', matrix.reward_claim_owner(
            [definition], {'claim': self.claim}, {'claim'})['state'])
