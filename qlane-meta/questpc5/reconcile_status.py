"""Update this task's status record only from retained real-dispatch proofs."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys

status_path = Path('.questpc/QUEST-STATUS.json')
status = json.loads(status_path.read_text())
proof_path = Path(sys.argv[1])
proof = json.loads(proof_path.read_text())
result = json.loads(proof_path.with_name(proof_path.name.replace('.qualified.json', '.result.json')).read_text())
manifest_path = Path('.questpc/restart10-standalone-cohorts.json')
manifest = json.loads(manifest_path.read_text())
source_path = Path('crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs')
assert proof['manifest_sha256'] == hashlib.sha256(manifest_path.read_bytes()).hexdigest()
assert proof['test_source_sha256'] == hashlib.sha256(source_path.read_bytes()).hexdigest()
log_path = Path(proof['log'])
log = log_path.read_text()
assert result['exit'] == 0
assert result['log_sha256'] == hashlib.sha256(log_path.read_bytes()).hexdigest()
assert set(result['test_results']) == set(manifest)
assert all(value == 'ok' for value in result['test_results'].values())
assert set(proof['qualified']) == set(manifest)
coverage = {row['quest']: row for row in json.loads(Path('docs/repository/QUEST_CALLBACK_COVERAGE.json').read_text())['quests']}
definitions = {}
for path in Path('content/quests/definitions').glob('quests-*.json'):
    for row in json.loads(path.read_text())['records']:
        definitions[row['definition']['identity']['key']] = row['definition']
updates = {}
for test, cohort in proof['qualified'].items():
    assert cohort['quests'] == manifest[test]['quests']
    for quest in cohort['quests']:
        updates[quest] = (test, cohort)
for row in status['quests']:
    definition = definitions[row['canonical']]
    source = definition.get('source_refs', {}).get('quest', {}).get('key')
    if source not in updates:
        continue
    assert not row.get('excluded')
    assert definition['kind'] == 'reward_only'
    assert definition['shown_in_quest_log'] is False
    assert not definition['source_refs']['gates']
    assert set(definition['reported_source_readiness']['features']) == {'reward_claim', 'trigger_use'}
    claims = {claim['key'] for claim in definition['claims']}
    assert claims == {claim['claim'] for claim in coverage[source]['chests']}
    test, cohort = updates[source]
    qualification = []
    for claim in coverage[source]['chests']:
        placements = cohort['claims'][claim['claim']]
        assert {tuple(p['position']) for p in placements} == {
            tuple(p['position'][axis] for axis in ('x', 'y', 'z')) for p in claim['placements']}
        for placement in placements:
            assert placement['marker'] in log
            assert 'POSTGRES_CASE_COMPLETED' in placement['marker']
            assert 'native_source=true' in placement['marker']
            assert float(placement['duration'][:-1]) > 0
        qualification.append({'claim': claim['claim'], 'test': test,
                              'placements': placements, 'evidence': str(proof_path), 'log': str(log_path)})
    row.update(status='QUALIFIED', scope='standalone source reward-only contract', qualification=qualification)
    for key in ('blocker', 'first_unqualified_stage', 'queued_stage_binding'):
        if key in row:
            row.setdefault('additional_authored_programme_not_qualified', {})[key] = row.pop(key)
status['counts'] = dict(Counter(row['status'] for row in status['quests']))
status['qualification_boundary'] = (
    'QUALIFIED is explicitly scoped per row. Standalone source contracts prove every original '
    'reward claim and native source placement through production USE, replay and refusal. '
    'Additional authored programme routes are retained separately and are not certified by '
    'claim tests. Historical source readiness holds remain unchanged. BLOCKED requires an '
    'owner exclusion or a concrete engine piece outside this lane; missing tests/composition '
    'are UNFINISHED and must be implemented.')
status['latest_pg'] = {'evidence': str(proof_path), **result}
status_path.write_text(json.dumps(status, indent=2) + '\n')
lines = ['# questpc A..M quest status', '',
         '178 pinned titles: ' + ', '.join(f'{count} {state}' for state, count in sorted(status['counts'].items())) + '.', '',
         status['qualification_boundary'], '',
         'Exact claims, per-placement POSTGRES_CASE_COMPLETED markers/durations, canonical JSON pointers, attempts and commands: `QUEST-STATUS.json`.', '',
         '| Quest | Status | Evidence / remaining work |', '| --- | --- | --- |']
for row in status['quests']:
    if row['status'] == 'QUALIFIED':
        tests = sorted({q['test'] for q in row['qualification']})
        evidence = ', '.join(f'`{test}`' for test in tests) + '; ' + row['scope']
    else:
        evidence = row['blocker']['missing_piece'] + ' `' + row['definition'] + '`'
    lines.append('| ' + row['title'].replace('|', '\\|') + ' | ' + row['status'] + ' | ' + evidence.replace('|', '\\|').replace('\n', ' ') + ' |')
Path('.questpc/QUEST-STATUS.md').write_text('\n'.join(lines) + '\n')
print(json.dumps(status['counts'], sort_keys=True))
