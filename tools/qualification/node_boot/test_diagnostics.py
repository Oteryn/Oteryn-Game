#!/usr/bin/env python3
"""Exercise the actual run.sh stage, traps and monitor with disposable commands."""
from pathlib import Path
import os
import re
import selectors
import signal
import subprocess
import sys
import tempfile
import time
import unittest

SOURCE = Path(__file__).resolve().with_name('run.sh')
# An explicit source path also runs the same fixture against a saved baseline.
if len(sys.argv) > 1 and sys.argv[1].endswith('run.sh'):
    SOURCE = Path(sys.argv.pop(1))
TEXT = SOURCE.read_text()


def function(name):
    ending = r"^PY_RUNTIME_DIAGNOSTICS\n.*?^}" if name == 'start_runtime_diagnostics' else r"^}"
    found = re.search(rf"(?ms)^{name}\(\).*?{ending}", TEXT)
    return found.group(0) if found else ""


class RuntimeEvidenceTests(unittest.TestCase):
    def exercise(self, signum):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            work = root / 'work'
            work.mkdir()
            (work / 'vars').touch()
            (work / 'node.log').touch()
            definitions = '\n'.join(function(name) for name in [
                'start_runtime_diagnostics', 'stop_runtime_diagnostics',
                'interrupted', 'cleanup', 'stage',
            ])
            trap_lines = '\n'.join(re.findall(r'^trap .+$', TEXT, re.M))
            script = f'''#!/usr/bin/env bash
set -Eeuo pipefail
WORK={work}
BASE={root / 'base'}
VARS="$WORK/vars"
node_log="$WORK/node.log"
NODE_PID=""
RUNTIME_DIAGNOSTICS_PID=""
ACTIVE_STAGE=setup
NODE_BOOT_WORLD=1
NODE_BOOT_SPELLS=0
NODE_BOOT_PROFILE=sorcerer
SERVICE_USER=fixture
PG_CONTAINER=fixture
result=NODE_BOOT_FAIL
declare -A STATUS=()
ORDER=()
evidence() {{ printf 'NODE_BOOT_EVIDENCE %s\\n' "$*"; }}
compose() {{ :; }}
docker() {{ :; }}
sudo() {{ :; }}
{definitions}
{trap_lines}
blocked() {{
  printf 'FIXTURE_RUNNING\\n'
  while [[ ! -e "$WORK/release" ]]; do sleep 1; done
}}
echo 'NODE_BOOT_RESULT=NODE_BOOT_PASS'
stage seam_stages -- blocked
'''
            harness = root / 'harness.sh'
            harness.write_text(script)
            output = bytearray()
            process = subprocess.Popen(['bash', str(harness)], stdout=subprocess.PIPE,
                                       stderr=subprocess.STDOUT, start_new_session=True)
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)

            def until(marker, seconds):
                deadline = time.monotonic() + seconds
                while marker not in output and time.monotonic() < deadline:
                    for key, _ in selector.select(max(0, deadline - time.monotonic())):
                        block = os.read(key.fd, 65536)
                        if not block:
                            return False
                        output.extend(block)
                return marker in output

            try:
                self.assertTrue(until(b'FIXTURE_RUNNING', 3), output.decode())
                # On the old script this is deliberately absent; the later assertions
                # then demonstrate the lost diagnostics rather than a fixture failure.
                until(b'NODE_BOOT_RUNTIME_MONITOR state=watching', 2)
                with (work / 'node.log').open('a') as log:
                    # Slow-pass noise must not consume the slots for the later cause.
                    for index in range(2000):
                        log.write(f'DURABILITY_PASS_SLOW caller=crates/gameplay/caller_{index}.rs:1:1 '
                                  'held_ms=6000 deadline_exceeded=false\n')
                    log.write('DURABILITY_PASS_RETIRED caller=crates/gameplay/example.rs:7:2 reason=no_evidence\n')
                    log.write('oteryn-game-server event=listener_connection_poll_slow ms=5650 ready=false\n')
                    log.write('oteryn-game-server event=serve_end reason=control_lost path=liveness_missed '
                              'source=crates/gameplay/serve.rs:8:3 next_command_id=17 server_sequence=23 '
                              'password=do-not-print-this-secret\n')
                    log.write('oteryn-game-server ts=1 level=error module=test event=admission_refused '
                              'error="permission denied for table secret-table" '
                              'database_url=postgres://fixture:do-not-print-this-secret@db/fixture\n')
                # Deliver INT/TERM to the fixture group while its stage waits on tee.
                # GitHub entry-shell cancellation is a separate signal boundary.
                until(b'NODE_BOOT_RUNTIME_LIMIT', 3)
                os.killpg(process.pid, signum)
                remainder, _ = process.communicate(timeout=5)
                output.extend(remainder)
            finally:
                selector.close()
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            text = output.decode()
            self.assertEqual(process.returncode, 128 + signum, text)
            self.assertEqual(text.count('cleanup=complete'), 1, text)
            self.assertIn('event=serve_end reason=control_lost path=liveness_missed', text)
            self.assertIn('event=durability_pass_retired', text)
            self.assertIn('event=listener_connection_poll_slow ready=false ms=5650', text)
            self.assertIn('signature=permission_denied', text)
            self.assertNotIn('do-not-print-this-secret', text)
            self.assertNotIn('secret-table', text)
            self.assertLess(text.index('event=serve_end'), text.index('cleanup=complete'))
            results = re.findall(r'^NODE_BOOT_RESULT=(.+)$', text, re.M)
            self.assertEqual(results, ['NODE_BOOT_PASS', 'NODE_BOOT_FAIL'], text)
            records = [line for line in text.splitlines() if line.startswith('NODE_BOOT_RUNTIME ')]
            self.assertLessEqual(len(records), 128, text)
            self.assertIn('NODE_BOOT_RUNTIME_LIMIT', text)
            self.assertIn('world_bundle=1', text)
            self.assertFalse(work.exists(), text)

    def test_term_preserves_bounded_safe_evidence_before_cleanup(self):
        self.exercise(signal.SIGTERM)

    def test_int_preserves_bounded_safe_evidence_before_cleanup(self):
        self.exercise(signal.SIGINT)


if __name__ == '__main__':
    unittest.main()
