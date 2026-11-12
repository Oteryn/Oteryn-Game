"""Retain real cohort results and checkpoint each five qualified reward sets."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

manifest_path = Path(sys.argv[1] if len(sys.argv) > 1 else '.questpc/restart-phase-a-cohorts-all.json')
manifest = json.loads(manifest_path.read_text())
source = Path('crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs')
source_hash = hashlib.sha256(source.read_bytes()).hexdigest()
command = ['/home/mole/oteryn-cache/codex-bin/cargo', 'test', '--locked', '--offline',
           '-p', 'oteryn-gameplay-dispatch', '--all-features', '--lib',
           '--', *manifest, '--test-threads=1', '--nocapture']
marker_pattern = re.compile(r'QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED (\S+) '
                            r'\((\d+), (\d+), (-?\d+)\) native_source=(true|false) duration=(\S+)')
result_pattern = re.compile(r'^test \S+::(quest_catalogue_chests_\w+_use_replay_and_refusal) \.\.\. (ok|FAILED)$')
current_test = None
observed = {}
results = {}
qualified = {}
start = time.monotonic()
# Preserve every attempt: a rerun must not replace a log referenced by an
# earlier QUALIFIED checkpoint, even if the rerun fails before its first case.
attempt = 1
label = 'restart4-story-reward-pg'
while Path(f'.questpc/{label}.log').exists() or Path(f'.questpc/{label}.result.json').exists():
    attempt += 1
    label = f'restart4-story-reward-pg-{attempt}'
log_path = Path(f'.questpc/{label}.log')
result_path = Path(f'.questpc/{label}.result.json')
qualified_path = Path(f'.questpc/{label}.qualified.json')
print('Retaining attempt at ' + str(log_path), flush=True)
environment = dict(os.environ, CARGO_BUILD_JOBS='4')
with log_path.open('w') as log:
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               text=True, env=environment)
    for line in process.stdout:
        log.write(line)
        log.flush()
        match = marker_pattern.search(line)
        if match:
            claim, x, y, floor, native, duration = match.groups()
            elapsed = re.fullmatch(r'([0-9]+(?:\.[0-9]+)?)(s|ms|µs|ns)', duration)
            if native != 'true' or not elapsed or float(elapsed[1]) <= 0:
                raise ValueError('invalid placement completion: ' + match[0])
            position = (int(x), int(y), -int(floor))
            key = (claim, position)
            if key in observed:
                raise ValueError('duplicate placement completion: ' + str(key))
            observed[key] = {'marker': match[0], 'duration': duration, 'position': position}
        start_match = re.match(r'^test \S+::(quest_catalogue_chests_\w+_use_replay_and_refusal) \.\.\. ', line)
        if start_match:
            current_test = start_match[1]
        match = result_pattern.match(line.rstrip())
        if match:
            name, result = match.groups()
        elif current_test is not None and line.strip() in ('ok', 'FAILED'):
            name, result = current_test, line.strip()
        else:
            continue
        current_test = None
        results[name] = result
        if result != 'ok':
            continue
        cohort = manifest[name]
        evidence = {}
        for claim, record in cohort['claims'].items():
            placements = [tuple(p['position'][axis] for axis in ('x', 'y', 'z'))
                          for p in record['placements']]
            evidence[claim] = [observed[(claim, p)] for p in placements]
        if hashlib.sha256(source.read_bytes()).hexdigest() != source_hash:
            raise ValueError('test source changed during qualification')
        qualified[name] = {'quests': cohort['quests'], 'claims': evidence}
        qualified_path.write_text(json.dumps({
            'log': str(log_path),
            'test_source_sha256': source_hash, 'manifest_sha256': hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
            'command': command, 'qualified': qualified}, indent=2) + '\n')
        titles = [quest.rsplit('/', 1)[-1] for quest in cohort['quests']]
        with Path('.questpc/RESUME.md').open('a') as resume:
            resume.write('QUALIFIED: ' + ', '.join(titles) + ' | test: ' + name +
                         ' | reward-source USE/replay/refusal only; full storyline/outfit/access not certified; all placements and durations: ' +
                         str(qualified_path) + '\n')
        print('QUALIFIED ' + name + ': ' + str(len(titles)) + ' quests, ' + str(len(evidence)) + ' claims', flush=True)
    code = process.wait()
if code == 0 and set(results) != set(manifest):
    code = 1
    print('Missing individual test results: no aggregate success claim', flush=True)
result_path.write_text(json.dumps({
    'log': str(log_path), 'log_sha256': hashlib.sha256(log_path.read_bytes()).hexdigest(),
    'command': command, 'exit': code, 'duration_seconds': round(time.monotonic() - start, 3),
    'test_results': results, 'qualified_cohorts': len(qualified),
    'qualified_quests': sum(len(c['quests']) for c in qualified.values())}, indent=2) + '\n')
print('exit=' + str(code) + ' elapsed=' + str(round(time.monotonic()-start, 3)), flush=True)
raise SystemExit(code)
