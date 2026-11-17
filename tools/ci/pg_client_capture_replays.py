#!/usr/bin/env python3
"""Run three assigned real PG transcripts through their exact client consumers."""
import argparse
from dataclasses import dataclass, replace
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


@dataclass(frozen=True)
class Replay:
    name: str
    producer: str
    consumer: str
    variable: str
    schema: str
    producer_marker: str
    consumer_marker: str


SERVER = 'oteryn-gameplay-dispatch'
CLIENT = 'oteryn-client'
ITEM = 'play::tests::feature_0015_item_use::activity_scope::actual_capture::'
BOSS = 'play::tests::feature_0117_boss_encounter::actual_capture::'
CASES = (
    Replay('soulpit', 'srx3_soulpit_native_use_scope_barrier_and_spawn_postgres',
           ITEM + 'srx3_soulpit_actual_server_transcript_native_use_waves_icons_and_return',
           'SRX3_SOULPIT_CAPTURE_PATH', 'OTERYN_NATIVE_SCOPE_REPLAY/v1',
           'SRX3_SOULPIT POSTGRES_CASE_COMPLETED native_use_scope_barrier_spawn fault_after_commit=false retained_grace=false',
           'SRX3_SOULPIT ACTUAL_SERVER_NATIVE_PLAY_COMPLETED'),
    Replay('soulpit-exit', 'srx3_soulpit_native_early_exit_retires_private_encounter_postgres',
           ITEM + 'early_exit::srx3_soulpit_actual_server_native_core_and_early_exit_gestures',
           'SRX3_SOULPIT_EXIT_CAPTURE_PATH', 'OTERYN_NATIVE_SOULPIT_EXIT_REPLAY/v1',
           'SRX3_SOULPIT NATIVE_EARLY_EXIT_COMPLETED',
           'SRX3_SOULPIT ACTUAL_SERVER_NATIVE_EARLY_EXIT_PLAY_COMPLETED'),
    Replay('primal', 'srx3_primal_real_source_entry_wave_pods_and_terminal_postgres',
           BOSS + 'srx3_primal_actual_server_transcript_native_entry_pod_and_return',
           'SRX3_PRIMAL_CAPTURE_PATH', 'OTERYN_NATIVE_SCOPE_REPLAY/v1',
           'SRX3_PRIMAL_PRODUCTION_POSTGRES_CASE_EXECUTED',
           'SRX3_PRIMAL ACTUAL_SERVER_NATIVE_PLAY_COMPLETED'),
)
FILTER = re.compile(r'--filterset=binary_id\(=([^\n)]+)\) & test\(=([^\n)]+)\)')


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def test_filter(binary, name):
    return f'--filterset=binary_id(={binary}) & test(={name})'


def listed_cases(listing):
    cases = [(suite['binary-id'], name)
             for suite in listing['rust-suites'].values()
             for name, case in suite['testcases'].items()
             if case['filter-match']['status'] == 'matches']
    if len(cases) != len(set(cases)):
        raise ValueError('duplicate listed test identity')
    return set(cases)


def partition(filters, listing):
    available = listed_cases(listing)
    resolved = []
    if any(binary == SERVER for binary, _ in available):
        for case in CASES:
            matches = [name for binary, name in available
                       if binary == SERVER and name.rsplit('::', 1)[-1] == case.producer]
            if len(matches) != 1:
                raise ValueError('dispatch archive needs exactly one real capture producer: ' + case.producer)
            resolved.append(replace(case, producer=matches[0]))
    parsed = []
    for line in filters:
        match = FILTER.fullmatch(line)
        if match is None or match.groups() not in available:
            raise ValueError('partition filter is not an exact listed test')
        parsed.append(match.groups())
    if len(parsed) != len(set(parsed)):
        raise ValueError('duplicate assigned test')
    selected = [case for case in resolved if (SERVER, case.producer) in parsed]
    special = {test_filter(SERVER, case.producer) for case in selected}
    ordinary = [line for line in filters if line not in special]
    if sorted(ordinary + list(special)) != sorted(filters):
        raise ValueError('capture routing changed the assigned PG test set')
    return ordinary, selected


def require_hex(value, size=None):
    if (not isinstance(value, str) or not value or len(value) % 2
            or re.fullmatch('[0-9a-f]+', value) is None
            or (size is not None and len(value) != size * 2)):
        raise ValueError('invalid captured hex value')


def admission(value):
    for key in ('game_session_id', 'world_id', 'channel_id'):
        require_hex(value[key], 16)
    for key in ('connection_generation', 'current_server_sequence', 'next_command_id', 'schema_revision'):
        number = value[key]
        if type(number) is not int or not 0 <= number < 2**64:
            raise ValueError('invalid captured admission counter')
    caps = value['selected_capabilities']
    if (not isinstance(caps, list) or not caps or len(caps) != len(set(caps))
            or any(type(cap) is not int or not 0 <= cap < 2**32 for cap in caps)):
        raise ValueError('invalid captured selected capabilities')


def frames(value):
    if not isinstance(value, list) or not value:
        raise ValueError('missing real captured frames')
    for frame in value:
        require_hex(frame)


def validate_capture(raw, case):
    if not 0 < len(raw) <= 32 * 1024 * 1024:
        raise ValueError('missing or oversized real capture')
    value = json.loads(raw)
    if value['schema'] != case.schema:
        raise ValueError('wrong real capture schema')
    require_hex(value['character_id'], 16)
    if case.name == 'soulpit-exit':
        for key in ('entry', 'exit'):
            epoch = value[key]
            admission(epoch['admission'])
            frames(epoch['initial_frames'])
            frames(epoch['subsequent_frames'])
            require_hex(epoch['command']['payload'])
    else:
        for key in ('source_admission', 'update_admission', 'before_return_admission', 'returned_admission'):
            admission(value[key])
        for key in ('initial_frames', 'entry_frames', 'update_frames', 'return_frames'):
            frames(value[key])
        require_hex(value['entry_command']['payload'])
        if case.name == 'soulpit':
            if value['autonomous_return'] is not True or value['return_command'] is not None:
                raise ValueError('Soulpit capture lost its autonomous return')
            if [stage['wave'] for stage in value['wave_stages']] != [1, 2, 3, 4]:
                raise ValueError('Soulpit capture must retain all four real wave stages')
            for stage in value['wave_stages']:
                admission(stage['admission'])
                frames(stage['frames'])
        else:
            source_digest = value['native_server_artifact_digest']
            require_hex(source_digest, 32)
            if source_digest == '0' * 64:
                raise ValueError('Primal native server artifact digest is zero')
            require_hex(value['return_command']['payload'])
    return value


def verify_client_archive(directory, head, expected_manifest):
    if re.fullmatch('[0-9a-f]{64}', expected_manifest) is None:
        raise ValueError('missing same-head client archive binding')
    names = {'head-sha', 'SHA256SUMS', 'client-replay.tar.zst'}
    if {path.name for path in directory.iterdir()} != names:
        raise ValueError('unexpected client archive inventory')
    if any(not (directory / name).is_file() or (directory / name).is_symlink() for name in names):
        raise ValueError('client archive entries must be regular files')
    if (digest(directory / 'SHA256SUMS') != expected_manifest
            or (directory / 'head-sha').read_text() != head + '\n'):
        raise ValueError('client archive head or manifest binding differs')
    expected = ''.join(f'{digest(directory / name)}  {name}\n'
                       for name in ('head-sha', 'client-replay.tar.zst'))
    if (directory / 'SHA256SUMS').read_text() != expected:
        raise ValueError('client archive checksum differs')
    return directory / 'client-replay.tar.zst'


def logged(command, path):
    with path.open('xb') as output, subprocess.Popen(
            command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as process:
        for chunk in iter(lambda: process.stdout.read1(65536), b''):
            output.write(chunk)
            sys.stdout.buffer.write(chunk)
            sys.stdout.buffer.flush()
        return process.wait()


def completed(code, path, marker):
    output = path.read_text(errors='replace')
    if code or marker not in output or 'SRX3_NATIVE_CAPTURE_SKIPPED' in output:
        raise ValueError('real execution did not complete: ' + marker)


def wrapped(runner, command, variables=()):
    # Set the capture only on its one producer/consumer, never sibling scenarios.
    return [runner, 'env', *(argument for case in CASES for argument in ('-u', case.variable)),
            *variables, *command]


def nextest(archive, workspace, filters, priority=None):
    command = ['cargo-nextest', 'nextest', 'run', '--archive-file', str(archive),
               '--workspace-remap', workspace, '--no-fail-fast']
    if priority:
        command += ['--tool-config-file', 'oteryn-pg-plan:' + str(priority)]
    return command + filters


def replay(case, args, client_archive, client_listing, evidence, run=logged):
    result = {'name': case.name, 'producer': case.producer, 'consumer': case.consumer,
              'state': 'FAILED'}
    producer_log, consumer_log = evidence / (case.name + '-pg.log'), evidence / (case.name + '-client.log')
    try:
        with tempfile.TemporaryDirectory(prefix=f'oteryn-{args.head[:12]}-{case.name}-', dir='/tmp') as scratch:
            capture = Path(scratch) / 'capture.json'
            variable = case.variable + '=' + str(capture)
            options = ['--success-output', 'immediate', '--failure-output', 'immediate', '--no-tests', 'fail', '--retries', '0']
            command = nextest(args.archive, args.workspace, [test_filter(SERVER, case.producer)], args.priority) + options
            result['producer_exit'] = run(wrapped(args.runner, command, [variable]), producer_log)
            completed(result['producer_exit'], producer_log, case.producer_marker)
            if capture.is_symlink() or not capture.is_file() or capture.stat().st_size > 32 * 1024 * 1024:
                raise ValueError('producer did not emit a bounded regular capture')
            raw = capture.read_bytes()
            value = validate_capture(raw, case)
            destination = evidence / (case.name + '.json')
            destination.write_bytes(raw)
            result['capture_sha256'] = digest(destination)
            result['schema'] = value['schema']
            result['character_id'] = value['character_id']
            if case.name == 'primal':
                result['native_server_artifact_digest'] = value['native_server_artifact_digest']
            if client_archive is None or (CLIENT, case.consumer) not in client_listing:
                raise ValueError('same-head client archive/consumer unavailable')
            command = nextest(client_archive, args.workspace, [test_filter(CLIENT, case.consumer)]) + options
            result['consumer_exit'] = run(wrapped(args.runner, command,
                [variable, 'OTERYN_ASSET_DIR=' + args.workspace]), consumer_log)
            completed(result['consumer_exit'], consumer_log, case.consumer_marker)
            if capture.read_bytes() != raw:
                raise ValueError('client replay changed the real capture')
            result['state'] = 'PASS'
    except Exception as error:
        result['error'] = str(error)
        print(f'{case.name}: {error}', flush=True)
    for path in (producer_log, consumer_log):
        if path.exists():
            result[path.stem + '_sha256'] = digest(path)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('plan', 'run'))
    for name in ('filters', 'listing'):
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ('archive', 'priority', 'client-directory', 'evidence'):
        parser.add_argument('--' + name, type=Path)
    for name in ('head', 'client-manifest', 'pg-manifest', 'partition'):
        parser.add_argument('--' + name)
    args = parser.parse_args()
    ordinary, selected = partition(args.filters.read_text().splitlines(), json.loads(args.listing.read_bytes()))
    if args.mode == 'plan':
        print('assigned real capture producers:', ', '.join(case.name for case in selected) or 'none')
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            output.write(f'required={str(bool(selected)).lower()}\n')
        return 0
    args.runner, args.workspace = os.environ['PG_RUN'], os.environ['PG_WORKSPACE']
    # Preserve ordinary assigned tests and no-fail-fast even if a replay fails.
    code = subprocess.run(wrapped(args.runner, nextest(args.archive, args.workspace, ordinary, args.priority))).returncode if ordinary else 0
    if not selected:
        return code
    if (re.fullmatch('[0-9a-f]{40}', args.head or '') is None
            or re.fullmatch('[0-9a-f]{64}', args.pg_manifest or '') is None):
        raise ValueError('missing exact-head PG archive binding')
    head = subprocess.check_output([args.runner, 'git', '-C', args.workspace, 'rev-parse', 'HEAD'], text=True).strip()
    if head != args.head:
        raise ValueError('replay workspace head changed')
    args.evidence.mkdir(parents=True, exist_ok=False)
    record = {'schema': 'OTERYN_NATIVE_CLIENT_REPLAY_EVIDENCE/v1', 'state': 'RUNNING', 'head': head,
              'run_id': os.environ['GITHUB_RUN_ID'], 'run_attempt': os.environ['GITHUB_RUN_ATTEMPT'],
              'partition': args.partition, 'pg_manifest_sha256': args.pg_manifest,
              'pg_archive_sha256': digest(args.archive), 'client_manifest_sha256': args.client_manifest,
              'asset_root': args.workspace, 'ordinary_exit': code,
              'assigned': [case.name for case in selected], 'results': []}
    client_archive, client_listing = None, set()
    try:
        client_archive = verify_client_archive(args.client_directory, head, args.client_manifest or '')
        record['client_archive_sha256'] = digest(client_archive)
        command = ['cargo-nextest', 'nextest', 'list', '--archive-file', str(client_archive),
                   '--workspace-remap', args.workspace, '--message-format', 'json']
        client_listing = listed_cases(json.loads(subprocess.check_output(wrapped(args.runner, command))))
        if not {(CLIENT, case.consumer) for case in CASES} <= client_listing:
            raise ValueError('client archive is missing a real capture consumer')
    except Exception as error:
        record['client_archive_error'] = str(error)
        code = code or 1
    receipt = args.evidence / 'evidence.json'
    receipt.write_text(json.dumps(record, indent=2) + '\n')
    for case in selected:
        result = replay(case, args, client_archive, client_listing, args.evidence)
        record['results'].append(result)
        receipt.write_text(json.dumps(record, indent=2) + '\n')
        if result['state'] != 'PASS':
            code = code or 1
    record['state'] = 'PASS' if code == 0 else 'FAILED'
    receipt.write_text(json.dumps(record, indent=2) + '\n')
    return code


if __name__ == '__main__':
    sys.exit(main())
