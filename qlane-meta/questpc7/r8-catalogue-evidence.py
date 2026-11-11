"""Verify exact run-8 A..M PG markers and individual test success in a shared log."""
import argparse
import hashlib
import json
import re
from pathlib import Path

PREFIX = 'gameplay_transport::monk_save::postgres_tests::quest_catalogue_chests::'
BATCH_TEST = PREFIX + 'quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal'
KEY_TEST = PREFIX + 'quest_catalogue_chests_mintwallin_prison_key3620_use_replay_and_refusal'
SOURCE_PATH = 'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs'
SNAPSHOTS = [
    (Path('.questpc/r8-catalogue-claims.json'), BATCH_TEST),
    (Path('.questpc/r8-catalogue-key3620-claims.json'), KEY_TEST),
]

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('log', type=Path)
parser.add_argument('--require-complete', action='store_true')
args = parser.parse_args()
source = {}
source_hashes = {}
for path, test in SNAPSHOTS:
    source_hashes[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
    for key, row in json.loads(path.read_text()).items():
        if key in source:
            raise ValueError(f'duplicate source claim: {key}')
        source[key] = {**row, 'fully_qualified_name': test}
if len(source) != 133:
    raise ValueError(f'expected frozen132 plus Mintwallin key3620, got {len(source)}')

raw = args.log.read_bytes()
text = raw.decode()
pattern = re.compile(
    r'QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED (\S+) '
    r'\((\d+), (\d+), (-?\d+)\) native_source=(true|false) duration=(\S+)'
)
matches = list(pattern.finditer(text))
if text.count('QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED ') != len(matches):
    raise ValueError('malformed catalogue completion marker')
completed = {}
for match in matches:
    claim, x, y, floor, native, duration = match.groups()
    if claim not in source:
        raise ValueError(f'out-of-scope completion marker: {claim}')
    parsed_duration = re.fullmatch(r'([0-9]+(?:\.[0-9]+)?)(s|ms|µs|ns)', duration)
    if native != 'true' or parsed_duration is None:
        raise ValueError(f'invalid completion: {match.group()}')
    magnitude, unit = parsed_duration.groups()
    seconds = float(magnitude) * {'s': 1, 'ms': 1e-3, 'µs': 1e-6, 'ns': 1e-9}[unit]
    if seconds <= 0:
        raise ValueError(f'nonpositive completion duration: {match.group()}')
    position = (int(x), int(y), -int(floor))
    expected = {tuple(p['position'][axis] for axis in ('x', 'y', 'z'))
                for p in source[claim]['placements']}
    if position not in expected:
        raise ValueError(f'unknown source placement: {claim} {position}')
    key = (claim, position)
    if key in completed:
        raise ValueError(f'duplicate source completion: {key}')
    completed[key] = {
        'position': dict(zip(('x', 'y', 'z'), position)),
        'native_source': True,
        'duration': duration,
        'duration_seconds': seconds,
        'marker': match.group(),
    }

individual_results = {}
for name in (BATCH_TEST, KEY_TEST):
    matches = re.findall(r'^test ' + re.escape(name) + r' \.\.\. (ok|FAILED)$', text, re.MULTILINE)
    if len(matches) > 1:
        raise ValueError(f'ambiguous individual test result: {name} {matches}')
    individual_results[name] = matches[0] if matches else 'pending'

claims = {}
ledger_candidates = {}
for key, row in source.items():
    positions = [tuple(p['position'][axis] for axis in ('x', 'y', 'z')) for p in row['placements']]
    observed = [completed[(key, p)] for p in positions if (key, p) in completed]
    all_placements = len(observed) == len(positions)
    test = row['fully_qualified_name']
    qualified = all_placements and individual_results[test] == 'ok'
    claims[key] = {
        'quest': row['quest'],
        'fully_qualified_name': test,
        'expected_placements': len(positions),
        'completed_placements': observed,
        'all_placements_completed': all_placements,
        'individual_test_result': individual_results[test],
        'qualified': qualified,
    }
    if qualified:
        ledger_candidates[key] = {
            'disposition': 'test',
            'evidence': {
                'kind': 'test',
                'path': SOURCE_PATH,
                'fully_qualified_name': test,
                'crate_or_target': 'oteryn-gameplay-dispatch --all-features --lib',
                'asserted_scenario': (
                    'Exact authored World source selectors; literal production USE dispatch; '
                    'native reward mint and complete custody/attributes; once-only receipt; '
                    'retained duplicate replay; cold-owner replay with identical item identity; '
                    'second new-command refusal; stale-generation refusal without reservation. '
                    'Reward proof does not qualify quest access callbacks.'
                ),
                'observed_result': (
                    f'PASS individual test; {len(observed)}/{len(positions)} exact native-source '
                    f'POSTGRES_CASE_COMPLETED markers with positive elapsed durations in {args.log}'
                ),
                'postgres_completion_markers': observed,
            },
        }

result = {
    'log': str(args.log),
    'log_sha256': hashlib.sha256(raw).hexdigest(),
    'source_snapshots_sha256': source_hashes,
    'expected_claims': len(source),
    'expected_placements': sum(len(row['placements']) for row in source.values()),
    'completed_markers': len(completed),
    'qualified_claims': len(ledger_candidates),
    'individual_test_results': individual_results,
    'aggregate_test_result_lines': re.findall(r'^test result:.*$', text, re.MULTILINE),
    'claims': claims,
}
complete = len(ledger_candidates) == len(source)
candidate_path = Path('.questpc/r8-catalogue-ledger-candidate.json')
if not complete:
    candidate_path.unlink(missing_ok=True)
result['complete'] = complete
Path('.questpc/r8-catalogue-root.evidence.json').write_text(json.dumps(result, indent=2) + '\n')
if complete:
    candidate_path.write_text(
        json.dumps({'log': str(args.log), 'log_sha256': result['log_sha256'],
                    'claims': ledger_candidates}, indent=2) + '\n'
    )
print(json.dumps({key: value for key, value in result.items() if key != 'claims'}, indent=2))
if args.require_complete and not complete:
    raise SystemExit(1)
