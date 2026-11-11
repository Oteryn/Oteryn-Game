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
    def test_logout_failures_keep_only_closed_schema_stage_and_kind(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        lines = [
            'oteryn-game-server event=session_durability_failed stage=logout_settle kind=holder_busy '
            'database_url=secret character=secret payload=secret',
            'oteryn-game-server event=logout_unknown stage=pvp_refresh kind=unknown',
            'oteryn-game-server event=session_durability_failed stage=secret kind=secret',
        ]
        script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
        result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                capture_output=True, text=True, check=True)
        self.assertIn('event=session_durability_failed kind=holder_busy failure_stage=logout_settle', result.stdout)
        self.assertIn('event=logout_unknown kind=unknown failure_stage=pvp_refresh', result.stdout)
        self.assertNotIn('secret', result.stdout)

    def test_loss_blockers_keep_closed_stages_without_sensitive_fields(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        expected = [
            ('logout_busy', 'foreground_demand'),
            ('logout_busy', 'admitted_binding'),
            ('logout_busy', 'activity_scope'),
            ('logout_busy', 'zones_before_reconcile'),
            ('logout_busy', 'newhaven_travel'),
            ('logout_busy', 'spell_reconcile'),
            ('logout_busy', 'zones_after_reconcile'),
            ('logout_busy', 'release_fence'),
            ('logout_busy', 'release_not_applicable'),
            ('logout_busy', 'release_retry_lifted'),
            ('control_loss_reconcile_pending', 'item_effects'),
            ('control_loss_reconcile_pending', 'gourmet_checkpoint'),
            ('control_loss_reconcile_pending', 'native_cast'),
            ('control_loss_reconcile_pending', 'familiar_cast'),
            ('control_loss_reconcile_pending', 'world_items'),
            ('control_loss_reconcile_pending', 'parameters'),
            ('control_loss_reconcile_pending', 'training_save'),
            ('control_loss_reconcile_pending', 'pending_spell_commit'),
            ('control_loss_reconcile_pending', 'actor_unreserved'),
            ('control_loss_reconcile_pending', 'newhaven_travel'),
            ('session_durability_failed', 'loss_session'),
            ('session_durability_failed', 'loss_session_time'),
            ('session_durability_failed', 'training_session'),
            ('session_durability_failed', 'training_readback'),
            ('session_durability_failed', 'stamina_session'),
            ('session_durability_failed', 'hunting_stamina_readback'),
            ('session_durability_failed', 'proficiency_readback'),
            ('session_durability_failed', 'appearance_observation'),
            ('character_authority_check_failed', 'recovery_fence'),
            ('character_authority_check_failed', 'offline_training'),
            ('character_authority_check_failed', 'npc_purchase'),
            ('character_authority_check_failed', 'creation'),
            ('character_authority_check_failed', 'bazaar'),
            ('character_authority_check_failed', 'revision_chain'),
            ('character_authority_check_failed', 'appearance'),
            ('character_authority_check_failed', 'build_chain'),
            ('character_authority_check_failed', 'proficiency_history'),
            ('character_authority_check_failed', 'familiar_chain'),
            ('character_authority_check_failed', 'spell_familiar_group'),
            ('character_authority_check_failed', 'spell_item_chain'),
            ('character_authority_check_failed', 'audit_payload'),
        ]
        lines = [f'oteryn-game-server event={event} stage={stage} '
                 'account=secret transport=secret receipt=secret'
                 for event, stage in expected]
        lines.append('oteryn-game-server event=logout_busy stage=secret')
        script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
        result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                capture_output=True, text=True, check=True)
        for event, stage in expected:
            self.assertIn(f'event={event} failure_stage={stage}', result.stdout)
        self.assertNotIn('secret', result.stdout)

    def test_startup_integrity_failure_reports_only_closed_check_once(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            log = root / 'node.log'
            lines = [
                'oteryn-game-server module=durability '
                'event=character_authority_check_failed stage=creation '
                'account=secret error=secret sql=secret',
                'oteryn-game-server module=durability '
                'event=character_authority_check_failed stage=secret',
                'oteryn-game-server module=other '
                'event=character_authority_check_failed stage=bazaar',
                'oteryn-game-server module=durability event=other stage=build_chain',
            ]
            log.write_text('\n'.join(lines * 30) + '\n'
                           + ('x' * 4097) + lines[0] + '\n'
                           + 'oteryn-game-server module=durability '
                             'event=character_authority_check_failed stage=audit_payload\n')
            script = (f'node_log={log}\nNODE_BOOT_WORLD=1\n'
                      + function('report_character_authority_failures')
                      + '\nreport_character_authority_failures superseding_replacement\n')
            result = subprocess.run(['bash', '-c', script], capture_output=True,
                                    text=True, check=True)
            self.assertEqual(result.stdout.splitlines(), [
                'NODE_BOOT_CHARACTER_AUTHORITY_CHECK stage=superseding_replacement '
                'world_bundle=1 check=creation',
                'NODE_BOOT_CHARACTER_AUTHORITY_CHECK stage=superseding_replacement '
                'world_bundle=1 check=audit_payload',
            ])
            self.assertNotIn('secret', result.stdout)

    def test_step_rejections_keep_only_closed_schema_fields(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        events = [f'step_rejected_{branch}' for branch in (
            'dawnport', 'source_map', 'dead', 'lane', 'pvp_locks',
            'stale_position', 'pending_spell', 'drome', 'movement', 'holder_busy',
        )]
        lines = [f'oteryn-game-server event={event} kind=Actor character=secret '
                 'position=secret database_url=secret' for event in events]
        lines.append('oteryn-game-server event=step_rejected_movement kind=secret')
        script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
        result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                capture_output=True, text=True, check=True)
        for event in events:
            self.assertIn(f'event={event} ', result.stdout)
        self.assertIn('kind=Actor', result.stdout)
        self.assertNotIn('secret', result.stdout)

    def exercise(self, signum):
        inbound_phases = (
            'frame', 'frame_error', 'held', 'step_due', 'probe', 'server_message',
            'serene', 'quest_log', 'chat', 'visibility', 'combat', 'chase',
            'target_move', 'npc',
        )
        authority_stages = (
            'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
            'bazaar', 'revision_chain', 'appearance', 'build_chain',
            'proficiency_history', 'familiar_chain', 'spell_familiar_group',
            'spell_item_chain', 'audit_payload',
        )
        # Cover every label across both signal fixtures without exceeding the
        # monitor's limit of sixteen records for this event.
        authority_stages = authority_stages[::2] if signum == signal.SIGTERM else authority_stages[1::2]
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
                    for phase in inbound_phases:
                        log.write(f'oteryn-game-server event=serve_end reason=disconnected phase={phase} '
                                  'source=crates/gameplay/serve.rs:9:3 next_command_id=18 server_sequence=24\n')
                    log.write('oteryn-game-server event=serve_end reason=disconnected '
                              'phase=unapproved-phase-secret '
                              'source=crates/gameplay/serve.rs:10:3 next_command_id=19 server_sequence=25\n')
                    for stage in authority_stages:
                        log.write('oteryn-game-server ts=2 level=warn module=durability '
                                  f'event=character_authority_check_failed stage={stage}\n')
                    log.write('oteryn-game-server ts=3 level=warn module=durability '
                              'event=character_authority_check_failed stage=unapproved-stage-secret\n')
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
            for phase in inbound_phases:
                self.assertIn(f'event=serve_end reason=disconnected phase={phase} ', text)
            self.assertIn('event=serve_end reason=disconnected source=crates/gameplay/serve.rs:10:3 ', text)
            self.assertNotIn('unapproved-phase-secret', text)
            for stage in authority_stages:
                self.assertIn('event=character_authority_check_failed '
                              f'failure_stage={stage} check_stage={stage} ', text)
            self.assertIn('event=character_authority_check_failed ts=3 ', text)
            self.assertNotIn('unapproved-stage-secret', text)
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
