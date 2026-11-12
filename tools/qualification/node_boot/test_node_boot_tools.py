#!/usr/bin/env python3
"""Tests for the node boot knobs, stack reuse, PostgreSQL observability, failure summary and timeline."""
from pathlib import Path
import importlib.util
import json
import os
import re
import stat
import subprocess
import tempfile
import textwrap
import time
import unittest

HERE = Path(__file__).resolve().parent
# NODE_BOOT_RUN_SH runs the run.sh tests against another copy (a saved baseline).
RUN_SH = Path(os.environ.get('NODE_BOOT_RUN_SH', HERE / 'run.sh'))
RUN_TEXT = RUN_SH.read_text()


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def section(start, end):
    """The run.sh text from the line starting with `start` up to the line starting with `end`."""
    found = re.search(rf'(?ms)^{re.escape(start)}.*?(?=^{re.escape(end)})', RUN_TEXT)
    return found.group(0) if found else 'echo SECTION_MISSING; exit 99\n'


def function(name):
    found = re.search(rf'(?ms)^{name}\(\).*?^}}', RUN_TEXT)
    return found.group(0) if found else f'{name}() {{ echo FUNCTION_MISSING; return 99; }}'


def bash(script, env=None, cwd=None):
    base = {key: value for key, value in os.environ.items()
            if not key.startswith(('NODE_BOOT_', 'GITHUB_')) and key != 'CI'}
    base.update(env or {})
    return subprocess.run(['bash', '-c', script], capture_output=True, text=True, env=base,
                          cwd=cwd, timeout=60, check=False)


KNOBS = section('# Local parallel stacks', 'GAME_SOURCE=')


class KnobTests(unittest.TestCase):
    def knobs(self, **env):
        script = 'set -Eeuo pipefail\n' + KNOBS + 'printf "%s|%s|%s|%s" "$SERVICE_USER" "$BASE" "$NODE_BOOT_KEEP_STACK" "$NODE_BOOT_REUSE_STACK"\n'
        return bash(script, env)

    def test_defaults_are_the_ci_values(self):
        result = self.knobs()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, 'oteryn-node-boot|/srv/oteryn-node-boot|0|0')
        self.assertIn('WP5_PROJECT="${NODE_BOOT_PROJECT:-nodeboot${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}}"', RUN_TEXT)
        self.assertIn('PG_PORT="${NODE_BOOT_PG_PORT:-15433}"', RUN_TEXT)
        self.assertIn('WP5_PORT="${NODE_BOOT_PLATFORM_PORT:-18463}"', RUN_TEXT)

    def test_parallel_stack_overrides_are_accepted(self):
        with tempfile.TemporaryDirectory() as temp:
            temp = os.path.realpath(temp)
            result = self.knobs(NODE_BOOT_SERVICE_USER='oteryn-node-boot-s3', NODE_BOOT_BASE=f'{temp}/nb3',
                                TMPDIR=temp, NODE_BOOT_PROJECT='nbf3', NODE_BOOT_PG_PORT='15443',
                                NODE_BOOT_PLATFORM_PORT='18473', NODE_BOOT_GAME_PORT='17191')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(result.stdout, f'oteryn-node-boot-s3|{temp}/nb3|0|0')
        self.assertEqual(self.knobs(NODE_BOOT_BASE='/srv/nb3').returncode, 0)

    def test_unsafe_overrides_block_before_any_privileged_command(self):
        cases = {
            'base_outside_srv_or_tmpdir': [{'NODE_BOOT_BASE': '/etc'}, {'NODE_BOOT_BASE': '/srv'},
                                           {'NODE_BOOT_BASE': '/srv/'}, {'NODE_BOOT_BASE': '/srv/../etc'},
                                           {'NODE_BOOT_BASE': '/tmp', 'TMPDIR': '/tmp'},
                                           {'NODE_BOOT_BASE': '/home/x', 'TMPDIR': '/'},
                                           {'NODE_BOOT_BASE': 'srv/x'}],
            'invalid_service_user': [{'NODE_BOOT_SERVICE_USER': 'root'},
                                     {'NODE_BOOT_SERVICE_USER': 'oteryn-node-boot-$(id)'}],
            'invalid_project': [{'NODE_BOOT_PROJECT': 'Bad Project'}, {'NODE_BOOT_PROJECT': ''}],
            'invalid_port_node_boot_pg_port': [{'NODE_BOOT_PG_PORT': '80'}, {'NODE_BOOT_PG_PORT': '70000'},
                                               {'NODE_BOOT_PG_PORT': '1544x'}],
            'invalid_port_node_boot_game_port': [{'NODE_BOOT_GAME_PORT': ''}],
            'stack_reuse_is_local_only': [{'NODE_BOOT_KEEP_STACK': '1', 'CI': 'true'},
                                          {'NODE_BOOT_REUSE_STACK': '1', 'GITHUB_ACTIONS': 'true'},
                                          {'NODE_BOOT_REUSE_STACK': 'teardown', 'CI': 'true'}],
            'invalid_stack_reuse_mode': [{'NODE_BOOT_REUSE_STACK': '2'},
                                         {'NODE_BOOT_KEEP_STACK': '1', 'NODE_BOOT_REUSE_STACK': '1'}],
        }
        for reason, envs in cases.items():
            for env in envs:
                with self.subTest(reason=reason, env=env):
                    result = self.knobs(**env)
                    self.assertEqual(result.returncode, 2, result.stdout)
                    self.assertEqual(result.stdout.strip(), f'NODE_BOOT_RESULT=BLOCKED reason={reason}')


class ReuseTests(unittest.TestCase):
    GUARD = re.search(r'(?ms)^if \[\[ "\$NODE_BOOT_REUSE_STACK" != 0 \]\]; then.*?^fi\n', RUN_TEXT)

    def run_guard(self, temp, mode, extra=''):
        guard = self.GUARD.group(0) if self.GUARD else 'echo GUARD_MISSING; exit 99\n'
        script = textwrap.dedent(f'''\
            set -Eeuo pipefail
            WORK={temp}/fresh; mkdir -p "$WORK"
            BASE={temp}/base
            NODE_BOOT_REUSE_STACK={mode}
            NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0
            PG_CONTAINER=fixture-postgres
            STACK_KEPT=0
            REUSE_STATE={temp}/state.env
            docker() {{ echo true; }}
            sudo() {{ return 0; }}
            {extra}
            ''') + guard + 'printf "kept=%s work=%s pid=%s fresh=%s\\n" "$STACK_KEPT" "$WORK" "$NODE_PID" "$([[ -e ' + f'{temp}/fresh' + ' ]] && echo yes || echo no)"\n'
        return bash(script)

    def save_state(self, temp):
        script = textwrap.dedent(f'''\
            set -Eeuo pipefail
            REUSE_STATE={temp}/state.env
            NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; STACK_KEPT=0
            WORK={temp}/kept; node_log="$WORK/node.log"; failed_node_log="$WORK/failed-node.log"
            WP5_PKI="$WORK/pki"; WP5_SCRATCH="$WORK/scratch"; WP5_PORT=18463
            WP5_DB_PASSWORD=a; WP5_DB_ROOT_PASSWORD=b; WP5_APP_KEY=c; PG_PORT=15433
            PG_ADMIN_PASSWORD=d; CONTROL_PASSWORD=e; RUNTIME_PASSWORD=f
            ADMIN_URL='postgresql://x:y@127.0.0.1:15433/oteryn_node_boot'
            FRESH_SEED_HEX=00; RECOVERY_SEED_HEX=11; MAP_REVISION=digest; NODE_PID=4242
            NODE_CLOCK_FEATURES=(--features qualification-clock)
            NODE_GAMEPLAY_ENV=("OTERYN_NATIVE_GAMEPLAY_MANIFEST=/srv/a b/m.json")
            PG_OBS="$WORK/postgres"
            ''') + function('save_reuse_state') + '\nsave_reuse_state\necho "kept=$STACK_KEPT"\n'
        return bash(script)

    def test_saved_state_is_owner_only_and_restores_the_seam_inputs(self):
        with tempfile.TemporaryDirectory() as temp:
            saved = self.save_state(temp)
            self.assertEqual(saved.stdout.strip(), 'kept=1', saved.stderr)
            self.assertEqual(stat.S_IMODE(os.stat(f'{temp}/state.env').st_mode), 0o600)
            result = self.run_guard(temp, 1, 'NODE_PID=""')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(result.stdout.strip(), f'kept=1 work={temp}/kept pid=4242 fresh=no')
            arrays = bash(f'source {temp}/state.env; printf "%s|" "${{NODE_CLOCK_FEATURES[@]}}" "${{NODE_GAMEPLAY_ENV[@]}}"')
            self.assertEqual(arrays.stdout, '--features|qualification-clock|OTERYN_NATIVE_GAMEPLAY_MANIFEST=/srv/a b/m.json|')

    def test_reuse_refuses_missing_foreign_or_stopped_stacks(self):
        with tempfile.TemporaryDirectory() as temp:
            result = self.run_guard(temp, 1)
            self.assertEqual(result.stdout.strip(), 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_state_missing')
            self.save_state(temp)
            os.chmod(f'{temp}/state.env', 0o644)
            result = self.run_guard(temp, 1)
            self.assertEqual(result.stdout.strip(), 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_state_missing')
            os.chmod(f'{temp}/state.env', 0o600)
            result = self.run_guard(temp, 1, 'NODE_BOOT_GAME_PORT=17191')
            self.assertEqual(result.stdout.strip(), 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_mode_mismatch')
            result = self.run_guard(temp, 1, 'sudo() { return 1; }')
            self.assertEqual(result.stdout.strip(), 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_not_running')
            # Teardown loads the stack without keeping it, so cleanup removes it.
            result = self.run_guard(temp, 'teardown', 'sudo() { return 1; }')
            self.assertEqual(result.stdout.strip(), f'kept=0 work={temp}/kept pid=4242 fresh=no')

    def test_kept_or_reused_stack_never_reports_a_pass(self):
        self.assertIn('STATUS[stack_kept]="SKIP(shutdown and restart stages not run)"', RUN_TEXT)
        self.assertIn('result=NODE_BOOT_PARTIAL\n  exit 3', RUN_TEXT)
        self.assertIn('"$NODE_BOOT_KEEP_STACK$NODE_BOOT_REUSE_STACK" == 00 ]]; then\n  NODE_BOOT_WORLD=0 bash', RUN_TEXT)


class SeamPassThroughTests(unittest.TestCase):
    def test_seam_stage_passes_harness_knobs_only_when_set(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp, 'cargo')
            fake.write_text('#!/bin/sh\nenv | grep -E "^NODE_BOOT_(E2E_ONLY|NO_FAILFAST|NO_PROGRESS_S)=" | sort\n')
            fake.chmod(0o755)
            script = textwrap.dedent(f'''\
                set -Eeuo pipefail
                PATH={temp}:$PATH
                sleep() {{ :; }}
                NODE_BOOT_SPELLS=0; NODE_BOOT_WORLD=0; NODE_GAMEPLAY_ENV=(); NODE_CLOCK_FEATURES=()
                BASE=/srv/x; WP5_PKI=/x; ADMIN_URL=u; WORLD_ID=w; CHANNEL_ID=c; WP5_PORT=1
                ACCOUNT_ID=a; SECOND_ACCOUNT_ID=b; FRESH_KEY_ID=f; FRESH_SEED_HEX=s
                RECOVERY_KEY_ID=r; RECOVERY_SEED_HEX=t
                NODE_BOOT_E2E_ONLY=f6,core; NODE_BOOT_NO_PROGRESS_S=75
                ''') + function('seam_stages') + '\nseam_stages\n'
            result = bash(script)
            self.assertEqual(result.stdout, 'NODE_BOOT_E2E_ONLY=f6,core\nNODE_BOOT_NO_PROGRESS_S=75\n',
                             result.stderr)


class StageDumpTests(unittest.TestCase):
    def run_stage(self, command):
        with tempfile.TemporaryDirectory() as temp:
            script = textwrap.dedent(f'''\
                set -Eeuo pipefail
                WORK={temp}; VARS={temp}/vars; : > "$VARS"; node_log={temp}/node.log
                failed_node_log={temp}/failed-node.log; PG_OBS={temp}/postgres
                NODE_BOOT_WORLD=0; NODE_BOOT_SPELLS=0; ACTIVE_STAGE=setup
                RUNTIME_DIAGNOSTICS_PID=""; PG_SAMPLER_PID=4242; PG_CONTAINER=x
                declare -A STATUS=(); ORDER=()
                evidence() {{ :; }}; psql_admin() {{ :; }}; docker() {{ :; }}; compose() {{ :; }}
                durability_report_lines() {{ cat; }}; sleep() {{ :; }}
                start_runtime_diagnostics() {{ :; }}; stop_runtime_diagnostics() {{ :; }}
                pg_observe() {{ echo "PG_OBSERVE $*"; }}
                ''') + function('pg_observability_dump') + '\n' + function('stage') + f'\nstage sample -- {command}\n'
            return bash(script).stdout

    def test_each_stage_end_dumps_postgres_and_a_failed_stage_is_verbose(self):
        passed = self.run_stage('true')
        self.assertIn('PG_OBSERVE dump --stage sample --dir ', passed)
        self.assertNotIn('--verbose', passed)
        failed = self.run_stage('false')
        self.assertRegex(failed, r'PG_OBSERVE dump --stage sample --dir \S+ --activity \S+activity.log --verbose')


class PgSamplerLifecycleTests(unittest.TestCase):
    """stop_pg_sampler must end the sampler itself and leave the run's stdout closed (NBHANG-1)."""

    def test_stop_reaches_the_sampler_and_the_run_pipe_reaches_eof(self):
        with tempfile.TemporaryDirectory() as tmp:
            tools = Path(tmp, 'src/tools/qualification/node_boot')
            tools.mkdir(parents=True)
            pid_file = Path(tmp, 'sampler.pid')
            # Polls forever like the real sampler when PostgreSQL is gone; stops only on SIGTERM.
            tools.joinpath('pg_observe.py').write_text(textwrap.dedent(f'''\
                import os, signal, sys, time
                open({str(pid_file)!r}, 'w').write(str(os.getpid()))
                signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
                while True:
                    print('PG_ACTIVITY sample', flush=True)
                    time.sleep(0.05)
                '''))
            script = textwrap.dedent(f'''\
                set -Eeuo pipefail
                GAME_SOURCE={tmp}/src PG_OBS={tmp}/obs PG_CONTAINER=none PG_SAMPLER_PID=""
                ''') + function('pg_observe') + '\n' + function('start_pg_sampler') + '\n' + function('stop_pg_sampler') + textwrap.dedent('''
                start_pg_sampler
                for _ in $(seq 1 100); do [[ -s "$PID_FILE" ]] && break; sleep 0.05; done
                stop_pg_sampler
                echo STOPPED
                ''')
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith(('NODE_BOOT_', 'GITHUB_')) and key != 'CI'}
            env['PID_FILE'] = str(pid_file)
            # Like the workflow's `run.sh | tee`: the reader returns only at EOF on the pipe.
            done = subprocess.run(['bash', '-c', f'( {script} ) | cat'], capture_output=True, text=True,
                                  env=env, timeout=20, check=False)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertIn('STOPPED', done.stdout)
            self.assertNotIn('PG_ACTIVITY', done.stdout)
            sampler = int(pid_file.read_text())
            for _ in range(100):
                try:
                    os.kill(sampler, 0)
                except ProcessLookupError:
                    break
                time.sleep(0.05)
            else:
                os.kill(sampler, 9)
                self.fail('the sampler outlived stop_pg_sampler')


SERVER_LOG = '''\
2026-11-10 20:16:30.000 UTC [101] nb_runtime LOG:  connection authorized: user=nb_runtime
2026-11-10 20:16:31.562 UTC [140] nb_runtime LOG:  process 140 still waiting for ShareLock on transaction 769 after 200.300 ms
2026-11-10 20:16:31.562 UTC [140] nb_runtime DETAIL:  Process holding the lock: 132. Wait queue: 140.
2026-11-10 20:16:31.562 UTC [140] nb_runtime CONTEXT:  while updating tuple (0,1) in relation "game_characters"
2026-11-10 20:16:31.562 UTC [140] nb_runtime STATEMENT:  UPDATE game_characters SET name='Leaky Name'
\t  WHERE character_id = '01934f10-7c00-7000-8000-000000000001' AND revision = 41
2026-11-10 20:16:32.000 UTC [150] nb_runtime LOG:  duration: 812.004 ms  execute sqlx_s_3: SELECT * FROM t WHERE v = $1
2026-11-10 20:16:32.000 UTC [150] nb_runtime DETAIL:  Parameters: $1 = 'parameter-value-leak'
2026-11-10 20:16:33.000 UTC [77] oteryn_node_boot_admin LOG:  duration: 900.1 ms  statement: CREATE ROLE nb_runtime LOGIN PASSWORD 'pw-leak'
2026-11-10 20:16:34.000 UTC [160] nb_runtime ERROR:  deadlock detected
2026-11-10 20:16:34.000 UTC [160] nb_runtime DETAIL:  Process 160 waits for ShareLock on transaction 9; blocked by process 170.
\tProcess 170 waits for ShareLock on transaction 8; blocked by process 160.
\tProcess 160: UPDATE a SET v = 'deadlock-literal'
2026-11-10 20:16:34.000 UTC [160] nb_runtime HINT:  See server log for query details.
2026-11-10 20:16:34.000 UTC [160] nb_runtime STATEMENT:  UPDATE a SET v = 'deadlock-literal'
2026-11-10 20:16:35.000 UTC [180] nb_runtime LOG:  checkpoint complete
'''
LEAKS = ('Leaky', '01934f10', 'parameter-value-leak', 'pw-leak', 'deadlock-literal', 'password', '41')


class PgObserveTests(unittest.TestCase):
    pg = load('pg_observe')

    def test_redact_sql_keeps_shape_and_drops_values(self):
        redact = self.pg.redact_sql
        self.assertEqual(redact("SELECT a FROM t1 WHERE b = 'x''y' AND c = 42 AND d = $1 AND e = E'z'"),
                         "SELECT a FROM t1 WHERE b = '?' AND c = ? AND d = $1 AND e = '?'")
        self.assertEqual(redact('SELECT $q$body$q$, 1.5e3'), 'SELECT $$?$$, ?')
        self.assertEqual(redact("ALTER ROLE r PASSWORD 'pw'"), '<redacted>')
        self.assertEqual(redact('postgres://u:p@h/db'), '<redacted>')
        self.assertEqual(redact("WHERE id = '01934f10-7c00-7000-8000-000000000001"), "WHERE id = '?")
        self.assertEqual(redact('id = 01934f10-7c00-7000-8000-000000000001'), 'id = <uuid>')
        self.assertLessEqual(len(redact('SELECT ' + 'x, ' * 200)), 200)

    def test_server_log_filter_keeps_redacted_lock_deadlock_and_slow_records(self):
        kept = self.pg.filter_server_log(SERVER_LOG.splitlines())
        text = '\n'.join(kept)
        for leak in LEAKS:
            self.assertNotIn(leak, text)
        self.assertIn('[140] nb_runtime LOG: process 140 still waiting for ShareLock on transaction 769 after 200.300 ms', text)
        self.assertIn('DETAIL: Process holding the lock: 132. Wait queue: 140.', text)
        self.assertIn("STATEMENT: UPDATE game_characters SET name='?' WHERE character_id = '?' AND revision = ?", text)
        self.assertIn('LOG: duration: 812.004 ms  execute sqlx_s_3: SELECT * FROM t WHERE v = $1', text)
        self.assertIn('LOG: duration: 900.1 ms  statement: <redacted>', text)
        self.assertIn('ERROR: deadlock detected', text)
        self.assertIn('  Process 170 waits for ShareLock on transaction 8; blocked by process 160.', text)
        self.assertNotIn('connection authorized', text)
        self.assertNotIn('checkpoint', text)
        self.assertNotIn('Parameters', text)

    def test_dump_writes_stage_files_and_prints_a_bounded_summary(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp, 'bin', 'docker')
            fake.parent.mkdir()
            server_log = Path(temp, 'server.log')
            server_log.write_text(SERVER_LOG)
            fake.write_text(textwrap.dedent(f'''\
                #!/bin/sh
                if [ "$1" = logs ]; then cat {server_log} >&2; exit 0; fi
                cat > /dev/null
                echo "calls=3 total_ms=2500 mean_ms=833.3 max_ms=900 rows=3 query=UPDATE t SET v = 'stmt-leak' WHERE id = \\$1"
                '''))
            fake.chmod(0o755)
            activity = Path(temp, 'activity.log')
            activity.write_text(
                'PG_ACTIVITY ts=2000 pid=140 user=nb_runtime state=active wait=Lock.transactionid '
                'xact_start_ms=1000 xact_ms=1000 state_ms=900 blocked_by=132 query_id=7 query=UPDATE t SET v=$1\n'
                'PG_ACTIVITY ts=4000 pid=140 user=nb_runtime state=active wait=Lock.transactionid '
                'xact_start_ms=1000 xact_ms=3000 state_ms=2900 blocked_by=132 query_id=7 query=UPDATE t SET v=$1\n')
            out_dir = Path(temp, 'postgres')
            env = dict(os.environ, PATH=f'{fake.parent}:{os.environ["PATH"]}')
            command = ['python3', str(HERE / 'pg_observe.py'), 'dump', '--container', 'c', '--user', 'u',
                       '--database', 'oteryn_node_boot', '--stage', 'seam_stages', '--dir', str(out_dir),
                       '--activity', str(activity)]
            first = subprocess.run(command + ['--verbose'], capture_output=True, text=True, env=env, check=True)
            self.assertIn('NODE_BOOT_PG stage=seam_stages slow_statements=2 lock_wait_logs=1 deadlocks=1 '
                          'sampled=2 sampled_lock_waits=2 sampled_blocked=2', first.stdout)
            self.assertIn("NODE_BOOT_PG_STATEMENT calls=3 total_ms=2500 mean_ms=833.3 max_ms=900 rows=3 "
                          "query=UPDATE t SET v = '?' WHERE id = $1", first.stdout)
            self.assertIn('NODE_BOOT_PG_ACTIVITY ts=4000 pid=140', first.stdout)
            self.assertLess(len(first.stdout.splitlines()), 50)
            for leak in LEAKS + ('stmt-leak',):
                self.assertNotIn(leak, first.stdout)
                for path in out_dir.iterdir():
                    self.assertNotIn(leak, path.read_text())
            self.assertTrue((out_dir / 'statements-seam_stages.txt').exists())
            # The next stage reads only what is new since the previous dump.
            second = subprocess.run(command[:-4] + ['--stage', 'graceful_shutdown', '--dir', str(out_dir),
                                                    '--activity', str(activity)],
                                    capture_output=True, text=True, env=env, check=True)
            self.assertEqual(second.stdout.strip(), 'NODE_BOOT_PG stage=graceful_shutdown slow_statements=0 '
                             'lock_wait_logs=0 deadlocks=0 sampled=0 sampled_lock_waits=0 sampled_blocked=0')

    def test_run_sh_starts_postgres_with_observability_settings(self):
        for setting in ('-c log_lock_waits=on', '-c deadlock_timeout=200ms', '-c log_min_duration_statement=500',
                        '-c log_parameter_max_length=0', '-c shared_preload_libraries=pg_stat_statements',
                        '-c "log_line_prefix=%m [%p] %u "'):
            self.assertIn(setting, RUN_TEXT)
        self.assertIn('echo "CREATE EXTENSION pg_stat_statements" | psql_admin postgres', RUN_TEXT)
        self.assertIn('cp -R "$PG_OBS" "$NODE_BOOT_FAILURE_LOG_DIR/postgres-world', RUN_TEXT)


class FailureSummaryTests(unittest.TestCase):
    summary = load('failure_summary')

    def test_rows_group_by_root_then_gap_then_evidence(self):
        lines = [
            'SEAM_EVIDENCE E2EF6_CHECK S16a release FAIL assertion-not-completed cause=precondition root=S16a/fixture-1-released',
            'SEAM_EVIDENCE e2e_all scenario=S16a.full status=FAIL seconds=160.006 cause=precondition root=S16a/fixture-1-released',
            'SEAM_EVIDENCE e2e_all scenario=C20a.full status=FAIL seconds=0 cause=blocked-by root=S16a/fixture-1-released',
            'SEAM_EVIDENCE e2e_all scenario=C20b.full status=FAIL seconds=0 cause=blocked-by root=S16a/fixture-1-released',
            'SEAM_EVIDENCE E2EF6_GAP C19a expected backpack for 01934f10-7c00-7000-8000-000000000001 password=x',
            'SEAM_EVIDENCE e2e_all scenario=C19a.full status=FAIL seconds=6.766',
            'SEAM_EVIDENCE E2EF6_GAP P16 rat route: transport I/O failed: see https://docs.rs/x#eof',
            'SEAM_EVIDENCE E2EF1_GAP row=P17 rat route: transport I/O failed: see https://docs.rs/x#eof',
            'SEAM_EVIDENCE e2e_all scenario=P16.full status=FAIL seconds=131.2',
            'SEAM_EVIDENCE e2e_all scenario=P17.full status=FAIL seconds=12',
            'SEAM_EVIDENCE e2e_all scenario=M01.full status=FAIL seconds=0 evidence=journey-aborted:apps/x.rs',
            'SEAM_EVIDENCE e2e_all scenario=M01.turn-result status=PASS seconds=0.807',
            'SEAM_EVIDENCE e2e_all scenario=W20a.full status=FAIL seconds=1',
        ]
        output = self.summary.summarize(lines)
        self.assertEqual(output, [
            'NODE_BOOT_ROOT_CAUSES failed_rows=8 roots=5',
            'NODE_BOOT_ROOT_CAUSE root=S16a/fixture-1-released cause=precondition rows=3 blocked=2 first=S16a members=S16a,C20a,C20b',
            'NODE_BOOT_ROOT_CAUSE gap="rat route: transport I/O failed: see <url>" cause=unclassified rows=2 blocked=0 first=P16 members=P16,P17',
            'NODE_BOOT_ROOT_CAUSE gap="<redacted>" cause=unclassified rows=1 blocked=0 first=C19a members=C19a',
            'NODE_BOOT_ROOT_CAUSE evidence=journey-aborted cause=unclassified rows=1 blocked=0 first=M01 members=M01',
            'NODE_BOOT_ROOT_CAUSE no-cause-recorded cause=unclassified rows=1 blocked=0 first=W20a members=W20a',
        ])

    def test_output_is_bounded_and_appended_to_the_step_summary(self):
        lines = [f'SEAM_EVIDENCE e2e_all scenario=R{index}.full status=FAIL seconds=1 cause=assertion root=R{index}/check'
                 for index in range(30)]
        with tempfile.TemporaryDirectory() as temp:
            log = Path(temp, 'stage-seam_stages.log')
            log.write_text('\n'.join(lines) + '\n')
            summary = Path(temp, 'summary.md')
            summary.write_text('existing\n')
            result = subprocess.run(['python3', str(HERE / 'failure_summary.py'), str(log),
                                     str(Path(temp, 'stage-missing.log')), '--summary', str(summary)],
                                    capture_output=True, text=True, check=True)
            output = result.stdout.splitlines()
            self.assertEqual(len(output), 10)
            self.assertEqual(output[0], 'NODE_BOOT_ROOT_CAUSES failed_rows=30 roots=30')
            self.assertEqual(output[-1], 'NODE_BOOT_ROOT_CAUSE more_roots=22 rows=22')
            self.assertTrue(summary.read_text().startswith('existing\n### Node boot failed rows by root cause'))
        self.assertEqual(self.summary.summarize(['SEAM_EVIDENCE e2e_all scenario=A.full status=PASS']),
                         ['NODE_BOOT_ROOT_CAUSES failed_rows=0'])

    def test_run_sh_prints_the_summary_before_the_stage_table(self):
        self.assertRegex(RUN_TEXT, r'failure_summary\.py" "\$WORK"/stage-\*\.log \\\n'
                                   r'  \$\{GITHUB_STEP_SUMMARY:\+--summary "\$GITHUB_STEP_SUMMARY"\} \|\| true\n'
                                   r'echo "NODE_BOOT_STAGES"')


NODE_LOG = '''\
oteryn-game-server ts=1000 level=info module=durability event=holder_window window_ms=10000 held_ms=4200 passes=12 refused=3
oteryn-game-server ts=1000 level=info module=durability event=holder_usage caller=crates/a.rs:1:1 passes=2 held_ms=4000 refused=0
DURABILITY_PASS_SLOW caller=crates/b.rs:33:14 held_ms=1822 deadline_exceeded=false
oteryn-game-server ts=5000 level=info module=gameplay_transport event=transport_close trace=01a1c6d7-05b6-72ea outcome=admitted_then_logged_out served=true ms=3000
oteryn-game-server ts=5100 level=warn module=gameplay_transport event=admission_refused code=12 account_id=01934f10-7c00-7000-8000-000000000001 error="secret text"
oteryn-game-server ts=5200 level=info module=x event=unrelated value=1
not a log line
'''
PG_ACTIVITY = '''\
PG_ACTIVITY ts=2000 pid=140 user=nb_runtime state=idle_in_transaction wait=Client.ClientRead xact_start_ms=1500 xact_ms=500 state_ms=10 blocked_by=- query_id=1 query=LOCK TABLE a
PG_ACTIVITY ts=4000 pid=140 user=nb_runtime state=active wait=Lock.relation xact_start_ms=1500 xact_ms=2500 state_ms=10 blocked_by=132 query_id=2 query=UPDATE a SET v=$1
PG_ACTIVITY ts=4000 pid=150 user=nb_runtime state=active wait=-.- xact_start_ms=3600 xact_ms=400 state_ms=400 blocked_by=- query_id=- query=-
PG_ACTIVITY_UNAVAILABLE ts=6000
'''


class TimelineTests(unittest.TestCase):
    timeline = load('timeline')

    def test_node_log_and_pg_samples_become_chrome_trace_events(self):
        trace = self.timeline.convert(NODE_LOG.splitlines(), PG_ACTIVITY.splitlines())
        events = trace['traceEvents']
        by_name = {event['name']: event for event in events}
        self.assertEqual(by_name['holder_window'], {'name': 'holder_window', 'ph': 'C', 'ts': 1_000_000,
                                                    'pid': 1, 'args': {'held_ms': 4200, 'passes': 12, 'refused': 3}})
        slow = by_name['durability_pass_slow crates/b.rs:33:14']
        self.assertEqual((slow['ph'], slow['ts'], slow['dur'], slow['args']['approx_ts']),
                         ('X', 1_000_000 - 1_822_000, 1_822_000, 'true'))
        self.assertEqual(by_name['holder_usage crates/a.rs:1:1']['ph'], 'i')
        close = by_name['connection admitted_then_logged_out']
        self.assertEqual((close['ts'], close['dur'], close['tid']), (2_000_000, 3_000_000, 'connection 01a1c6d7'))
        refused = by_name['admission_refused']
        self.assertEqual((refused['tid'], refused['args']), ('admission', {'level': 'warn', 'module': 'gameplay_transport', 'code': '12'}))
        self.assertNotIn('unrelated', by_name)
        blocked = by_name['xact blocked']
        self.assertEqual((blocked['tid'], blocked['ts'], blocked['dur']), ('postgres pid 140', 1_500_000, 2_500_000))
        self.assertEqual(blocked['args']['waits'], 'Client.ClientRead Lock.relation')
        self.assertEqual(blocked['args']['blocked_by'], '132')
        self.assertEqual(blocked['args']['queries'], 'LOCK TABLE a | UPDATE a SET v=$1')
        self.assertEqual(by_name['xact']['tid'], 'postgres pid 150')
        self.assertEqual([event['ts'] for event in events], sorted(event['ts'] for event in events))
        text = json.dumps(trace)
        self.assertNotIn('01934f10', text)
        self.assertNotIn('secret', text)

    def test_command_line_writes_json(self):
        with tempfile.TemporaryDirectory() as temp:
            node = Path(temp, 'node.log')
            node.write_text(NODE_LOG)
            out = Path(temp, 'trace.json')
            result = subprocess.run(['python3', str(HERE / 'timeline.py'), '--node-log', str(node), '--out', str(out)],
                                    capture_output=True, text=True, check=True)
            self.assertIn('NODE_BOOT_TIMELINE events=5', result.stdout)
            self.assertEqual(len(json.loads(out.read_text())['traceEvents']), 5)


if __name__ == '__main__':
    unittest.main()
