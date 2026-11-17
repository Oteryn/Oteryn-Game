"""Bounded routing/receipt tests; synthetic unit fixtures are never replay evidence."""
import copy
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import pg_client_capture_replays as replay


HEAD = 'a' * 40


def listing(rows):
    suites = {}
    for binary, name in rows:
        suite = suites.setdefault(binary, {'binary-id': binary, 'testcases': {}})
        suite['testcases'][name] = {'filter-match': {'status': 'matches'}}
    return {'rust-suites': suites}


def fixture(case):
    admitted = {'game_session_id': '11' * 16, 'world_id': '22' * 16, 'channel_id': '33' * 16,
                'connection_generation': 1, 'current_server_sequence': 0,
                'next_command_id': 1, 'schema_revision': 1, 'selected_capabilities': [11, 18, 30]}
    value = {'schema': case.schema, 'character_id': '44' * 16}
    if case.name == 'soulpit-exit':
        for key in ('entry', 'exit'):
            value[key] = {'admission': copy.deepcopy(admitted), 'initial_frames': ['abcd'],
                          'subsequent_frames': ['abcd'], 'command': {'payload': 'abcd'}}
    else:
        for key in ('source_admission', 'update_admission', 'before_return_admission', 'returned_admission'):
            value[key] = copy.deepcopy(admitted)
        for key in ('initial_frames', 'entry_frames', 'update_frames', 'return_frames'):
            value[key] = ['abcd']
        value['entry_command'] = {'payload': 'abcd'}
        if case.name == 'soulpit':
            value.update(autonomous_return=True, return_command=None,
                         wave_stages=[{'wave': n, 'admission': copy.deepcopy(admitted), 'frames': ['abcd']}
                                      for n in range(1, 5)])
        else:
            value.update(native_server_artifact_digest='55' * 32, return_command={'payload': 'abcd'})
    return value


def archive(root):
    root.mkdir()
    (root / 'head-sha').write_text(HEAD + '\n')
    (root / 'client-replay.tar.zst').write_bytes(b'unit archive fixture, not an executable archive')
    (root / 'SHA256SUMS').write_text(''.join(
        f'{replay.digest(root / name)}  {name}\n' for name in ('head-sha', 'client-replay.tar.zst')))
    return replay.digest(root / 'SHA256SUMS')


class RoutingTests(unittest.TestCase):
    def test_all_24_partitions_keep_every_original_test_exactly_once(self):
        producers = [(replay.SERVER, 'actual::module::' + case.producer) for case in replay.CASES]
        ordinary = [(replay.SERVER, f'other::case_{n}') for n in range(100)]
        rows = producers + ordinary
        observed, captures = [], []
        for index in range(24):
            assigned = rows[index::24]
            normal, selected = replay.partition([replay.test_filter(*row) for row in assigned], listing(rows))
            observed += normal + [replay.test_filter(replay.SERVER, case.producer) for case in selected]
            captures += [case.name for case in selected]
        self.assertCountEqual(observed, [replay.test_filter(*row) for row in rows])
        self.assertEqual(len(observed), len(set(observed)))
        self.assertCountEqual(captures, [case.name for case in replay.CASES])

    def test_absent_dispatch_scope_preserves_the_ordinary_route(self):
        row = ('oteryn-gameplay-durability', 'test::postgres_case')
        filters = [replay.test_filter(*row)]
        self.assertEqual(replay.partition(filters, listing([row])), (filters, []))
        self.assertEqual(replay.partition([], listing([row])), ([], []))

    def test_missing_ambiguous_or_unassigned_identity_cannot_silently_skip(self):
        rows = [(replay.SERVER, 'real::' + case.producer) for case in replay.CASES]
        for changed in (rows[:-1], rows + [(replay.SERVER, 'wrong::' + replay.CASES[0].producer)]):
            with self.subTest(rows=changed), self.assertRaisesRegex(ValueError, 'exactly one'):
                replay.partition([], listing(changed))
        line = replay.test_filter(*rows[0])
        for filters in ([line, line], ['--filterset=test(postgres)'], [replay.test_filter(replay.SERVER, 'absent')]):
            with self.subTest(filters=filters), self.assertRaises(ValueError):
                replay.partition(filters, listing(rows))

    def test_checked_workflow_supplies_archive_and_runs_the_capture_owner(self):
        path = Path(os.environ.get('OTERYN_PG_REPLAY_TEST_WORKFLOW',
                    str(Path(__file__).resolve().parents[2] / '.github/workflows/pr-postgres-harness.yml')))
        text = path.read_text()
        self.assertTrue('nextest archive --locked -p oteryn-client --lib' in text, "missing checked-in route: " + 'nextest archive --locked -p oteryn-client --lib')
        self.assertTrue('pg_client_capture_replays.py plan' in text, "missing checked-in route: " + 'pg_client_capture_replays.py plan')
        self.assertTrue('pg_client_capture_replays.py run' in text, "missing checked-in route: " + 'pg_client_capture_replays.py run')
        self.assertTrue('needs.build.outputs.client_replay_sha256' in text, "missing checked-in route: " + 'needs.build.outputs.client_replay_sha256')
        self.assertTrue('needs.build.outputs.lib_sha256' in text, "missing checked-in route: " + 'needs.build.outputs.lib_sha256')
        self.assertTrue('if-no-files-found: error' in text, "missing checked-in route: " + 'if-no-files-found: error')
        self.assertIn('partition: [' + ', '.join(map(str, range(1, 25))) + ']', text)

    def test_ordinary_failure_still_runs_every_assigned_producer_and_fails_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rows = [(replay.SERVER, 'real::' + case.producer) for case in replay.CASES]
            rows.append((replay.SERVER, 'ordinary::postgres_case'))
            (root / 'listing.json').write_text(json.dumps(listing(rows)))
            (root / 'filters').write_text('\n'.join(replay.test_filter(*row) for row in rows) + '\n')
            (root / 'pg.tar.zst').write_bytes(b'unit PG archive fixture')
            binding = archive(root / 'client')
            arguments = ['capture-replays', 'run', '--filters', str(root / 'filters'),
                         '--listing', str(root / 'listing.json'), '--archive', str(root / 'pg.tar.zst'),
                         '--priority', str(root / 'priority.toml'), '--client-directory', str(root / 'client'),
                         '--client-manifest', binding, '--pg-manifest', 'b' * 64, '--head', HEAD,
                         '--partition', '1', '--evidence', str(root / 'evidence')]
            consumers = listing([(replay.CLIENT, case.consumer) for case in replay.CASES])
            def readback(command, **kwargs):
                return HEAD + '\n' if kwargs.get('text') else json.dumps(consumers).encode()
            with patch.dict(os.environ, {'PG_RUN': '/runner', 'PG_WORKSPACE': '/workspace',
                            'GITHUB_RUN_ID': '1', 'GITHUB_RUN_ATTEMPT': '1'}), \
                    patch('sys.argv', arguments), \
                    patch.object(replay.subprocess, 'run', return_value=SimpleNamespace(returncode=100)) as ordinary, \
                    patch.object(replay.subprocess, 'check_output', side_effect=readback), \
                    patch.object(replay, 'replay', side_effect=lambda case, *args: {'name': case.name, 'state': 'PASS'}) as actual:
                self.assertEqual(replay.main(), 100)
            self.assertEqual(actual.call_count, 3)
            self.assertEqual([call.args[0].name for call in actual.call_args_list], [case.name for case in replay.CASES])
            command = ordinary.call_args.args[0]
            self.assertEqual([arg for arg in command if arg.startswith('--filterset=')],
                             [replay.test_filter(*rows[-1])])
            receipt = json.loads((root / 'evidence/evidence.json').read_bytes())
            self.assertEqual(receipt['state'], 'FAILED')
            self.assertEqual(receipt['ordinary_exit'], 100)
            self.assertEqual(len(receipt['results']), 3)


class ReceiptTests(unittest.TestCase):
    def test_real_capture_shape_requires_frames_admission_and_nonzero_primal_digest(self):
        for case in replay.CASES:
            value = fixture(case)
            self.assertEqual(replay.validate_capture(json.dumps(value).encode(), case), value)
            changed = copy.deepcopy(value)
            changed['schema'] = 'WRONG'
            with self.assertRaisesRegex(ValueError, 'schema'):
                replay.validate_capture(json.dumps(changed).encode(), case)
        primal = fixture(replay.CASES[2])
        for key, value in [('native_server_artifact_digest', '0' * 64), ('initial_frames', []),
                           ('character_id', '00'), ('source_admission', {})]:
            changed = copy.deepcopy(primal)
            changed[key] = value
            with self.subTest(key=key), self.assertRaises((ValueError, KeyError)):
                replay.validate_capture(json.dumps(changed).encode(), replay.CASES[2])
        soulpit = fixture(replay.CASES[0])
        soulpit['wave_stages'].pop()
        with self.assertRaisesRegex(ValueError, 'four'):
            replay.validate_capture(json.dumps(soulpit).encode(), replay.CASES[0])

    def test_archive_is_bound_to_exact_head_manifest_and_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'archive'
            binding = archive(root)
            self.assertEqual(replay.verify_client_archive(root, HEAD, binding), root / 'client-replay.tar.zst')
            for head, manifest in [('b' * 40, binding), (HEAD, '0' * 64)]:
                with self.assertRaisesRegex(ValueError, 'binding'):
                    replay.verify_client_archive(root, head, manifest)
            (root / 'client-replay.tar.zst').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                replay.verify_client_archive(root, HEAD, binding)

    def run_case(self, case, *, producer_marker=True, consumer_marker=True, missing=False,
                 producer_exit=0, consumer_exit=0, zero_digest=False, mutate=False):
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory)
            args = SimpleNamespace(head=HEAD, runner='/runner/pg-run', workspace='/build/exact-head',
                                   archive=Path('/archive/pg-lib.tar.zst'), priority=Path('/plan/priority.toml'))
            calls, captures = [], []
            def unit_runner(command, log):
                calls.append(command)
                value = next(arg for arg in command if arg.startswith(case.variable + '='))
                capture = Path(value.split('=', 1)[1])
                captures.append(str(capture))
                self.assertTrue(capture.is_relative_to('/tmp'))
                for other in replay.CASES:
                    self.assertIn(other.variable, command)
                self.assertEqual(sum(arg.startswith('SRX3_') and '=' in arg for arg in command), 1)
                if len(calls) == 1:
                    self.assertFalse(capture.exists())
                    if not missing:
                        value = fixture(case)
                        if zero_digest:
                            value['native_server_artifact_digest'] = '0' * 64
                        capture.write_bytes(json.dumps(value).encode())
                    log.write_text(case.producer_marker if producer_marker else 'test result: ok (no execution marker)')
                    return producer_exit
                self.assertIn('OTERYN_ASSET_DIR=' + args.workspace, command)
                self.assertIn(replay.test_filter(replay.CLIENT, case.consumer), command)
                if mutate:
                    capture.write_text('{}')
                log.write_text(case.consumer_marker if consumer_marker else 'SRX3_NATIVE_CAPTURE_SKIPPED')
                return consumer_exit
            result = replay.replay(case, args, Path('/archive/client-replay.tar.zst'),
                                   {(replay.CLIENT, case.consumer)}, evidence, run=unit_runner)
            if result['state'] == 'PASS':
                self.assertEqual(result['capture_sha256'], replay.digest(evidence / (case.name + '.json')))
                self.assertEqual(captures[0], captures[1])
            return result, calls, captures

    def test_each_producer_precedes_only_its_consumer_with_fresh_real_path(self):
        paths = []
        for case in replay.CASES:
            result, calls, captures = self.run_case(case)
            self.assertEqual(result['state'], 'PASS')
            self.assertEqual(len(calls), 2)
            self.assertIn(replay.test_filter(replay.SERVER, case.producer), calls[0])
            paths.append(captures[0])
        self.assertEqual(len(paths), len(set(paths)))

    def test_skips_missing_capture_and_failures_never_count_as_replay(self):
        for options, count in [({'producer_marker': False}, 1), ({'missing': True}, 1),
                               ({'producer_exit': 100}, 1), ({'zero_digest': True}, 1),
                               ({'consumer_marker': False}, 2), ({'consumer_exit': 100}, 2),
                               ({'mutate': True}, 2)]:
            with self.subTest(options=options):
                result, calls, _ = self.run_case(replay.CASES[2], **options)
                self.assertEqual(result['state'], 'FAILED')
                self.assertEqual(len(calls), count)


if __name__ == '__main__':
    unittest.main()
