"""Promote only marker-backed SOURCE qualification; retain authored obligations."""
from collections import Counter
from pathlib import Path
import hashlib
import json
import re
import subprocess

ROOT = Path('.')
STATUS = ROOT / '.questpc6/QUEST-STATUS.json'
TEST = 'questpc6_source_deeper_fibula_well_and_complete_reward_inventory'
LOG = ROOT / '.questpc6/source-final.log'
text = LOG.read_text()
assert 'test result: ok. 1 passed; 0 failed; 0 ignored;' in text
assert 'OTERYN_TEST_POSTGRES_ADMIN_URL is not configured' not in text
match = re.search(r'POSTGRES_CASE_COMPLETED ' + TEST + r' duration=([0-9.]+)s', text)
assert match, 'A successful PG marker with positive duration is required'
duration = float(match[1])
assert duration > 0
rows = json.loads(STATUS.read_text())
titles = (ROOT / '.questpc6/MY-TITLES.txt').read_text().splitlines()
assert [r['title'] for r in rows] == titles
row = next(r for r in rows if r['title'] == 'Deeper Fibula Quest')
claims = {r['claim'] for r in row['reward_claims']}
assert len(claims) == 6
# A refused non-native source placement is not source qualification.
actual = set(re.findall(r'QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED (\S+) .* native_source=true duration=[0-9.]+s', text))
assert claims <= actual
assert not re.search(r'QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED (?:' + '|'.join(re.escape(c) for c in claims) + r') .*native_source=false', text)
assert re.search(r'POSTGRES_CASE_COMPLETED deeper-fibula-well duration=[0-9.]+s', text)
if 'authored_supplement' not in row:
    row['authored_supplement'] = {
        'status': 'BLOCKED',
        'scope': 'separate chosen Oteryn recipe, not the original source contract',
        'first_unqualified_stage': row.pop('first_unqualified_stage'),
        'missing_piece': 'Key3940/expertise access route and credited eight-Dragon/DragonLord personal encounter require the shared qengine encounter/access owner; the source well and six caches do not implement this chosen supplement.',
        'evidence': [row['definition'], '.questpc6/NOTE-coordination.md:16'],
    }
row.pop('blocker', None)
row.pop('queued_stage_binding', None)
row['status'] = 'QUALIFIED'
row['scope'] = 'complete original SOURCE well + six reward-claim contract; separate authored supplement remains BLOCKED'
row['qualification'] = {
    'test': TEST,
    'marker': 'POSTGRES_CASE_COMPLETED ' + TEST,
    'duration_seconds': duration,
    'log': str(LOG),
    'log_sha256': hashlib.sha256(LOG.read_bytes()).hexdigest(),
    'dispatch': 'literal framed USE through serve_admitted, source callback and every native source chest placement; stale well target, replay, unchanged quest state, exact items and refusal',
}
for claim in row['reward_claims']:
    claim['status'] = 'dispatch_pg_qualified'
    claim['retained_test'] = TEST
    claim['qualification_log'] = str(LOG)
for r in rows:
    assert r['status'] in {'QUALIFIED', 'BLOCKED'}, r['title']
    if r['status'] == 'QUALIFIED':
        assert r['qualification']['duration_seconds'] > 0
        continue
    blocker = r['blocker']
    assert blocker['missing_piece'] and blocker['attempts'] and blocker['evidence'], r['title']
    assert not blocker['missing_piece'].startswith('No complete real-dispatch PG qualification'), r['title']
    for ref in blocker['evidence']:
        path, locator = ref.split(':', 1)
        line = int(locator.split('#')[0])
        assert Path(path).is_file(), ref
        assert 1 <= line <= len(Path(path).read_text().splitlines()), ref
STATUS.write_text(json.dumps(rows, indent=2, ensure_ascii=False) + '\n')
md = ['# questpc6 status', '',
      'Qualification scope is explicit: the Fibula SOURCE contract is qualified; its separate authored recipe is still blocked. No selected storyline, outfit or deployed map is certified by a chest receipt.', '',
      '| Title | Status | Evidence / missing engine |', '| --- | --- | --- |']
for r in rows:
    detail = (r['qualification']['test'] + f' ({duration:.9f}s, original SOURCE only)'
              if r['status'] == 'QUALIFIED' else r['blocker']['missing_piece'])
    md.append(f"| {r['title']} | {r['status']} | {detail} |")
Path('.questpc6/QUEST-STATUS.md').write_text('\n'.join(md) + '\n')
files = [
    'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc6_access.rs',
    'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs',
    'crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs',
    'crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs',
    'crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs',
    'content/quests/definitions/quests-00000-00099.json',
    'docs/repository/QUEST_CALLBACK_COVERAGE.json',
    '.questpc6/QUEST-STATUS.json',
]
audit = {
    'task_id': 'questpc6',
    'base_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
    'counts': dict(Counter(r['status'] for r in rows)),
    'qualification': row['qualification'],
    'file_sha256': {p: hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in files},
    'no_full_chosen_recipe_claim': True,
    'no_publication_or_index_mutation': True,
}
Path('.questpc6/final-status-audit.json').write_text(json.dumps(audit, indent=2) + '\n')
print(json.dumps(audit['counts']))
print(f'QUALIFIED: Deeper Fibula Quest - {TEST} (original SOURCE contract only, {duration}s)')
