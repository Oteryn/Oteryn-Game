"""Composition guards for indexed rows and their real-capture consumers."""
from pathlib import Path
import os
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
PHASES = ROOT / 'apps/game-server/src/qualification/client_phases'


def packet_body(source):
    match = re.search(r'pub\(super\) async fn run_packet\b', source)
    if not match:
        return None
    start = source.index('{', match.start())
    depth, end = 1, start + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]


class IndexedVerdictTests(unittest.TestCase):
    def test_shared_result_adapter_preserves_concrete_errors_and_elapsed(self):
        source = (PHASES / 'indexed_result.rs').read_text()
        self.assertIn('Future<Output = Result<T, E>>', source)
        self.assertIn('Result<Result<T, E>, tokio::time::error::Elapsed>', source)
        self.assertNotIn('Future<Output = TestResult<T>>', source)
        self.assertIn('source: Box<dyn Error>', source)
        self.assertIn('Some(self.source.as_ref())', source)

    def test_remembered_failure_has_a_verdict_even_when_reporter_emits_only_gap(self):
        source = (PHASES / 'indexed_verdict.rs').read_text()
        self.assertIn('state.stall.is_some() && state.pending.is_empty()', source)
        self.assertIn('watch.ensure_failure_verdict();', source)
        self.assertIn('observed-error-without-verdict', source)

    def test_expired_wait_is_excluded_from_next_phase_idle_time(self):
        source = (PHASES / 'verdict.rs').read_text()
        self.assertRegex(source, r'idle_until\s*\.map_or\(last, \|until\| last\.max\(until\)\)\s*\.elapsed\(\)')

    def test_every_indexed_dispatch_enters_its_scenario_watch(self):
        source = (PHASES / 'world.rs').read_text()
        calls = set(re.findall(r'((?:full|client)_row_\w+)::run_packet', source))
        wrapped = dict(re.findall(
            r'verdict::indexed\(\s*"([A-Z]\d\d[a-z]?\.(?:full|client))",\s*async \{\s*((?:full|client)_row_\w+)::run_packet', source
        ))
        self.assertEqual(calls, set(wrapped.values()))
        for scenario, module in wrapped.items():
            kind, _, row = module.partition('_row_')
            self.assertEqual(scenario, row[0].upper() + row[1:] + '.' + kind)

    def test_row_timeouts_observe_typed_errors_before_reporter_formatting(self):
        for path in PHASES.glob('*_row_*.rs'):
            body = packet_body(path.read_text())
            if body is not None:
                with self.subTest(path=path.name):
                    self.assertNotIn('tokio::time::timeout(', body)
        shared = (PHASES / 'client_row_s13c.rs').read_text()
        self.assertIn('indexed_timeout(Duration::from_secs(420), flow(clients))', shared)
        self.assertIn('shared_failure("S13-party-observation")', shared)

    def test_offline_recovery_uses_its_780_second_bound_in_both_rows(self):
        for name in ['full_row_c15', 'client_row_c15']:
            source = (PHASES / (name + '.rs')).read_text()
            with self.subTest(row=name):
                self.assertRegex(source, r'bounded_wait\(\s*tokio::time::Instant::now\(\)\s*\+\s*Duration::from_secs\(780\),\s*"offline-recovery"')
                self.assertIn('sleep(Duration::from_secs(780))', source)

    def test_runner_selects_all_capture_consuming_packages(self):
        source = (ROOT / 'tools/qualification/e2e_all/run.sh').read_text()
        for package in ['oteryn-client-runtime', 'oteryn-client-domain', 'oteryn-client-ui',
                        'oteryn-client', 'oteryn-session', 'oteryn-dev-client']:
            with self.subTest(package=package):
                self.assertRegex(source, r'-p ' + package + r'(?:\s|$)')
        self.assertIn('export E2E_ALL_CAPTURE_DIR=', source)
        self.assertIn('client_status=$?', source)

    def test_capture_replays_require_their_own_writer_in_this_run(self):
        source = (ROOT / 'tools/qualification/e2e_all/run.sh').read_text()
        suite = source.split('replay_suite() {', 1)[1].split('\n}\n', 1)[0]
        c16d = 'tests::e2e_c16d::e2e_c16d_item_appearance_snapshot_replay'
        unit = 'tests::e2e_c16d::e2e_c16d_capture_round_trip_with_negotiated_entity_fields'
        c16c = 'tests::e2e_c16c_replay_tests::e2e_c16c_summon_delta_replay'
        c01 = 'tests::e2e_c01::e2e_c01_replays_server_combat_state_and_rejects_invalid_delta'
        for writer, encoding, selected in [
            ('C01.client', 'e2e_capture', c01),
            ('C16c.client', 'e2e_all', c16c),
            ('C16d.client', 'e2e_all', c16d),
        ]:
            with self.subTest(writer=writer), tempfile.TemporaryDirectory() as tmp:
                log = Path(tmp) / 'node.log'
                status = 'CAPTURED' if encoding == 'e2e_capture' else 'PASS'
                log.write_text(f'SEAM_EVIDENCE {encoding} scenario={writer} status={status} seconds=1.0\n')
                result = subprocess.run(
                    ['bash', '-c', """
                        set -uo pipefail
                        RUN="$1"
                        CLIENT_CAPTURES="$1"
                        ROOT="$2"
                        export NODE_BOOT_E2E_ONLY=f6
                        CARGO=mock_cargo
                        mock_cargo() {
                            case "$*" in
                                *--list*--ignored*) ;;
                                *--list*) printf '%s: test\n' "$c16d" "$unit" "$c16c" "$c01" ;;
                                *) for test in "${@:8}"; do
                                       [[ "$test" == --exact ]] || printf 'test %s ... ok\n' "$test"
                                   done ;;
                            esac
                        }
                        """ + f"replay_suite() {{{suite}\n}}\n" +
                     f'c16d={c16d}; unit={unit}; c16c={c16c}; c01={c01}\n' +
                     'replay_suite -p oteryn-session', 'test', tmp, str(ROOT)],
                    capture_output=True, text=True, check=False, timeout=10,
                    cwd=tmp,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(unit, result.stdout)
                for replay in (c01, c16c, c16d):
                    self.assertEqual(replay in result.stdout, replay == selected)

    def test_selected_capture_writers_must_report_success(self):
        selector = ROOT / 'tools/qualification/e2e_all/select_replays.py'
        world = (PHASES / 'world.rs').read_text()
        with tempfile.TemporaryDirectory() as tmp:
            log, listing, ignored = (Path(tmp) / name for name in ('node.log', 'listed', 'ignored'))
            ignored.touch()
            registry = (selector.with_name('capture_replays.txt')).read_text()
            for row, capture in {
                'C14.client': 'c14-skill-display.bin',
                'C18a.client': 'c18a-combat.bin',
                'M01.client': 'm01-autowalk.bin',
                'M07.client': 'm07-spatial.bin',
                'M11a.client': 'm11a-up-spatial.bin',
            }.items():
                with self.subTest(row=row):
                    index = re.search(r'e2e_row_selected\((\d+)\) \{\s*verdict::indexed\("' +
                                      re.escape(row) + '"', world)[1]
                    name = re.search(r'^' + re.escape(row) + r' (\S+)$', registry, re.M)[1]
                    listing.write_text(f'{name}: test\nunit_test: test\n')
                    (Path(tmp) / capture).touch()
                    log.write_text('')
                    env = {**os.environ, 'NODE_BOOT_E2E_ONLY': 'rows', 'OTERYN_E2E_ROW_BATCH': f'{index}:1'}

                    def invoke(*args):
                        return subprocess.run(
                            ['python3', str(selector), *args], capture_output=True,
                            text=True, check=False, env=env,
                        )

                    args = (str(log), tmp, str(ignored), str(listing))
                    result = invoke(*args)
                    self.assertNotEqual(result.returncode, 0, result.stdout)
                    self.assertIn(f'{row}: missing success verdict', result.stderr)
                    log.write_text(f'SEAM_EVIDENCE e2e_capture scenario={row} status=CAPTURED seconds=1.0\n')
                    result = invoke(*args)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(result.stdout.splitlines(), [name, 'unit_test'])
                    suite_log = Path(tmp) / 'suite.log'
                    suite_log.write_text('test unit_test ... ok\n')
                    result = invoke('--verify-passes', str(suite_log))
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(f'{row}: no passing capture replay', result.stderr)
                    suite_log.write_text(f'test {name} ... ok\n')
                    result = invoke('--verify-passes', str(suite_log))
                    self.assertEqual(result.returncode, 0, result.stderr)

    def test_real_c16_captures_do_not_write_fabricated_ids_or_repeat_full_packets(self):
        source = (PHASES / 'client_row_c16c.rs').read_text()
        self.assertIn('status=PASS evidence=animate-dead-replay-captured', source)
        self.assertIn('.c16c_capture_files(summon.entity)', source)
        self.assertIn('for (name, bytes) in files?', source)
        self.assertIn('std::fs::write(directory.join(name), bytes)?', source)
        for name in ['client_row_c16c', 'client_row_c16d']:
            self.assertNotIn('full_e2ef5::run(', (PHASES / (name + '.rs')).read_text())


if __name__ == '__main__':
    unittest.main()
