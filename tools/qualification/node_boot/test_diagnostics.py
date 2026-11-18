#!/usr/bin/env python3
"""Exercise the actual run.sh stage, traps and monitor with disposable commands."""
from pathlib import Path
import os
import re
import selectors
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest

# The workflows run this file only; it carries the node boot tool tests with it.
from test_node_boot_tools import (  # noqa: F401
    EmptyEvidenceGateTests, KnownFailureGateTests, FailureSummaryTests, KnobTests, PgObserveTests, ReuseTests, SeamPassThroughTests,
    RoundTwoCallerTests, StageDumpTests, TimelineTests,
)

from test_provision import ProvisionTests, SpawnTests, C17BootstrapTests  # noqa: F401

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
    def test_shutdown_guard_check_handles_multiple_runtime_scopes(self):
        sql = re.search(r"echo '([^']+)' \| psql_admin \"\$database\"", function('assert_shutdown_guards')).group(1)
        with sqlite3.connect(':memory:') as database:
            database.execute('CREATE TABLE game_durability_admission_runtime_guards (ready boolean)')
            for values, expected in [([], False), ([False], True), ([False, False], True), ([False, True], False)]:
                database.execute('DELETE FROM game_durability_admission_runtime_guards')
                database.executemany('INSERT INTO game_durability_admission_runtime_guards VALUES (?)', [(value,) for value in values])
                rows = database.execute(sql).fetchall()
                # The shell must receive one truth value, independent of scope count.
                self.assertEqual(rows, [(int(expected),)], values)

    def test_served_content_revision_is_the_characters_interpretation(self):
        # NBSEAM: the quest and Task Board owners refuse a Character whose stored progression
        # content differs from the served readiness.content_revision (ProgressionContextMismatch).
        interpretation = re.search(r'^readonly INTERPRETATION=\(([^)]*)\)$', TEXT, re.M).group(1).split()
        readiness = TEXT.split('[readiness]\n', 1)[1].split('\n[', 1)[0]
        served = re.search(r'^content_revision = "([^"]*)"$', readiness, re.M).group(1)
        self.assertEqual(served.replace('${INTERPRETATION[2]}', interpretation[2]), interpretation[2])
        harness = SOURCE.parents[3].joinpath('apps/game-server/src/qualification.rs')
        if harness.exists():
            rust = harness.read_text()
            content = re.search(r'const INTERPRETATION: \[&str; 4\] = \[\s*"[^"]*",\s*"[^"]*",\s*"([^"]*)"', rust).group(1)
            self.assertEqual(content, interpretation[2])
            self.assertNotRegex(rust, r'content_revision"?: "')

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
            ('control_loss_reconcile_pending', 'soul_war_source_use'),
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

    def test_loss_publication_and_grace_keep_only_fixed_results_and_stages(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        expected = [f'event=control_loss_result kind={kind}' for kind in (
            'recorded', 'resumed_history', 'terminal', 'unknown', 'not_applicable', 'refused',
        )]
        expected.extend([
            'event=control_loss_publication stage=entry',
            'event=control_loss_publication kind=pending stage=loss_readback',
            'event=control_loss_publication kind=pending stage=native_publication',
            'event=control_loss_grace stage=entry',
        ])
        expected.extend(f'event=control_loss_grace kind=pending stage={stage}' for stage in (
            'activity_successor', 'staff_release', 'terminal_session', 'combat_hold',
            'foreground_demand', 'death_save', 'activity_return', 'activity_publish',
            'item_effects', 'release_fence', 'familiar_save', 'training_save', 'wheel_gift',
            'monk_save', 'gourmet_checkpoint', 'condition_save', 'kill_rewards',
            'release_not_applicable', 'release_not_expired', 'release_settle',
            'release_commit', 'retire',
        ))
        # Separate schema coverage from the per-event cap exercised below.
        for offset in range(0, len(expected), 8):
            batch = expected[offset:offset + 8]
            lines = [f'oteryn-game-server {fields} account=secret character=secret '
                     'transport=secret receipt=secret connection_generation=secret '
                     'lease_generation=secret state=secret database_url=secret payload=secret'
                     for fields in batch]
            lines.append('oteryn-game-server event=control_loss_grace stage=secret kind=secret')
            script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
            result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                    capture_output=True, text=True, check=True)
            for fields in batch:
                self.assertIn(fields.replace('stage=', 'failure_stage='), result.stdout)
            self.assertNotIn('secret', result.stdout)

    def test_grace_deduplicates_before_numbers_and_keeps_event_and_total_caps(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        lines = [f'oteryn-game-server event=control_loss_grace stage=combat_hold kind=pending '
                 f'ts={index} ms={index} server_sequence={index}' for index in range(200)]
        lines.extend(f'oteryn-game-server event=control_loss_grace stage={stage} kind=pending'
                     for stage in (
                         'foreground_demand', 'death_save', 'activity_return', 'activity_publish',
                         'item_effects', 'release_fence', 'familiar_save', 'training_save',
                         'wheel_gift', 'monk_save', 'gourmet_checkpoint', 'condition_save',
                         'kill_rewards', 'release_commit', 'retire',
                     ))
        # Exhausting grace's own budget leaves space for the lifecycle result.
        lines.append('oteryn-game-server event=control_loss_result kind=recorded')
        for event in (
            'serve_end', 'serve_command_end', 'command_guard_failed', 'join_prepare_failed',
            'listener_poll_slow', 'listener_connection_poll_slow', 'pass_dropped',
            'step_rejected_dawnport', 'step_rejected_source_map', 'step_rejected_dead',
            'step_rejected_lane', 'step_rejected_pvp_locks', 'step_rejected_stale_position',
            'step_rejected_pending_spell', 'step_rejected_drome', 'step_rejected_movement',
        ):
            lines.extend(f'oteryn-game-server event={event} '
                         f'source=crates/gameplay/example_{index}.rs:1:1' for index in range(32))
        script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
        result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                capture_output=True, text=True, check=True)
        records = [line for line in result.stdout.splitlines()
                   if line.startswith('NODE_BOOT_RUNTIME ')]
        grace = [line for line in records if 'event=control_loss_grace ' in line]
        self.assertEqual(len(records), 128)
        self.assertEqual(len(grace), 16)
        self.assertEqual(sum('failure_stage=combat_hold ' in line for line in grace), 2)
        self.assertIn('event=control_loss_grace records=16', result.stdout)
        self.assertIn('event=control_loss_result kind=recorded', result.stdout)

    def test_appearance_failures_keep_only_closed_classes(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        for event, classes in (
            ('appearance_initial_failed', ('observation', 'mappings', 'attach', 'encode')),
            ('appearance_observation_failed',
             ('content', 'fence', 'revision', 'state', 'visibility', 'conditions')),
        ):
            lines = [f'oteryn-game-server event={event} class={kind} '
                     'account=secret character=secret session=secret role=secret '
                     'error=secret payload=secret' for kind in classes]
            lines.append(f'oteryn-game-server event={event} class=secret')
            script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
            result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                    capture_output=True, text=True, check=True)
            for kind in classes:
                self.assertIn(f'event={event} class={kind}', result.stdout)
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

    def exercise(self, signum, *, entry_shell_only=False):
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
                'stop_pg_sampler', 'pg_observability_dump',
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
PG_SAMPLER_PID=""
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
                if entry_shell_only:
                    os.kill(process.pid, signum)
                else:
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

    def test_entry_shell_term_cancels_a_blocked_stage(self):
        self.exercise(signal.SIGTERM, entry_shell_only=True)


class KnownFailRowsTests(unittest.TestCase):
    """A FAIL row fails its stage unless known_fail_rows.txt lists it."""

    def rows(self, log, status, known):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, 'stage.log').write_text(log)
            Path(tmp, 'known.txt').write_text(known)
            script = function('classify_stage_rows') + '\nclassify_stage_rows "$1/stage.log" "$1/known.txt"'
            out = subprocess.run(['bash', '-c', 'set -Eeuo pipefail\n' + script, 'x', tmp],
                                 capture_output=True, text=True, check=True).stdout
        records = [line.split() for line in out.splitlines()]
        label = 'known' if status == 'FAIL' else 'stale'
        return tuple(','.join(row for kind, row in records if kind == item) for item in ('new', label))

    def test_new_failure_is_not_hidden_and_known_failure_is_listed(self):
        log = ('SEAM_EVIDENCE e2e_all scenario=C01.full status=FAIL seconds=1 cause=assertion root=C01.full/admission\n'
               'SEAM_EVIDENCE e2e_all scenario=Q01a.full status=FAIL seconds=1 cause=assertion\n'
               'SEAM_EVIDENCE e2e_all scenario=C03.full status=PASS seconds=1\n')
        known = '# comment\n\nC01.full assertion at C01.full/admission\nC03.full assertion at C03.full/admission\n'
        self.assertEqual(self.rows(log, 'FAIL', known), ('Q01a.full', 'C01.full'))
        self.assertEqual(self.rows(log, 'PASS', known)[1], 'C03.full')

    def test_missing_list_hides_nothing(self):
        log = 'SEAM_EVIDENCE e2e_all scenario=C01.full status=FAIL seconds=1\n'
        self.assertEqual(self.rows(log, 'FAIL', ''), ('C01.full', ''))

    def test_known_failure_requires_the_recorded_cause_and_root(self):
        known = 'C01.full assertion at C01.full/admission\nC20a.full blocked-by at S16a/oressa-reachable\n'
        for row, cause, root in (
            ('C01.full', 'transport', 'C01.full/admission'),
            ('C01.full', 'assertion', 'C01.full/baseline'),
            ('C20a.full', 'blocked-by', 'S16a/new-check'),
        ):
            with self.subTest(row=row, cause=cause, root=root):
                log = f'SEAM_EVIDENCE e2e_all scenario={row} status=FAIL seconds=1 cause={cause} root={root}\n'
                self.assertEqual(self.rows(log, 'FAIL', known), (row, ''))

    def test_known_failure_requires_its_recorded_evidence_prefix(self):
        known = 'C02.full no-progress at C02.full/walk; watch-aborted-before-verdict\n'
        log = ('SEAM_EVIDENCE e2e_all scenario=C02.full status=FAIL seconds=1 '
               'evidence=unclean-exit cause=no-progress root=C02.full/walk\n')
        self.assertEqual(self.rows(log, 'FAIL', known), ('C02.full', ''))
        self.assertEqual(self.rows(log.replace('unclean-exit', 'watch-aborted-before-verdict'), 'FAIL', known),
                         ('', 'C02.full'))

    def test_seed_list_has_one_reason_per_row(self):
        listed = SOURCE.with_name('known_fail_rows.txt').read_text().splitlines()
        rows = [line.split(None, 1) for line in listed if line.strip() and not line.startswith('#')]
        self.assertTrue(all(len(row) == 2 and row[0].endswith('.full') for row in rows))
        self.assertEqual(len({row[0] for row in rows}), len(rows))


if __name__ == '__main__':
    unittest.main()
