import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from reward_claim_runtime import enabled_claims, PLACEMENTS

ROOT = Path(__file__).resolve().parents[3]
GATES = 'tools/content-schema/quest-authoring/samples/doors/gates.json'


class MixedCaptureEvidence(unittest.TestCase):
    def evaluate(self, path, raw):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_bytes(raw)
            index = root / PLACEMENTS
            index.parent.mkdir(parents=True)
            index.write_text(json.dumps({'source': {'quest_chests': {'captures': [
                {'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}]}}}))
            return enabled_claims(root, {})

    def test_exact_panpipe_gate_evidence_cannot_become_a_reward_binding(self):
        self.assertEqual(self.evaluate(GATES, (ROOT / GATES).read_bytes()), set())

    def test_other_missing_binding_documents_are_refused(self):
        with self.assertRaises(ValueError):
            self.evaluate('unknown.json', b'{"gates":[]}')
        with self.assertRaises(ValueError):
            self.evaluate(GATES, b'{"gates":[]}')
