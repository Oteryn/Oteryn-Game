"""Composition guards for indexed rows and their real-capture consumers."""
from pathlib import Path
import re
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
        wrapped = list(re.finditer(
            r'verdict::indexed\(\s*"([A-Z]\d\d[a-z]?\.(?:full|client))",\s*async \{\s*row_clients!\(mode, clients, "\1"\);\s*((?:full|client)_row_\w+)::run_packet',
            source,
        ))
        self.assertEqual(calls, {match[2] for match in wrapped})
        chest_ids = re.findall(r'chest\("(Q0[13][a-z])",', (PHASES / 'full_row_q01.rs').read_text())
        self.assertEqual(len(chest_ids), 16)
        dispatched_chests = []
        for match in wrapped:
            scenario, module = match.groups()
            kind, _, row = module.partition('_row_')
            expected = row[0].upper() + row[1:]
            if module == 'full_row_q01':
                index = re.match(r'\(clients, &full_row_q01::CHESTS\[(\d+)\]\)', source[match.end():])
                self.assertIsNotNone(index)
                expected = chest_ids[int(index[1])]
                dispatched_chests.append(expected)
            self.assertEqual(scenario, expected + '.' + kind)
        self.assertCountEqual(dispatched_chests, chest_ids)

    def test_row_timeouts_observe_typed_errors_before_reporter_formatting(self):
        for path in PHASES.glob('*_row_*.rs'):
            body = packet_body(path.read_text())
            if body is not None:
                with self.subTest(path=path.name):
                    self.assertNotIn('tokio::time::timeout(', body)
        shared = (PHASES / 'client_row_s13c.rs').read_text()
        self.assertIn('indexed_timeout(Duration::from_secs(420), flow(clients, checks))', shared)
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

    def test_real_c16_captures_do_not_write_fabricated_ids_or_repeat_full_packets(self):
        source = (PHASES / 'client_row_c16c.rs').read_text()
        self.assertNotIn('status=MISSING-CLIENT', source)
        self.assertIn('full_e2ef5::client_companions', source)
        self.assertRegex(source, r'entities\s*\.get\(&companion\)')
        self.assertIn('entities.get(&corpse).is_some()', source)
        self.assertRegex(source, r'observed\s*\.entity\s*\.identity')
        self.assertIn('observed.entity.generation', source)
        journey = (PHASES / 'full_e2ef5.rs').read_text()
        self.assertIn('client_row_c16c::capture(client, "convince", rat, None)', journey)
        self.assertIn('client_row_c16c::capture(client, "animate", skeleton, Some(corpse.entity))', journey)
        tile_capture = (PHASES / 'client_row_c16d.rs').read_text()
        self.assertIn('full_e2ef5::client_tile_items', tile_capture)
        self.assertIn('client_row_c16d::capture(client)', journey)
        for name in ['client_row_c16c', 'client_row_c16d']:
            self.assertNotIn('full_e2ef5::run(', (PHASES / (name + '.rs')).read_text())


if __name__ == '__main__':
    unittest.main()
