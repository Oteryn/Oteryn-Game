#!/usr/bin/env python3
"""Tests for the node boot knobs, stack reuse, PostgreSQL observability, failure summary and timeline."""
from pathlib import Path
import importlib.util
import json
import os
import re
import select
import signal
import socket
import stat
import struct
import sys
import subprocess
import tempfile
import textwrap
import time
import unittest
from unittest import mock

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
    def test_world_rejects_out_of_range_and_occupied_pvp_port(self):
        validation = section('NODE_BOOT_WORLD="${NODE_BOOT_WORLD:-1}"', '# A kept or reused stack')
        script = 'set -Eeuo pipefail\n' + KNOBS + '\nNODE_BOOT_SPELLS=0\n' + validation
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen()
            for primary in (65535, listener.getsockname()[1] - 1):
                with self.subTest(primary=primary):
                    result = bash(script, {'NODE_BOOT_GAME_PORT': str(primary)})
                    self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                    self.assertIn('reason=pvp_port_unavailable', result.stdout)

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
            PG_CONTAINER=fixture-postgres; WP5_PROJECT=fixture; SERVICE_USER=fixture
            STACK_KEPT=0
            REUSE_STATE={temp}/state.env
            docker() {{ if [[ $3 == *State.Running* ]]; then echo true; else echo fixture-container; fi; }}
            signal_node_process() {{ sudo kill -0 4242; }}
            sudo() {{ return 0; }}
            {extra}
            ''') + guard + 'printf "kept=%s work=%s pid=%s fresh=%s\\n" "$STACK_KEPT" "$WORK" "$NODE_PID" "$([[ -e ' + f'{temp}/fresh' + ' ]] && echo yes || echo no)"\n'
        return bash(script)

    def save_state(self, temp):
        script = textwrap.dedent(f'''\
            set -Eeuo pipefail
            REUSE_STATE={temp}/state.env
            BASE={temp}/base; mkdir -p "$BASE"
            SERVICE_USER=fixture; WP5_PROJECT=fixture; PG_CONTAINER_ID=fixture-container; COMPOSE_OWNER=0123456789abcdef0123456789abcdef
            NODE_PROCESS_DIR={temp}/kept/node-process.1
            NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; STACK_KEPT=0
            WORK={temp}/kept; node_log="$WORK/node.log"; failed_node_log="$WORK/failed-node.log"
            WP5_PKI="$WORK/pki"; WP5_SCRATCH="$WORK/scratch"; WP5_PORT=18463
            WP5_DB_PASSWORD=a; WP5_DB_ROOT_PASSWORD=b; WP5_APP_KEY=c; PG_PORT=15433
            PG_ADMIN_PASSWORD=d; CONTROL_PASSWORD=e; RUNTIME_PASSWORD=f
            ADMIN_URL='postgresql://x:y@127.0.0.1:15433/oteryn_node_boot'
            FRESH_SEED_HEX=00; RECOVERY_SEED_HEX=11; MAP_REVISION=digest; NODE_PID=4242
            NODE_CLOCK_FEATURES=(--features qualification-clock)
            NODE_SPAWN_FEATURES=(--features qualification-spawn)
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
        self.assertIn('"$NODE_BOOT_KEEP_STACK$NODE_BOOT_REUSE_STACK" == 00 ]]; then', RUN_TEXT)
        self.assertIn('NODE_BOOT_WORLD=0 bash "$GAME_SOURCE/tools/qualification/node_boot/run.sh"', RUN_TEXT)

    def test_entry_term_reaches_the_default_basic_child(self):
        recursion = section('# A kept or reused stack is one topology', 'if [[ "$NODE_BOOT_SPELLS" == 1 ]];')
        with tempfile.TemporaryDirectory() as temp:
            child = Path(temp, 'tools/qualification/node_boot/run.sh')
            child.parent.mkdir(parents=True)
            child.write_text("trap 'sleep 0.3; echo cleanup-complete; exit 143' TERM\necho ready\nfor i in {1..10}; do sleep 1; done\n")
            script = '''
                set -Eeuo pipefail
                NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; NODE_BOOT_KEEP_STACK=0; NODE_BOOT_REUSE_STACK=0
                sleep() { SECONDS=$((SECONDS + 51)); }
            ''' + recursion
            proc = subprocess.Popen(['bash', '-c', script], env=dict(os.environ, GAME_SOURCE=temp),
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, start_new_session=True)
            try:
                self.assertTrue(select.select([proc.stdout], [], [], 5)[0])
                self.assertEqual(proc.stdout.readline().strip(), 'ready')
                proc.send_signal(signal.SIGTERM)
                output, _ = proc.communicate(timeout=5)
                self.assertEqual(proc.returncode, 143)
                self.assertIn("cleanup-complete", output)
            finally:
                try:
                    os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                proc.wait()

    def test_kept_stack_rejects_a_different_base_or_service_account(self):
        with tempfile.TemporaryDirectory() as temp:
            self.save_state(temp)
            for override in ('BASE=/srv/other', 'SERVICE_USER=oteryn-node-boot-other'):
                with self.subTest(override=override):
                    result = self.run_guard(temp, 'teardown', override)
                    self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                    self.assertIn('reason=kept_stack_identity_mismatch', result.stdout)

    def test_cleanup_leaves_resources_that_setup_did_not_acquire(self):
        with tempfile.TemporaryDirectory() as temp:
            setup = f'''
                set -Eeuo pipefail
                WORK={temp}/test; mkdir "$WORK"
                BASE={temp}/base; mkdir "$BASE"
                REUSE_STATE={temp}/kept; touch "$REUSE_STATE"
                STACK_KEPT=0; NODE_PID=""; SERVICE_USER=test; WP5_PROJECT=test; PG_CONTAINER=test
                ACTIVE_STAGE=setup; NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; result=NODE_BOOT_FAIL
                stop_runtime_diagnostics() {{ :; }}; stop_pg_sampler() {{ :; }}; evidence() {{ :; }}
                compose() {{ echo "unexpected compose"; }}
                docker() {{ echo "unexpected docker"; }}
                sudo() {{ echo "unexpected sudo"; }}
            '''
            result = bash(setup + function('cleanup') + '\ntrap cleanup EXIT\nexit 1')
            self.assertNotIn('unexpected', result.stdout)
            self.assertTrue(Path(temp, 'kept').exists())

    def test_platform_caller_does_not_claim_a_conflicting_postgres_container(self):
        platform = (HERE.parent / 'game_platform' / 'lifecycle.sh').read_text()
        start = re.search(r'(?ms)^game_platform_start\(\).*?^}', platform).group(0)
        with tempfile.TemporaryDirectory() as temp:
            result = bash('''
            set -Eeuo pipefail
            PG_PORT=15433; PG_IMAGE=fixture; WP5_PROJECT=fixture; GP_PG_CONTAINER=""
            COMPOSE_OWNER=fixture POSTGRES_JOURNAL=$WORK
            run_interruptible() { "$@"; }
            openssl() { echo fixture; }
            docker() { return 1; }
            trap 'printf "owned=%s\\n" "$GP_PG_CONTAINER"' EXIT
        ''' + function('create_owned_postgres') + '\n' + start + '\ngame_platform_start', {'WORK': temp})
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout.strip(), 'owned=')


class SeamPassThroughTests(unittest.TestCase):
    def test_selected_world_run_uses_a_selection_verdict(self):
        final = RUN_TEXT[RUN_TEXT.index('unset FRESH_SEED_HEX RECOVERY_SEED_HEX'):]
        result = bash('''
            set -Eeuo pipefail
            NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; failures=0
            declare -A STATUS=()
            trap 'echo "$result"' EXIT
        ''' + final, {'NODE_BOOT_E2E_ONLY': 'core'})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.strip(), 'NODE_BOOT_SELECTION_PASS')

    def test_capture_mode_uses_all_world_rows(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp, 'cargo')
            fake.write_text('#!/bin/sh\nprintf "%s\\n" "${OTERYN_E2E_ROW_BATCH-unset}"\n')
            fake.chmod(0o755)
            setup = f'''
                set -Eeuo pipefail
                export PATH={temp}:$PATH E2E_ALL_CAPTURE_DIR={temp}/captures
                unset OTERYN_E2E_ROW_BATCH
                sleep() {{ :; }}
                NODE_BOOT_SPELLS=0; NODE_BOOT_WORLD=1; NODE_BOOT_LIFECYCLE=game_platform
                NODE_GAMEPLAY_ENV=(); NODE_CLOCK_FEATURES=(); NODE_SPAWN_FEATURES=()
                BASE=/srv/x; WP5_PKI=/x; ADMIN_URL=u; WORLD_ID=w; CHANNEL_ID=c; WP5_PORT=1
                ACCOUNT_ID=a; SECOND_ACCOUNT_ID=b; PACKET_ACCOUNT_ID=p; SECOND_PACKET_ACCOUNT_ID=q
                FRESH_KEY_ID=f; FRESH_SEED_HEX=s; RECOVERY_KEY_ID=r; RECOVERY_SEED_HEX=t; MAP_REVISION=m
            '''
            result = bash(setup + function('seam_stages') + '\nseam_stages\n')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), 'all')

    def test_hosted_world_supplies_a_private_capture_directory(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp, 'cargo')
            fake.write_text('#!/bin/sh\nprintf "%s\\n" "${E2E_ALL_CAPTURE_DIR-unset}"\n')
            fake.chmod(0o755)
            setup = f'''
                set -Eeuo pipefail
                export PATH={temp}:$PATH
                unset E2E_ALL_CAPTURE_DIR
                WORLD_CAPTURE_DIR={temp}/captures; export OTERYN_E2E_ROW_BATCH=200:10
                sleep() {{ :; }}
                NODE_BOOT_SPELLS=0; NODE_BOOT_WORLD=1; NODE_BOOT_LIFECYCLE=game_platform
                NODE_GAMEPLAY_ENV=(); NODE_CLOCK_FEATURES=(); NODE_SPAWN_FEATURES=()
                BASE=/srv/x; WP5_PKI=/x; ADMIN_URL=u; WORLD_ID=w; CHANNEL_ID=c; WP5_PORT=1
                ACCOUNT_ID=a; SECOND_ACCOUNT_ID=b; PACKET_ACCOUNT_ID=p; SECOND_PACKET_ACCOUNT_ID=q
                FRESH_KEY_ID=f; FRESH_SEED_HEX=s; RECOVERY_KEY_ID=r; RECOVERY_SEED_HEX=t; MAP_REVISION=m
            '''
            result = bash(setup + function('seam_stages') + '\nseam_stages\n')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), f'{temp}/captures')

    def test_install_reads_cargo_target_directory(self):
        target_assignment = re.search(r'(?m)^TARGET=.*$', RUN_TEXT).group(0)
        metadata = re.search(r'(?m)^run_interruptible .* cargo .* metadata .*$', RUN_TEXT)
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp, 'cargo')
            fake.write_text("""#!/bin/sh
printf '%s\n' '{"target_directory":"/cargo/shared output"}'
""")
            fake.chmod(0o755)
            setup = 'set -Eeuo pipefail\nGAME_SOURCE=/checkout\nrun_interruptible() { "$@"; }\n'
            script = setup + (metadata.group(0) + '\n' if metadata else '') + target_assignment
            result = bash(script + '\nprintf "%s\n" "$TARGET"', {
                'WORK': temp, 'PATH': f'{temp}:{os.environ["PATH"]}',
            })
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), '/cargo/shared output/debug')

    def test_seam_stage_passes_harness_knobs_only_when_set(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp, 'cargo')
            fake.write_text('#!/bin/sh\nenv | grep -E "^NODE_BOOT_(E2E_ONLY|NO_FAILFAST|NO_PROGRESS_S)=" | sort\n')
            fake.chmod(0o755)
            script = textwrap.dedent(f'''\
                set -Eeuo pipefail
                PATH={temp}:$PATH
                sleep() {{ :; }}
                NODE_BOOT_SPELLS=0; NODE_BOOT_WORLD=0; NODE_GAMEPLAY_ENV=(); NODE_CLOCK_FEATURES=(); NODE_SPAWN_FEATURES=()
                BASE=/srv/x; WP5_PKI=/x; ADMIN_URL=u; WORLD_ID=w; CHANNEL_ID=c; WP5_PORT=1
                ACCOUNT_ID=a; SECOND_ACCOUNT_ID=b; FRESH_KEY_ID=f; FRESH_SEED_HEX=s
                RECOVERY_KEY_ID=r; RECOVERY_SEED_HEX=t
                NODE_BOOT_E2E_ONLY=f6,core; NODE_BOOT_NO_PROGRESS_S=75
                ''') + function('seam_stages') + '\nseam_stages\n'
            result = bash(script)
            self.assertEqual(result.stdout, 'NODE_BOOT_E2E_ONLY=f6,core\nNODE_BOOT_NO_PROGRESS_S=75\n',
                             result.stderr)


class RoundTwoCallerTests(unittest.TestCase):
    def test_platform_identity_uses_privileged_process_identity(self):
        lifecycle = (HERE.parent / 'game_platform/lifecycle.sh').read_text()
        for phase in ('identity', 'native'):
            line = next(line for line in lifecycle.splitlines() if f'node_exited_during_{phase}' in line)
            self.assertIn('signal_node_process 0 "$NODE_PROCESS_DIR"', line)

    def test_platform_bootstrap_checks_the_complete_receipt(self):
        lifecycle = Path(os.environ.get('NODE_BOOT_PLATFORM_LIFECYCLE',
                                        HERE.parent / 'game_platform/lifecycle.sh')).read_text()
        bootstrap = lifecycle.split('  ACTIVE_STAGE=character_bootstrap\n')[1].split('  local character\n')[0]
        with tempfile.TemporaryDirectory() as temp:
            script = r"""
                set -Eeuo pipefail
                GAME_SOURCE=. BASE=. SECOND_ACCOUNT_ID=account WORLD_ID=world
                INTENT_OPERATIONS=(first second) INTERPRETATION=()
                python3() { :; }
                ops() {
                    if [[ -f "$WORK/receipt" ]]; then
                        echo altered > "$WORK/receipt"
                    else
                        echo original > "$WORK/receipt"
                    fi
                    echo character_bootstrap=committed
                }
                run_interruptible() { "$@"; }
                psql_admin() {
                    read -r sql
                    if [[ "$sql" == "SELECT row_to_json(r) FROM game_character_operation_receipts r WHERE operation_id='second' AND account_id='account'" ]]; then
                        cat "$WORK/receipt"
                    else
                        echo constant-character-id
                    fi
                }
                fail() { return 1; }
                game_platform_fail() { echo "$*"; exit 1; }
            """
            result = bash(script + function('assert_operator_replay') + '\nbootstrap_fixture() {\n'
                          + bootstrap + '}\nbootstrap_fixture', {'WORK': temp})
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn('character_bootstrap replay_refused', result.stdout, result.stderr)

    def test_setup_waits_are_interruptible(self):
        for action in ('build', 'test'):
            line = next(line for line in RUN_TEXT.splitlines() if f'cargo +1.100.0 {action} --locked' in line)
            self.assertTrue(line.startswith('run_interruptible '), line)
        self.assertIn('run_interruptible compose up ', RUN_TEXT)

    def test_term_cleans_base_during_identity_handoff(self):
        with tempfile.TemporaryDirectory() as temp:
            setup = r"""
                set -eu
                mkdir "$WORK"
                BASE=$WORK/../base
                mkdir "$BASE"
                stat -c '%d:%i' "$BASE" > "$WORK/base.identity"
                BASE_OWNED=0 BASE_IDENTITY="$(<"$WORK/base.identity")" PG_CONTAINER=empty PG_CONTAINER_ID=""
                ACTIVE_STAGE=setup NODE_BOOT_WORLD=0 NODE_BOOT_SPELLS=0
                stop_runtime_diagnostics() { :; }
                stop_pg_sampler() { :; }
                evidence() { :; }
                timeout() { while [[ $1 == --* ]]; do shift; done; shift; "$@"; }
                sudo() { "$@"; }
            """
            result = bash(setup + function('interrupted') + '\n' + function('cleanup')
                          + "\ntrap cleanup EXIT; trap 'interrupted TERM 143' TERM; kill -TERM $$",
                          {'WORK': f'{temp}/work'})
            self.assertEqual(result.returncode, 143, result.stderr)
            self.assertFalse(Path(temp, 'base').exists())

    def test_postgres_project_state_survives_tmpdir_changes(self):
        state = section('# Keep reuse and teardown mutually exclusive.', 'if [[ "$NODE_BOOT_REUSE_STACK" == 0 && -e "$REUSE_STATE" ]]')
        # Isolate the host state root while keeping the production TMPDIR expression.
        state = state.replace('/tmp/node-boot', '$FIXTURE/node-boot')
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for directory in ('first', 'second'):
                (root / directory).mkdir()
            for project in ('ordinary', 'postgres'):
                with self.subTest(project=project):
                    setup = 'set -eu\nWORK=$TMPDIR; WP5_PROJECT=$PROJECT\n'
                    journal = 'mkdir -p "$POSTGRES_JOURNAL"; touch "$POSTGRES_JOURNAL/record"\n'
                    env = dict(FIXTURE=temp, PROJECT=project, TMPDIR=str(root / 'first'))
                    write = bash(setup + state + journal, env)
                    self.assertEqual(write.returncode, 0, write.stdout + write.stderr)
                    read = bash(setup + state + 'test -f "$POSTGRES_JOURNAL/record"', dict(env, TMPDIR=str(root / 'second')))
                    self.assertEqual(read.returncode, 0, read.stdout + read.stderr)

    def test_postgres_recovery_with_legacy_and_concurrent_journals(self):
        owner = '0123456789abcdef0123456789abcdef'
        legacy_discovery = function('recover_legacy_postgres').replace(
            ' /tmp "${NODE_BOOT_LEGACY_TMPDIR:-}"',
            ' "$FIXTURE/default" "${NODE_BOOT_LEGACY_TMPDIR:-}"',
        )
        for service in ('postgres', 'portal-postgres'):
            for scenario in ('legacy', 'legacy_custom', 'legacy_default', 'legacy_pending', 'legacy_locked',
                             'legacy_scan_failed', 'legacy_scan_timeout', 'legacy_foreign_timeout',
                             'legacy_replay_deadline', 'vanished_first',
                             'late_registration', 'inventory_failed', 'foreign_discovery_timeout',
                             'budget_starvation', 'probe_starvation', 'foreign_lock',
                             'foreign_late_registration', 'foreign_probe_timeout'):
                with self.subTest(service=service, scenario=scenario), tempfile.TemporaryDirectory() as temp:
                    root = Path(temp)
                    current = root / 'tmp'
                    original = root / 'original'
                    default = root / 'default'
                    locks = root / 'locks'
                    journals = root / 'journal'
                    for path in (current, original, default, locks, journals):
                        path.mkdir()
                    work = root / 'work'
                    work.mkdir()
                    name = f'fixture-{service}'
                    if scenario.startswith('legacy'):
                        prior = {'legacy_custom': original, 'legacy_default': default}.get(scenario, current)
                        journal = prior / f'node-boot-locks-{os.getuid()}' / 'postgres/fixture'
                        journal.parent.parent.mkdir(mode=0o700)
                    else:
                        journal = journals / 'fixture'
                    journal.mkdir(parents=True)
                    marker_owner = 'invalid-owner' if scenario == 'legacy_replay_deadline' else owner
                    record = journal / f'{name}.{marker_owner}'
                    record.touch()
                    replay_record = journal / f'{name}.{owner}'
                    if scenario == 'legacy_replay_deadline':
                        replay_record.touch()
                    first = journals / 'aardvark' / f'aardvark-postgres.{owner}'
                    if scenario in ('vanished_first', 'late_registration', 'foreign_discovery_timeout'):
                        first.parent.mkdir()
                        first.touch()
                    if scenario in ('budget_starvation', 'probe_starvation'):
                        for index in range(28 if scenario == 'probe_starvation' else 13):
                            project = f'earlier{index:02}'
                            pending = journals / project
                            pending.mkdir()
                            (pending / f'{project}-postgres.{owner}').touch()
                    if scenario not in ('late_registration', 'legacy_pending', 'foreign_late_registration', 'foreign_probe_timeout'):
                        (root / 'registered').touch()
                    setup = r"""
                        set -Eeuo pipefail
                        WP5_PROJECT=fixture lock_directory=$FIXTURE/locks POSTGRES_JOURNALS=$FIXTURE/journal
                        WORK=$FIXTURE/work
                        if [[ "$SCENARIO" == foreign_lock || "$SCENARIO" == legacy_foreign_timeout ||
                              "$SCENARIO" == foreign_late_registration || "$SCENARIO" == probe_starvation ||
                              "$SCENARIO" == foreign_probe_timeout ]]; then
                            WP5_PROJECT=other
                        fi
                        exec 9>>"$lock_directory/$WP5_PROJECT"
                        flock -n 9
                        evidence() { echo "$*"; }
                        unset SECONDS
                        SECONDS=0
                        flock() {
                            command flock "$@" || return
                            # Advance the virtual clock after acquiring the legacy lock.
                            if [[ "$*" == '-n 7' ]]; then SECONDS=23; fi
                        }
                        timeout() {
                            while [[ $1 == --* ]]; do shift; done
                            local duration=$1
                            shift
                            if [[ "$1" == find && "$SCENARIO" == legacy*timeout ]]; then
                                [[ "$duration" == 37s ]] || return 98
                                [[ ! -e /proc/$BASHPID/fd/7 && ! -e /proc/$BASHPID/fd/8 &&
                                   ! -e /proc/$BASHPID/fd/9 ]] || return 99
                                return 124
                            fi
                            "$@"
                        }
                        find() {
                            if [[ "$SCENARIO" == legacy_replay_deadline ]]; then
                                printf '%s\0%s\0' "$LEGACY_RECORD" "$REPLAY_RECORD"
                                return 0
                            fi
                            if [[ "$SCENARIO" == legacy_scan_failed ]]; then
                                printf '%s\0' "$LEGACY_RECORD"
                                return 1
                            fi
                            command find "$@"
                        }
                        docker() {
                            case "$1" in
                              container)
                                if [[ "$2" == inspect ]]; then
                                    if [[ "$SCENARIO" == probe_starvation && " $* " == *earlier* ]]; then
                                        echo "$((SECONDS + 2))" > "$FIXTURE/clock"
                                        return 1
                                    fi
                                    if [[ "$SCENARIO" == foreign_probe_timeout ]]; then
                                        if [[ "$CONTAINER_NAME" == *-portal-postgres ]]; then return 137; fi
                                        return 124
                                    fi
                                    [[ " $* " == *" $CONTAINER_NAME "* && -f "$FIXTURE/registered" ]] || return 1
                                    flock -n "$lock_directory/fixture" true && return 98
                                    printf '/%s\n' "$CONTAINER_NAME"
                                    # Docker may return a row alongside an error for the other name.
                                    return 1
                                fi
                                if [[ " $* " == *" --format "* ]]; then
                                    if [[ "$SCENARIO" == foreign_late_registration || "$SCENARIO" == foreign_probe_timeout ]] &&
                                       [[ "${*: -1}" == '{{.Names}}' ]]; then
                                        touch "$FIXTURE/registered"
                                        return 0
                                    fi
                                    if [[ " $* " == *--filter*fixture-postgres* ]] &&
                                        flock -n "$lock_directory/fixture" true; then
                                        echo 'scoped inventory ran without the fixture project lock' >&2
                                        return 98
                                    fi
                                    if [[ "$SCENARIO" == inventory_failed ]]; then return 124; fi
                                    if [[ "$SCENARIO" == foreign_discovery_timeout && "${*: -1}" == '{{.Names}}' ]]; then
                                        if [[ -f "$FIXTURE/removed" ]]; then touch "$FIXTURE/requested-first"; fi
                                        return 124
                                    fi
                                    if [[ "$SCENARIO" == budget_starvation && " $* " == *earlier* ]]; then
                                        echo "$((SECONDS + 5))" > "$FIXTURE/clock"
                                        return 0
                                    fi
                                    if [[ "$SCENARIO" == vanished_first ]]; then rm -f -- "$FIRST"; fi
                                    if [[ "$SCENARIO" == late_registration && " $* " == *--filter*fixture-postgres* ]]; then
                                        touch "$FIXTURE/registered"
                                    fi
                                    if [[ -f "$FIXTURE/registered" ]]; then
                                        if [[ "${*: -1}" == '{{.Names}}' ]]; then
                                            echo "$CONTAINER_NAME"
                                        else
                                            echo "$CONTAINER_NAME $OWNER"
                                        fi
                                    fi
                                elif [[ -f "$FIXTURE/registered" && " $* " == *"name=^/$CONTAINER_NAME$ "* ]]; then
                                    echo owned-id
                                fi ;;
                              rm)
                                [[ "${*: -1}" == owned-id ]] || return 1
                                mv "$FIXTURE/registered" "$FIXTURE/removed"
                                echo "$SECONDS" > "$FIXTURE/removed-at" ;;
                            esac
                        }
                    """
                    # Remove the first entry after the old glob expands, before
                    # the old first-entry gate runs. New code has no such gate.
                    if scenario == 'vanished_first':
                        setup += r"""
                            set -T
                            trap 'if [[ $BASH_COMMAND == "[[ -f "* && $BASH_COMMAND == *"[0]"* ]]; then
                                rm -f -- "$FIRST"
                            fi' DEBUG
                        """
                    if scenario in ('budget_starvation', 'probe_starvation'):
                        setup += r"""
                            unset SECONDS
                            SECONDS=0
                            set -T
                            echo 0 > "$FIXTURE/clock"
                            trap 'read -r SECONDS < "$FIXTURE/clock"' DEBUG
                        """
                    if scenario == 'legacy_replay_deadline':
                        setup += r"""
                            set -T
                            trap 'if [[ $BASH_COMMAND == marker=* ]]; then SECONDS=60; fi' DEBUG
                        """
                    if scenario == 'legacy_locked':
                        setup += 'exec 6>>"$LEGACY_LOCK"; flock -n 6\n'
                    script = setup + '\n'.join(function(n) for n in (
                        'cleanup_owned_postgres', 'recover_project_postgres',
                        'import_legacy_postgres_project', 'recover_owned_postgres',
                    )) + '\n' + legacy_discovery + '\nrecover_owned_postgres\n'
                    env = dict(FIXTURE=temp, TMPDIR=str(current), SCENARIO=scenario,
                               FIRST=str(first), CONTAINER_NAME=name, OWNER=owner,
                               LEGACY_LOCK=str(journal.parent.parent / 'fixture'), LEGACY_RECORD=str(record),
                               REPLAY_RECORD=str(replay_record),
                               NODE_BOOT_LEGACY_TMPDIR=str(original) if scenario == 'legacy_custom' else '')
                    result = bash(script, env)
                    if scenario == 'foreign_probe_timeout':
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        status = 137 if service == 'portal-postgres' else 124
                        self.assertIn(f'postgres_recovery=probe_timeout project=fixture exit={status}', result.stdout)
                        self.assertTrue((root / 'registered').exists())
                        self.assertTrue(record.exists())
                        self.assertFalse((root / 'removed').exists())
                        continue
                    if scenario == 'legacy_replay_deadline':
                        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                        self.assertIn('postgres_recovery=deadline operation=legacy_import', result.stdout)
                        self.assertTrue(record.exists())
                        self.assertTrue(replay_record.exists())
                        self.assertFalse((journals / 'fixture' / replay_record.name).exists())
                        continue
                    if scenario in ('legacy_locked', 'inventory_failed', 'legacy_scan_failed',
                                    'legacy_scan_timeout', 'legacy_foreign_timeout'):
                        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                        reason = {
                            'legacy_locked': 'legacy_project_in_use',
                            'inventory_failed': 'inventory_failed',
                            'legacy_scan_failed': 'legacy_scan_failed',
                            'legacy_scan_timeout': 'legacy_scan_failed',
                            'legacy_foreign_timeout': 'legacy_scan_failed',
                        }[scenario]
                        self.assertIn(f'postgres_recovery={reason} project=fixture', result.stdout)
                        if scenario in ('legacy_scan_timeout', 'legacy_foreign_timeout'):
                            self.assertIn('exit=124', result.stdout)
                        self.assertTrue(record.exists())
                        if scenario.startswith('legacy'):
                            self.assertFalse((journals / 'fixture' / record.name).exists())
                        self.assertFalse((root / 'removed').exists())
                        continue
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    if scenario == 'legacy_pending':
                        self.assertFalse(record.exists())
                        self.assertTrue((journals / 'fixture' / record.name).exists())
                        (root / 'registered').touch()
                        result = bash(script, dict(env, TMPDIR=str(original)))
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertTrue((root / 'removed').exists(), result.stdout + result.stderr)
                    self.assertFalse(record.exists())
                    if scenario == 'foreign_discovery_timeout':
                        self.assertTrue((root / 'requested-first').exists())
                        self.assertIn('postgres_recovery=deferred operation=foreign_discovery reason=inventory_failed',
                                      result.stdout)
                        self.assertTrue(first.exists())
                    if scenario == 'budget_starvation':
                        self.assertEqual((root / 'clock').read_text().strip(), '0')
                    if scenario == 'probe_starvation':
                        self.assertEqual((root / 'removed-at').read_text().strip(), '0')
                        self.assertEqual((root / 'clock').read_text().strip(), '56')

    def test_postgres_cleanup_failure_retains_node_boot_artifacts(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            work = root / 'work'
            work.mkdir()
            (work / 'node.log').write_text('node evidence\n')
            (work / 'postgres').mkdir()
            (work / 'postgres' / 'samples').write_text('database evidence\n')
            setup = r"""
                set -eu
                result=NODE_BOOT_PASS PG_CONTAINER=test PG_CONTAINER_ID=id
                NODE_BOOT_WORLD=1 NODE_BOOT_SPELLS=0 ACTIVE_STAGE=finished
                node_log=$WORK/node.log failed_node_log=$WORK/failed.log PG_OBS=$WORK/postgres
                stop_runtime_diagnostics() { :; }
                stop_pg_sampler() { :; }
                cleanup_owned_postgres() { POSTGRES_CLEANUP_FAILED=1; }
                evidence() { echo "$*"; }
            """
            artifacts = root / 'artifacts'
            result = bash(setup + function('cleanup') + '\ntrap cleanup EXIT\nexit 0',
                          {'WORK': str(work), 'NODE_BOOT_FAILURE_LOG_DIR': str(artifacts)})
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn('cleanup=incomplete', result.stdout)
            self.assertEqual((artifacts / 'node.log').read_text(), 'node evidence\n')
            self.assertEqual((artifacts / 'postgres-world1-spells0' / 'samples').read_text(), 'database evidence\n')

    def test_platform_wrapper_rejects_pass_on_node_boot_failure(self):
        wrapper = (HERE.parent / 'game_platform/run.sh').read_text()
        cleanup = re.search(r'(?ms)^cleanup\(\).*?^}', wrapper).group(0)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scratch = root / 'scratch'
            scratch.mkdir()
            (scratch / 'node-boot.log').touch()
            (scratch / 'diagnostics').mkdir()
            (scratch / 'result').write_text('GAME_PLATFORM_RESULT=PASS\n')
            result = bash(cleanup + '\ntrap cleanup EXIT\nexit 1', {
                'scratch': str(scratch), 'TMPDIR': temp,
                'GAME_PLATFORM_RESULT_FILE': str(scratch / 'result'),
            })
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn('GAME_PLATFORM_RESULT=FAIL', result.stdout)
            self.assertNotIn('GAME_PLATFORM_RESULT=PASS', result.stdout)

    def test_reconciles_postgres_after_create_timeout(self):
        lifecycle = Path(os.environ.get('NODE_BOOT_PLATFORM_LIFECYCLE',
                                        HERE.parent / 'game_platform/lifecycle.sh')).read_text()
        platform_cleanup = re.search(r'(?ms)^game_platform_cleanup\(\).*?^}', lifecycle).group(0)
        platform_start = re.search(r'(?ms)^game_platform_start\(\).*?^}', lifecycle).group(0)
        platform_entry = section('# Each Compose object carries this run', 'else\nrun_interruptible compose config --quiet') + 'fi\n'

        def check_fixture_processes(token):
            leaked = []
            for environ in Path('/proc').glob('[0-9]*/environ'):
                try:
                    if token in environ.read_bytes().split(b'\0'):
                        os.kill(int(environ.parent.name), signal.SIGKILL)
                        leaked.append(environ.parent.name)
                except (ProcessLookupError, FileNotFoundError, PermissionError):
                    continue
            self.assertEqual(leaked, [], 'fixture leaked live processes')

        entry_traps = section('trap cleanup EXIT', 'if [[ "$NODE_BOOT_REUSE_STACK" == teardown ]]')
        for service, source in [('game', RUN_TEXT), ('portal', lifecycle)]:
            for client_status, send_term, outcome in [
                (124, True, 'late'),
                (137, False, 'late'),
                (1, False, 'immediate'),
                (1, False, 'lookup_budget'),
                (1, False, 'remove_failed'),
                (1, False, 'fixture_timeout'),
            ]:
                with self.subTest(service=service, client_status=client_status, outcome=outcome), \
                        tempfile.TemporaryDirectory() as temp:
                    root = Path(temp)
                    (root / 'work').mkdir()
                    journal = root / 'locks/postgres/fixture'
                    journal.mkdir(parents=True)
                    name = f'fixture-{"portal-" if service == "portal" else ""}postgres'
                    create = platform_entry if service == 'portal' else next(
                        line.strip().removesuffix(' \\') for line in source.splitlines()
                        if line.lstrip().startswith('create_owned_postgres ')
                    )
                    setup = r"""
                        set -Eeuo pipefail
                        WP5_PROJECT=fixture PG_CONTAINER=fixture-postgres PG_PORT=5432 PG_IMAGE=fixture
                        COMPOSE_OWNER=0123456789abcdef0123456789abcdef ACTIVE_STAGE=setup
                        NODE_BOOT_WORLD=0 NODE_BOOT_SPELLS=0 NODE_BOOT_REUSE_STACK=0 result=NODE_BOOT_FAIL
                        COMPOSE_OWNED=0 GP_HARNESS_PID="" GP_PG_CONTAINER=""
                        if [[ "$SERVICE" == portal ]]; then
                            NODE_BOOT_LIFECYCLE=game_platform
                        fi
                        lock_directory=$FIXTURE/locks POSTGRES_JOURNALS=$FIXTURE/locks/postgres
                        POSTGRES_JOURNAL=$POSTGRES_JOURNALS/fixture
                        stop_runtime_diagnostics() { :; }
                        stop_pg_sampler() { :; }
                        game_platform_retain_test_logs() { :; }
                        evidence() { echo "$*"; }
                        openssl() { echo fixture; }
                        timeout() {
                            while [[ $1 == --* ]]; do shift; done
                            if [[ "$2 $3" == 'docker container' &&
                                  " $* " == *" --filter name=^/$CONTAINER_NAME$ "* ]]; then
                                echo "$1" >> "$FIXTURE/lookup-budgets"
                            fi
                            shift
                            "$@"
                        }
                        sleep() {
                            case "$OUTCOME" in
                              late) SECONDS=31 ;;
                              lookup_budget) SECONDS=$((SECONDS + 29)) ;;
                            esac
                        }
                        docker() {
                            case "$1" in
                              create)
                                echo creating >&2
                                local end=$((SECONDS + 2))
                                until [[ -f "$FIXTURE/allow-client-exit" ]]; do
                                    (( SECONDS < end )) || return 99
                                    command sleep 0.02
                                done
                                touch "$FIXTURE/client-exited"
                                return "$CLIENT_STATUS" ;;
                              container)
                                if [[ " $* " == *" --format "* ]]; then
                                    if [[ -f "$FIXTURE/registered" ]]; then
                                        if [[ "${*: -1}" == '{{.Names}}' ]]; then
                                            echo "$CONTAINER_NAME"
                                        else
                                            echo "$CONTAINER_NAME 0123456789abcdef0123456789abcdef"
                                        fi
                                    fi
                                    return 0
                                fi
                                [[ " $* " == *" --filter name=^/$CONTAINER_NAME$ "* &&
                                   " $* " == *" --filter label=org.oteryn.node-boot.owner=0123456789abcdef0123456789abcdef "* ]] || return 0
                                echo lookup >> "$FIXTURE/lookups"
                                if [[ -f "$FIXTURE/registered" ]]; then
                                    echo owned-id
                                elif [[ "$OUTCOME" != late ]]; then
                                    touch "$FIXTURE/registered"
                                fi ;;
                              rm)
                                [[ "${*: -1}" == owned-id && "$OUTCOME" != remove_failed ]] || return 1
                                mv "$FIXTURE/registered" "$FIXTURE/removed" ;;
                            esac
                        }
                    """
                    script = setup + '\n'.join(function(n) for n in (
                        'interrupted', 'run_interruptible', 'create_owned_postgres',
                        'cleanup_owned_postgres', 'recover_project_postgres', 'import_legacy_postgres_project',
                        'recover_legacy_postgres', 'recover_owned_postgres', 'cleanup',
                    )) + '\n' + platform_cleanup + '\n' + platform_start + '\n'
                    env = dict(os.environ, WORK=str(root / 'work'), FIXTURE=temp, SERVICE=service,
                               CLIENT_STATUS=str(client_status), CONTAINER_NAME=name, OUTCOME=outcome,
                               QUALIFICATION_FIXTURE_TOKEN=temp)
                    token = f'QUALIFICATION_FIXTURE_TOKEN={temp}'.encode()
                    self.addCleanup(check_fixture_processes, token)
                    with subprocess.Popen(
                        ['bash', '-c', script + entry_traps + create],
                        env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                        text=True, start_new_session=True,
                    ) as proc:
                        try:
                            self.assertTrue(select.select([proc.stdout], [], [], 3)[0])
                            self.assertEqual(proc.stdout.readline().strip(), 'creating')
                            if send_term:
                                proc.send_signal(signal.SIGTERM)
                            if outcome == 'fixture_timeout':
                                with self.assertRaises(subprocess.TimeoutExpired):
                                    proc.communicate(timeout=0.05)
                            else:
                                (root / 'allow-client-exit').touch()
                                output, _ = proc.communicate(timeout=4)
                        finally:
                            # The controller owns publication and releases the
                            # bounded mock client even when the fixture times out.
                            (root / 'allow-client-exit').touch()
                            if proc.poll() is None:
                                try:
                                    output, _ = proc.communicate(timeout=4)
                                except subprocess.TimeoutExpired:
                                    proc.kill()
                                    raise
                    check_fixture_processes(token)
                    if outcome == 'fixture_timeout':
                        self.assertEqual(proc.returncode, client_status, output)
                        self.assertTrue((root / 'client-exited').exists(), output)
                        self.assertTrue((root / 'removed').exists(), output)
                        continue
                    self.assertEqual(proc.returncode, 143 if send_term else client_status, output)
                    self.assertTrue((root / 'client-exited').exists(), output)
                    if outcome in ('immediate', 'lookup_budget'):
                        self.assertTrue((root / 'removed').exists(), output)
                        if outcome == 'lookup_budget':
                            self.assertEqual((root / 'lookup-budgets').read_text().splitlines(), ['5s', '1s'])
                        continue
                    self.assertIn('cleanup=incomplete', output)
                    self.assertEqual(len(list(journal.iterdir())), 1)
                    if outcome == 'late':
                        self.assertEqual((root / 'lookups').read_text().splitlines(), ['lookup'])
                        (root / 'registered').touch()
                    else:
                        self.assertIn('postgres_create_cleanup=remove_failed', output)
                    # Start with a fresh owner and run the same recovery call used
                    # by the entry shell before it begins a new provisioning pass.
                    recover = next((line.strip() for line in RUN_TEXT.splitlines()
                                    if line.strip() == 'run_interruptible recover_owned_postgres'), 'run_interruptible recover_owned_postgres')
                    recovery = 'WP5_PROJECT=next-project COMPOSE_OWNER=ffffffffffffffffffffffffffffffff\n' + recover
                    recovery_env = dict(env, OUTCOME='late')
                    locked = bash(script + 'exec 7>>"$lock_directory/fixture"; flock -n 7\n' + recovery,
                                  recovery_env)
                    self.assertEqual(locked.returncode, 0, locked.stdout + locked.stderr)
                    self.assertTrue((root / 'registered').exists(), locked.stdout + locked.stderr)
                    for project in ('fixture', 'next-project'):
                        failed = bash(script + f'WP5_PROJECT={project} COMPOSE_OWNER=ffffffffffffffffffffffffffffffff\n'
                                      + entry_traps + 'echo provisioned', dict(env, OUTCOME='remove_failed'))
                        self.assertEqual(failed.returncode, 1, failed.stdout + failed.stderr)
                        self.assertIn('cleanup=incomplete', failed.stdout)
                        self.assertNotIn('provisioned', failed.stdout)
                    result = bash(script + recovery, recovery_env)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertTrue((root / 'removed').exists(), result.stdout + result.stderr)
                    self.assertEqual(list(journal.glob("*")), [])

    def test_term_cleans_portal_before_pid_assignment(self):
        lifecycle = Path(os.environ.get('NODE_BOOT_PLATFORM_LIFECYCLE',
                                        HERE.parent / 'game_platform/lifecycle.sh')).read_text()
        cleanup = re.search(r'(?ms)^game_platform_cleanup\(\).*?^}', lifecycle).group(0)
        launch = lifecycle.split('  export PORTAL_E2E_NATIVE_SCOPES=')[1].split('\n', 1)[1]
        launch = launch.split('  deadline=', 1)[0]
        launch = launch.replace('bash "$PLATFORM_SOURCE/portal-rs/e2e/start.sh" --seed-plan "$WORK/seed-plan.json"',
                                "bash -c 'exec sleep 5'")
        with tempfile.TemporaryDirectory() as temp:
            setup = r"""
                set -ETeu
                GP_HARNESS_PID="" GP_PG_CONTAINER="" WP5_PROJECT=fixture
                game_platform_retain_test_logs() { :; }
                cleanup_owned_postgres() { :; }
                evidence() { :; }
                ACTIVE_STAGE=setup NODE_BOOT_WORLD=1
                exec 9>/dev/null
                trap game_platform_cleanup EXIT
                trap 'interrupted TERM 143' TERM
                trap 'if [[ "$BASH_COMMAND" == "GP_HARNESS_PID=\$!" ]]; then
                    printf "%s\n" "$!" > "$WORK/pid"
                    kill -TERM "$$"
                fi' DEBUG
            """
            script = '\n'.join((function('interrupted'), cleanup, setup, 'launch_portal() {', launch, '}', 'launch_portal'))
            result = bash(script, {'WORK': temp})
            pid = int(Path(temp, 'pid').read_text())
            try:
                self.assertEqual(result.returncode, 143, result.stderr)
                stat_file = Path(f'/proc/{pid}/stat')
                self.assertTrue(not stat_file.exists() or stat_file.read_text().rsplit(') ', 1)[1].startswith('Z '))
            finally:
                try:
                    os.killpg(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass

    def test_entry_term_interrupts_setup_and_stage_diagnostics(self):
        lifecycle = Path(os.environ.get('NODE_BOOT_PLATFORM_LIFECYCLE',
                                        HERE.parent / 'game_platform/lifecycle.sh')).read_text()
        portal = re.search(r'(?ms)^game_platform_start\(\).*?^}', lifecycle).group(0)
        bootstrap = lifecycle.split('  ACTIVE_STAGE=character_bootstrap\n')[1].split('  local character\n')[0]
        with tempfile.TemporaryDirectory() as temp:
            for phase in ('setup', 'dump', 'secret', 'portal', 'bootstrap', 'replay', 'route'):
                with self.subTest(phase=phase):
                    setup = r"""
                        set -Eeuo pipefail
                        PG_SAMPLER_PID=fixture PG_OBS=$WORK
                        node_log=$WORK/node.log failed_node_log=$WORK/failed.log VARS=$WORK/vars
                        : > "$node_log"; : > "$VARS"
                        ACTIVE_STAGE=setup NODE_BOOT_WORLD=0 NODE_BOOT_SPELLS=0
                        declare -A STATUS=(); ORDER=()
                        evidence() { echo "$*"; }
                        stop_runtime_diagnostics() { :; }
                        blocked() { echo blocked >&2; command sleep 3; }
                        pg_observe() { blocked; }
                        timeout() { blocked; }
                    """
                    functions = ('interrupted', 'pg_observability_dump', 'stage', 'secret_to', 'create_owned_postgres')
                    helper = function('run_interruptible')
                    invocation = ('run_interruptible blocked' if 'set -m' in helper else 'blocked') if phase == 'setup' else 'stage sample -- true'
                    if phase == 'secret':
                        invocation = 'secret_to root input output'
                    if phase == 'portal':
                        setup += r'''
                            PG_PORT=15433 PG_IMAGE=fixture WP5_PROJECT=fixture COMPOSE_OWNER=fixture POSTGRES_JOURNAL=$WORK
                            docker() {
                                if [[ "$1" == create ]]; then
                                    until [[ "$1" == --cidfile ]]; do shift; done
                                    echo fixture-id > "$2"
                                else
                                    blocked
                                fi
                            }
                        ''' + portal + '\n'
                        invocation = 'game_platform_start'
                    if phase == 'route':
                        setup += '\nGAME_SOURCE=.\npython3() { blocked; }\n'
                        invocation = re.search(r'(?m)^  .*python3 ".*?/route_check.py".*?\n    .*$', portal).group(0)
                    if phase in ('bootstrap', 'replay'):
                        setup += r'''
                            GAME_SOURCE=. BASE=. SECOND_ACCOUNT_ID=account WORLD_ID=world
                            INTENT_OPERATIONS=(first second) INTERPRETATION=()
                            python3() { :; }
                            psql_admin() { echo retained; }
                            ops() {
                                if [[ "$PHASE" == bootstrap || -f "$WORK/operator-called" ]]; then
                                    blocked
                                fi
                                : > "$WORK/operator-called"
                                echo character_bootstrap=committed
                            }
                        ''' + function('assert_operator_replay') + '\nbootstrap_fixture() {\n' + bootstrap + '}\n'
                        invocation = 'bootstrap_fixture'
                    script = setup + '\n'.join(map(function, functions)) + '\n' + helper
                    script += "\ntrap 'interrupted TERM 143' TERM\n" + invocation
                    with subprocess.Popen(['bash', '-c', script], env=dict(os.environ, WORK=temp, PHASE=phase),
                                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                          text=True, start_new_session=True) as proc:
                        try:
                            deadline = time.monotonic() + 4
                            received = b''
                            while b'blocked\n' not in received:
                                self.assertTrue(select.select([proc.stdout], [], [], max(0, deadline - time.monotonic()))[0])
                                chunk = os.read(proc.stdout.fileno(), 4096)
                                self.assertTrue(chunk, received)
                                received += chunk
                            proc.send_signal(signal.SIGTERM)
                            output, _ = proc.communicate(timeout=1.5)
                            self.assertEqual(proc.returncode, 143, output)
                        finally:
                            # Bash job control can put the worker and watchdog in
                            # separate groups within this fixture's private session.
                            groups = subprocess.run(
                                ['ps', '-s', str(proc.pid), '-o', 'pgid='],
                                capture_output=True, text=True, check=False, timeout=3,
                            ).stdout.split()
                            for group in set(map(int, groups)):
                                try:
                                    os.killpg(group, signal.SIGKILL)
                                except ProcessLookupError:
                                    pass

    def test_spell_timeout_closes_descendant_output(self):
        budget = next(line for line in RUN_TEXT.splitlines()
                      if '3600s bash "$GAME_SOURCE/tools/qualification/spells/run.sh"' in line)
        budget = budget.strip().split('bash "$GAME_SOURCE', 1)[0].replace('3600s', '0.2s')
        script = function('run_interruptible') + "\n" + budget + "bash -c 'sleep 4 & wait'"
        started = time.monotonic()
        self.assertEqual(bash(script).returncode, 124)
        self.assertLess(time.monotonic() - started, 3)

    def test_base_identity_is_recorded_before_publication(self):
        result = bash(r"""
            set -eu
            BASE=$WORK/base
            BASE_PENDING=$WORK/pending
            sudo() { "$@"; }
            timeout() { shift 3; "$@"; }
            stat() {
                [[ ! -e "$BASE" || -s "$WORK/base.identity" ]] || return 23
                command stat "$@"
            }
        """ + function('acquire_base') + '\nacquire_base\ntest -d "$BASE"', self.base_fixture())
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_container_created_without_cid_is_removed_by_ownership(self):
        with tempfile.TemporaryDirectory() as temp:
            docker = Path(temp, 'docker')
            docker.write_text(r"""#!/bin/bash
if [[ "$1 $2" == 'container ls' &&
      "$*" == *'name=^/fixture-postgres$'* &&
      "$*" == *'label=org.oteryn.node-boot.owner=0123456789abcdef0123456789abcdef'* ]]; then
    echo owned-container-id
elif [[ "$1" == rm ]]; then
    echo "${@: -1}" > "$AUDIT"
fi
""")
            docker.chmod(0o755)
            setup = """
                set -Eeuo pipefail
                mkdir "$WORK"
                BASE=$WORK/base PG_CONTAINER=fixture-postgres WP5_PROJECT=fixture
                NODE_PID="" SERVICE_USER=fixture NODE_BOOT_WORLD=1 NODE_BOOT_SPELLS=0 ACTIVE_STAGE=setup
                COMPOSE_OWNER=0123456789abcdef0123456789abcdef
                PG_CONTAINER_ID="" result=NODE_BOOT_FAIL
                stop_runtime_diagnostics() { :; }
                stop_pg_sampler() { :; }
                evidence() { :; }
            """
            result = bash(setup + function('cleanup_owned_postgres') + '\n' + function('cleanup')
                          + '\ntrap cleanup EXIT\nexit 143', {
                              'PATH': f'{temp}:{os.environ["PATH"]}', 'WORK': f'{temp}/work',
                              'AUDIT': f'{temp}/removed',
                          })
            self.assertEqual(result.returncode, 143, result.stderr)
            self.assertTrue(Path(temp, 'removed').exists(), 'the ownership-labelled container leaked')
            self.assertEqual(Path(temp, 'removed').read_text().strip(), 'owned-container-id')

    def base_fixture(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        return {'WORK': temp.name}

    def test_basic_parent_waits_for_cleanup_to_finish(self):
        recursion = section('# A kept or reused stack is one topology', 'if [[ "$NODE_BOOT_SPELLS" == 1 ]];')
        self.assertIn('wait "$basic_pid"', recursion)
        self.assertNotIn('kill -KILL', recursion)

    def test_compose_cleanup_targets_owned_resources_only(self):
        cleanup = function('cleanup')
        self.assertIn('cleanup_owned_compose', cleanup)
        self.assertNotIn('compose down', cleanup)

    def test_compose_replacement_with_same_project_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            docker = Path(temp, 'docker')
            docker.write_text(r"""#!/bin/bash
case "$2" in
  ls)
    [[ "$*" != *"name="* ]] || exit 0
    if [[ "$*" == *"label=org.oteryn.node-boot.owner=$COMPOSE_OWNER"* ]]; then
      [[ "$1" != container ]] || echo retained-id
    else
      echo replacement-id
    fi
    ;;
  rm) printf '%s\n' "${@: -1}" >> "$AUDIT" ;;
esac
""")
            docker.chmod(0o755)
            script = r"""
                set -Eeuo pipefail
                WP5_PROJECT=fixture PG_CONTAINER=fixture SERVICE_USER=fixture NODE_PID=""
                export COMPOSE_OWNER=0123456789abcdef0123456789abcdef
                COMPOSE_OWNED=1 STACK_KEPT=0 ACTIVE_STAGE=setup NODE_BOOT_WORLD=1 NODE_BOOT_SPELLS=0
                result=NODE_BOOT_FAIL REUSE_STATE=""
                BASE=$WORK/base
                stop_runtime_diagnostics() { :; }
                stop_pg_sampler() { :; }
                evidence() { :; }
                sudo() { :; }
                compose() { echo replacement-id >> "$AUDIT"; }
            """
            result = bash(script + '\n'.join(map(function, ('cleanup_owned_compose', 'cleanup_owned_postgres', 'cleanup')))
                          + '\ntrap cleanup EXIT\nexit 1', {
                              'WORK': str(Path(temp, 'scratch')), 'AUDIT': str(Path(temp, 'removed')),
                              'PATH': f'{temp}:{os.environ["PATH"]}',
                          })
            self.assertEqual(result.returncode, 1)
            self.assertEqual(Path(temp, 'removed').read_text().splitlines(), ['retained-id'])


class StageDumpTests(unittest.TestCase):
    def run_stage(self, command, setup='', name='sample'):
        with tempfile.TemporaryDirectory() as temp:
            script = textwrap.dedent(f'''\
                set -Eeuo pipefail
                WORK={temp}; VARS={temp}/vars; : > "$VARS"; node_log={temp}/node.log
                failed_node_log={temp}/failed-node.log; PG_OBS={temp}/postgres
                GAME_SOURCE={HERE.parents[2]}; : > "$node_log"
                NODE_BOOT_WORLD=0; NODE_BOOT_SPELLS=0; ACTIVE_STAGE=setup
                RUNTIME_DIAGNOSTICS_PID=""; PG_SAMPLER_PID=4242; PG_CONTAINER=x
                declare -A STATUS=(); ORDER=()
                evidence() {{ :; }}; psql_admin() {{ :; }}; docker() {{ :; }}; compose() {{ :; }}
                durability_report_lines() {{ cat; }}; sleep() {{ :; }}
                start_runtime_diagnostics() {{ :; }}; stop_runtime_diagnostics() {{ :; }}
                pg_observe() {{ echo "PG_OBSERVE $*"; }}
                report_character_authority_failures() {{ :; }}
                ''') + function('pg_observability_dump') + '\n' + function('classify_stage_rows') + '\n' + function('stage') + (
                    f'\n{setup}\nstage {name} -- {command}\nprintf "status=%s\\n" "${{STATUS[{name}]}}"\n'
                    'if [[ -f $failed_node_log ]]; then cat "$failed_node_log"; fi\n'
                    f'if [[ -f "$WORK/failed-stage-{name}.log" ]]; then grep -Fx timeout_incarnation "$WORK/failed-stage-{name}.log" || true; fi\n')
            return bash(script).stdout

    def test_failed_evidence_writer_fails_the_stage_with_zero_command_exit(self):
        self.assertIn('status=FAIL', self.run_stage('true', 'tee() { cat > /dev/null; return 1; }'))

    def test_evidence_pipeline_failure_cannot_publish_pass(self):
        self.assertIn('status=FAIL(evidence)', self.run_stage('true', 'sort() { cat; return 2; }'))

    def test_stage_attempt_ignores_previous_timeout_and_status(self):
        with self.subTest(stale="timeout"):
            self.assertIn('status=PASS', self.run_stage('true', ': > "$WORK/stage-sample.deadline"'))
        with self.subTest(stale="status"):
            setup = r'''printf "0\nPASS\n" > "$WORK/stage-sample.status"
                kill_worker() { kill -KILL -- "$(ps -o pgid= -p "$BASHPID" | tr -d " ")"; }
            '''
            self.assertIn('status=FAIL(evidence)', self.run_stage('kill_worker', setup))

    def test_stage_deadline_fails_a_stalled_command(self):
        setup = 'NODE_BOOT_STAGE_TIMEOUT_SECONDS=1; stall() { command sleep 5; }'
        started = time.monotonic()
        self.assertIn('status=FAIL(exit 124)', self.run_stage('stall', setup))
        self.assertLess(time.monotonic() - started, 4)

    def test_timeout_snapshots_its_own_incarnation_after_an_earlier_failure(self):
        setup = ('NODE_BOOT_STAGE_TIMEOUT_SECONDS=1; echo earlier > "$failed_node_log"; '
                 'echo timeout_incarnation > "$node_log"; stall() { command sleep 4; }')
        lines = self.run_stage('stall', setup).splitlines()
        self.assertIn('earlier', lines)
        self.assertIn('timeout_incarnation', lines)

    def test_stage_deadline_includes_pg_dump(self):
        setup = "NODE_BOOT_STAGE_TIMEOUT_SECONDS=1; pg_observe() { trap '' TERM; command sleep 4; }"
        started = time.monotonic()
        self.assertIn('status=FAIL(exit 124)', self.run_stage('true', setup))
        self.assertLess(time.monotonic() - started, 3)

    def test_zero_selected_libtest_cases_fail_both_qualification_stages(self):
        setup = '''
            cargo_stub() {
                printf 'running 0 tests\\ntest result: ok. 0 passed; 0 failed; 0 ignored;\\n'
            }
        '''
        for name in ('seam_stages', 'spell_scenarios'):
            with self.subTest(name=name):
                self.assertIn('status=FAIL', self.run_stage('cargo_stub', setup, name))

    def test_failure_diagnostics_allow_stage_processing_to_continue(self):
        output = self.run_stage('false', 'sort() { cat; return 141; }')
        self.assertIn('status=FAIL(exit 1)', output)

    def test_failure_snapshot_uses_the_failing_stage_node_and_preserves_row_failures(self):
        setup = '''
            echo primary_snapshot > "$node_log"
            echo pvp_snapshot > "$WORK/pvp-node.log"
            row_fail() { echo 'SEAM_EVIDENCE e2e_all scenario=new.full status=FAIL seconds=1'; }
        '''
        pvp = self.run_stage('false', setup, 'pvp_world_ready')
        self.assertEqual(pvp.splitlines()[-1], 'pvp_snapshot')
        primary = self.run_stage('row_fail', setup)
        self.assertEqual(primary.splitlines()[-1], 'primary_snapshot')

    def test_operator_setup_rejects_unrelated_refusal_or_changed_replay(self):
        setup = r'''
            set -Eeuo pipefail
            INTERPRETATION=(profile ruleset content starter)
            write_node_config() { :; }
            psql_admin() { cat >/dev/null; echo same_durable_snapshot; }
            evidence() { echo "$*"; }
            fail() { echo "$*"; return 1; }
            ops() {
                case "$1/$2" in
                    character/interpretation)
                        if [[ ! -f "$BASE/first" ]]; then
                            echo "$REFUSAL" >&2
                            return 6
                        fi ;;
                    character/fresh-store)
                        if [[ -f "$BASE/first" ]]; then echo "$REPLAY"
                        else touch "$BASE/first"; echo 'character_recovery=admitted generation=1'
                        fi ;;
                esac
            }
        '''
        scripts = '\n'.join(function(name) for name in ('assert_operator_replay', 'operator_setup'))
        for refusal, replay in (
            ('interpretation refused until generation 1 is admitted (Durability(Unavailable))',
             'character_recovery=admitted generation=1'),
            ('interpretation refused until generation 1 is admitted (Rejected)',
             'character_recovery=admitted generation=2'),
        ):
            with self.subTest(refusal=refusal, replay=replay), tempfile.TemporaryDirectory() as temp:
                result = bash(setup + scripts + '\noperator_setup', {
                    'BASE': temp, 'REFUSAL': refusal, 'REPLAY': replay,
                })
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_shutdown_rejects_a_failed_exit_after_the_complete_marker(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'node.log').write_text('event=readiness now ready=false\nevent=shutdown now state=complete\n')
            (root / 'exit').write_text('23\n')
            script = '''
                set -Eeuo pipefail
                sudo() { [[ $2 != -0 ]]; }
                psql_admin() { cat >/dev/null; echo t; }
                evidence() { echo "$*"; }
                fail() { echo "$*"; return 1; }
                NODE_PID=4242
                NODE_PROCESS_DIR=$BASE
                node_log=$BASE/node.log
            '''
            functions = ('signal_node_process', 'stop_node',
                         'assert_shutdown_guards', 'graceful_shutdown')
            result = bash(script + '\n'.join(map(function, functions)) + '\ngraceful_shutdown', {'BASE': temp})
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn('exit=23', result.stdout)

    def test_detached_supervisor_signals_the_server_and_retains_its_exit(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'bin').mkdir()
            sudo = root / 'bin/sudo'
            # This supervisor does not forward signals to its child.
            sudo.write_text('#!/bin/bash\nif [[ $1 == -u ]]; then shift 2; "$@"; else exec "$@"; fi\n')
            sudo.chmod(0o755)
            server = root / 'bin/oteryn-game-server'
            server.write_text(f'''#!{sys.executable}
import signal
import sys
signal.signal(signal.SIGTERM, lambda *_: sys.exit(23))
signal.signal(signal.SIGALRM, lambda *_: sys.exit(99))
signal.alarm(6)
print("server-ready", file=sys.stderr, flush=True)
signal.pause()
''')
            server.chmod(0o755)
            setup = '''
                set -Eeuo pipefail
                PATH=$BASE/bin:$PATH; WORK=$BASE; SERVICE_USER=test; NODE_GAMEPLAY_ENV=()
                fail() { echo "$*"; return 1; }
                sleep() { command sleep 0.05; }
            '''
            functions = ('signal_node_process', 'start_node_process', 'stop_node')
            script = setup + '\n'.join(map(function, functions)) + '''
                (start_node_process "$BASE/node.log" "$BASE/config"; printf '%s' "$NODE_PROCESS_DIR" > "$BASE/node-dir")
                for i in {1..50}; do
                    if grep -q server-ready "$BASE/node.log"; then break; fi
                    sleep 0.05
                done
                stop_node "$(<"$BASE/node-dir")"
            '''
            result = bash(script, {'BASE': temp})
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn('exit=23', result.stdout)

    def test_replacement_and_pvp_shutdown_check_their_durable_readiness_guards(self):
        for stage, database in (('superseding_replacement', 'oteryn_node_boot'),
                                ('pvp_world_shutdown', '"$PVP_DATABASE"')):
            with self.subTest(stage=stage):
                self.assertIn(f'assert_shutdown_guards {database}', function(stage))
        with tempfile.TemporaryDirectory() as temp:
            Path(temp, 'pvp-node.log').write_text('event=shutdown now state=complete\n')
            setup = '''
                set -Eeuo pipefail
                WORK=$BASE; PVP_BASE=$BASE/pvp; PVP_PROCESS_DIR=$BASE/process; PVP_DATABASE=pvp
                stop_node() { :; }
                psql_admin() { cat >/dev/null; printf 'database=%s\\n' "$1" >&2; echo f; }
                fail() { echo "$*"; return 1; }
                evidence() { echo "$*"; }
            '''
            result = bash(setup + function('assert_shutdown_guards') + '\n' + function('pvp_world_shutdown')
                          + '\npvp_world_shutdown', {'BASE': temp})
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn('database=pvp', result.stderr)
            self.assertIn('guard still ready', result.stdout)

    def test_control_probe_rejects_an_unattributed_eof_and_a_stalled_peer(self):
        for outcome in ('eof', 'stalled', 'uid_refused'):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as temp, socket.socket(socket.AF_UNIX) as listener:
                root = Path(temp)
                (root / 'run').mkdir()
                (root / 'node.log').touch()
                listener.bind(str(root / 'run/control.sock'))
                listener.listen()
                script = '''
                    set -Eeuo pipefail
                    SERVICE_USER=fixture; SERVICE_UID=$(id -u)
                    node_log=$BASE/node.log
                    sudo() { shift 2; "$@"; }
                    evidence() { echo "$*"; }
                    fail() { echo "$*"; return 1; }
                ''' + function('control_socket_peer') + '\ncontrol_socket_peer'
                proc = subprocess.Popen(['bash', '-c', script], env=dict(os.environ, BASE=temp),
                                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                try:
                    listener.settimeout(5)
                    with listener.accept()[0] as peer:
                        if outcome == 'uid_refused':
                            pid, uid, _ = struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                            (root / 'node.log').write_text(
                                f'event=control_peer_rejected detail="peer_pid={pid} peer_uid={uid}"\n')
                        if outcome != 'stalled':
                            peer.shutdown(socket.SHUT_WR)
                        output, _ = proc.communicate(timeout=5)
                    self.assertEqual(proc.returncode == 0, outcome == 'uid_refused', output)
                    self.assertEqual('service_user_peer=closed' in output, outcome == 'uid_refused', output)
                finally:
                    if proc.poll() is None:
                        proc.kill()
                        proc.communicate()

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

    def test_sampling_and_statement_reset_are_scoped_to_both_world_databases(self):
        self.assertIn("a.datname IN (:'db', :'other_db')", self.pg.ACTIVITY_SQL)
        self.assertIn("d.datname IN (:'db', :'other_db')", self.pg.STATEMENTS_SQL)
        self.assertIn("pg_stat_statements_reset(0, d.oid, 0)", self.pg.STATEMENTS_SQL)
        self.assertNotIn("pg_stat_statements_reset()", self.pg.STATEMENTS_SQL)
        self.assertIn("p.dbid = a.datid", self.pg.ACTIVITY_SQL)

    def test_failed_log_read_preserves_the_cursor_and_reports_unavailable(self):
        with tempfile.TemporaryDirectory() as temp:
            cursor = Path(temp, '.server-log-lines')
            cursor.write_text('8')
            args = type('Args', (), dict(dir=temp, stage='seam', container='c', user='u',
                                         database='db', other_database='', activity=None, verbose=False))()
            for failure in (subprocess.CompletedProcess([], 1, '', 'daemon unavailable'),
                            subprocess.TimeoutExpired('docker logs', 30)):
                with self.subTest(failure=failure), mock.patch.object(self.pg, 'psql', return_value=[]):
                    with mock.patch.object(self.pg.subprocess, 'run') as command:
                        if isinstance(failure, Exception):
                            command.side_effect = failure
                        else:
                            command.return_value = failure
                        with mock.patch('builtins.print') as output:
                            self.pg.dump(args)
                        self.assertIn('server_logs=unavailable', str(output.call_args_list))
                        self.assertNotIn('slow_statements=0', str(output.call_args_list))
                        self.assertEqual(cursor.read_text(), '8')

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
        self.assertIn('run_interruptible psql_admin postgres <<< "CREATE EXTENSION pg_stat_statements"', RUN_TEXT)
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
