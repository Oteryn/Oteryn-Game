import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import quest_reward_use_proofs as proofs


class RewardUseProofTest(unittest.TestCase):
    def test_only_current_non_skipped_real_dispatch_proofs_qualify(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            test = root / 'dispatch.rs'
            test.write_text('fn claim_once() {} // CASE_ONCE\n')
            claim = {'readiness': 'ready', 'placements': [{}]}
            case = {'claim_key': 'claim', 'test': 'claim_once', 'marker': 'CASE_ONCE',
                    'duration_seconds': 1.234, 'claim_sha256': proofs.digest(claim)}
            proof = {'schema': 'OTERYN_QUEST_REWARD_USE_PROOF/v1',
                     'test_file': 'dispatch.rs',
                     'test_sha256': hashlib.sha256(test.read_bytes()).hexdigest(),
                     'cases': [case]}
            target = root / proofs.PROOFS / 'lane.json'
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(proof))
            self.assertEqual({'claim'}, proofs.load(root, {'claim': claim}))
            for field, value in [('duration_seconds', 0.001), ('claim_sha256', 'stale'),
                                 ('test', 'absent'), ('marker', 'absent')]:
                invalid = copy.deepcopy(proof)
                invalid['cases'][0][field] = value
                target.write_text(json.dumps(invalid))
                with self.assertRaises(ValueError):
                    proofs.load(root, {'claim': claim})
            target.write_text(json.dumps(proof))
            test.write_text('changed runtime assertion')
            with self.assertRaises(ValueError):
                proofs.load(root, {'claim': claim})

    def test_absent_proof_directory_never_promotes_claims(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(set(), proofs.load(Path(directory), {}))
