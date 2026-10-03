import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
import wheel_authoring as wheel
from jsonschema.exceptions import ValidationError


class PredecessorBindingTests(unittest.TestCase):
    def setUp(self):
        self.raw = (wheel.ROOT / 'samples/wheel-candidate.json').read_bytes()
        self.previous = wheel.decode_json(self.raw)
        self.successor = copy.deepcopy(self.previous)
        self.successor.update(revision='r2', release={'kind': 'value_only',
            'predecessor': self.previous['revision'],
            'predecessor_sha256': hashlib.sha256(self.raw).hexdigest()})

    def test_exact_predecessor_is_required_before_gem_compatibility(self):
        wheel.validate(self.successor, self.previous, self.raw)
        changed = copy.deepcopy(self.previous)
        changed['gems']['grade_costs'][0]['basic']['gold'] += 1
        with self.assertRaisesRegex(ValueError, 'PREDECESSOR_BYTES_REQUIRED'):
            wheel.validate(self.successor, self.previous)
        with self.assertRaisesRegex(ValueError, 'PREDECESSOR_CONTENT'):
            wheel.validate(self.successor, changed, self.raw)
        with self.assertRaisesRegex(ValueError, 'PREDECESSOR_DIGEST'):
            wheel.validate(self.successor, changed, json.dumps(changed).encode())
        with self.assertRaisesRegex(ValueError, 'PREDECESSOR_DIGEST'):
            wheel.validate(self.successor, self.previous, json.dumps(self.previous).encode())

    def test_predecessor_byte_decoder_refuses_conflicting_keys(self):
        raw = self.raw.replace(b'"runtime_admitted": false',
            b'"runtime_admitted": true, "runtime_admitted": false', 1)
        self.successor['release']['predecessor_sha256'] = hashlib.sha256(raw).hexdigest()
        with self.assertRaisesRegex(ValueError, 'DUPLICATE_JSON_KEY'):
            wheel.validate(self.successor, self.previous, raw)

    def test_copying_changed_gem_contract_into_predecessor_cannot_skip_declaration(self):
        self.successor['gems']['grade_costs'][0]['basic']['gold'] += 1
        forged = copy.deepcopy(self.previous)
        forged['gems'] = copy.deepcopy(self.successor['gems'])
        with self.assertRaisesRegex(ValueError, 'PREDECESSOR_DIGEST'):
            wheel.validate(self.successor, forged, json.dumps(forged).encode())
        with self.assertRaisesRegex(ValueError, 'GEM_REVISION_DECLARATION_REQUIRED'):
            wheel.validate(self.successor, self.previous, self.raw)

    def test_successor_schema_requires_digest_and_initial_rejects_previous_bytes(self):
        for value in (None, 'invalid'):
            candidate = copy.deepcopy(self.successor)
            if value is None:
                candidate['release'].pop('predecessor_sha256')
            else:
                candidate['release']['predecessor_sha256'] = value
            with self.subTest(value=value), self.assertRaises(ValidationError):
                wheel.validate(candidate, self.previous, self.raw)
        with self.assertRaisesRegex(ValueError, 'INITIAL_WITH_PREDECESSOR'):
            wheel.validate(self.previous, previous_bytes=self.raw)

    def test_cli_checks_the_exact_previous_file_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            previous = root / 'previous.json'
            previous.write_bytes(self.raw)
            candidate = root / 'candidate.json'
            candidate.write_text(json.dumps(self.successor))
            command = [sys.executable, str(wheel.ROOT / 'wheel_authoring.py'), 'validate',
                       '--file', str(candidate), '--previous', str(previous)]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            previous.write_text(json.dumps(self.previous))
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('PREDECESSOR_DIGEST', result.stderr)
