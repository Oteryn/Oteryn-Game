"""Exercise the actual workflow shell before its host SSH boundary."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / '.github/workflows/oracle-operations.yml'


class RunnerBoundary(unittest.TestCase):
    def test_reachable_alternative_docker_endpoint_stops_before_ssh(self):
        text = WORKFLOW.read_text()
        script = text.split('        run: |\n', 1)[1]
        script = '\n'.join(line[10:] for line in script.splitlines())
        # Identity/socket facts are separate invariants. Start at the endpoint
        # probe so the test host's own Docker socket cannot determine this case.
        script = script[script.index('if command -v docker'):]
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            stubs = {'uname': 'echo aarch64', 'id': 'echo 1001',
                     'docker': 'exit "${PROBE_DOCKER_RESULT:-0}"',
                     'ssh': 'touch "$SSH_BOUNDARY_MARKER"'}
            for name, body in stubs.items():
                path = directory / name
                path.write_text('#!/bin/sh\n' + body + '\n')
                path.chmod(0o755)
            environment = dict(os.environ, PATH=str(directory) + ':' + os.environ['PATH'],
                               ACTUAL_RUNNER_NAME='oteryn-oracle-platform',
                               EXPECTED_RUNNER_NAME='oteryn-oracle-platform',
                               OPERATION='status', SOURCE_SHA='',
                               DOCKER_HOST='tcp://unintended-daemon:2375',
                               SSH_BOUNDARY_MARKER=str(directory / 'ssh-called'))
            rejected = subprocess.run(['bash', '-c', script], env=environment, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn(b'unexpectedly controls', rejected.stderr)
            self.assertFalse((directory / 'ssh-called').exists())
            environment['PROBE_DOCKER_RESULT'] = '1'
            allowed = subprocess.run(['bash', '-c', script], env=environment, capture_output=True)
            self.assertEqual(allowed.returncode, 0, allowed.stderr)
            self.assertTrue((directory / 'ssh-called').exists())


if __name__ == '__main__':
    unittest.main()
