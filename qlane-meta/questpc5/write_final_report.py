import hashlib
import json
import re
import subprocess
from collections import Counter
from pathlib import Path

root = Path.cwd()
rows = json.loads((root / '.questpc5/QUEST-STATUS.json').read_text())
checks = {}
for name in ('server-clippy', 'content-native-units', 'server-content', 'world-project', 'fmt'):
    value = json.loads((root / f'.questpc5/final-{name}.result.json').read_text())
    assert value['exit_code'] == 0, name
    checks[name] = value
assert Counter(r['status'] for r in rows) == {'BLOCKED': 30, 'QUALIFIED': 3}
expected = set((root / '.questpc5/MY-TITLES.txt').read_text().splitlines())
assert len(rows) == 33 and {r['title'] for r in rows} == expected
for row in rows:
    proof = row.get('qualification', row.get('partial_qualification'))
    if proof:
        log = (root / proof['log']).read_text()
        assert proof['marker'] in log and proof['duration_seconds'] > 0
        assert 'test result: ok.' in log
    if row['status'] == 'BLOCKED':
        blocker = row['blocker']
        assert blocker['attempts'] and blocker['missing_piece'] and blocker['evidence']
        assert 'No complete real-dispatch PG qualification' not in blocker['missing_piece']
        for evidence in blocker['evidence']:
            match = re.fullmatch(r'(.+):(\d+)(?:#.*)?', evidence)
            assert match and (root / match[1]).is_file(), evidence
            assert len((root / match[1]).read_text().splitlines()) >= int(match[2]), evidence
product = subprocess.check_output(['git', 'diff', '--name-only'], text=True).splitlines()
product.extend([
    'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc5_reward_paths.rs',
    'tools/content-migration/questpc5_father_item_profiles.py',
    'tools/content-migration/test_questpc5_father_item_profiles.py',
])
product.sort()
assert len(product) == len(set(product)) == 15
hashes = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in product}
assert subprocess.run(['git', 'diff', '--check']).returncode == 0
lines = [
    '# questpc5 final local evidence', '',
    'AUTHORING on detached local base28e4313e1; no frozen candidate or publication. All33allocated titles have final ledger rows:3QUALIFIED original-source contracts and30BLOCKED on concrete shared engine pieces. The separately authored approximation stage graphs and a deployed World are not certified.', '',
    '## Changed product files', '',
]
lines.extend(f'- `{path}`' for path in product)
lines.extend(['', 'Lane records: QUEST-STATUS.json, RESUME.md, SHARED-FIXES.md, REVIEW.md, final-status-audit.json and the named logs/results.', '', '## PostgreSQL evidence', ''])
for row in rows:
    if row['status'] == 'QUALIFIED':
        proof = row['qualification']
        lines.extend([f"- QUALIFIED: {row['title']} — `{proof['test']}`, `{proof['marker']}`; `{proof['log']}`.", f"  Scope: {row['scope']}"])
father = next(r for r in rows if r['title'] == "A Father's Burden Quest")
proof = father['partial_qualification']
lines.extend([f"- Partial Father hand-in proof: `{proof['test']}`, `{proof['marker']}`; `{proof['log']}`. Eight exact item debits, progress,28000XP, cape and replay pass. The8000gold and dungeon acquisition paths are not certified; the complete-quest row remains BLOCKED.", '', 'Every PostgreSQL command used CARGO_BUILD_JOBS=3, the absolute queued wrapper, --locked/--offline, and a mechanic filter. No whole PG suite ran. Exact executed filters:', '', '```sh',
'CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc5_ -- --nocapture --test-threads=1',
'CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc5_braindeath_ -- --nocapture --test-threads=1',
'CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib fathers_burden_eight_handins_cape_experience_and_replay -- --nocapture --test-threads=1', '```', '',
'The first filter selected the two lever cohorts when run; Braindeath was added afterward and ran with its own filter. All3source contracts have positive markers. Father RED at2704yes was traced to the1004PL/pgSQL local/column amount ambiguity and repaired by a local-variable rename, with no guard weakened.', '',
'## Other validation', '',
'- Dispatch and durability Clippy, --all-targets --all-features -- -D warnings: PASS, final-clippy-repaired.log. The inherited eight-argument NPC bridge required its narrow documented allowance; no signature or behavior changed.',
'- Explicit rustfmt check of all changed auto-mounted dispatch files and the sequencer bridge: PASS, final-mounted-fmt.log.',
'- Native profile source-closure Python unit:1passed; generator register_spell_families.py --check:PASS. Exactly9newnative profiles,0existing profile changes.',
'- Content native units:32passed,0failed,1pre-existing ignored full-source-manifest test; no ignore was added or changed.',
'- Server integration_rest content_:123passed,0failed.',
])
for name, value in checks.items():
    lines.append(f"- `{name}`: exit0, {value['duration_seconds']:.3f}s wall; `{'.questpc5/final-' + name + '.log'}`.")
lines.extend(['', 'Exact remaining cargo commands (CARGO_BUILD_JOBS=3):', '', '```sh'])
lines.extend(' '.join(value['command']) for value in checks.values())
lines.extend(['```', '', '## Unfinished', '', '30complete quest paths remain BLOCKED. These are engine dependencies with exact file:line references and attempts in QUEST-STATUS.json; none is blocked for missing tests.26depend on qengine2(NPC item/value composition or outfit/addon grants),4on the allocated qengine encounter/reset owner. All139blocker references resolve.', ''])
for row in rows:
    if row['status'] == 'BLOCKED':
        lines.append(f"- {row['title']}: {row['blocker']['missing_piece']}")
lines.extend(['', 'Independent exact-candidate review, final integration/CI and publication remain coordinator work. No Git/index, credential, Docker, protected routing, external posting or dependency-download mutation was performed.'])
(root / '.questpc5/VALIDATION.md').write_text('\n'.join(lines) + '\n')
(root / '.questpc5/CHANGED-FILES.txt').write_text('\n'.join(product) + '\n\nLane records and logs are under .questpc5/; see VALIDATION.md.\n')
(root / '.questpc5/final-validation.json').write_text(json.dumps({'task_id':'questpc5', 'base':'28e4313e1', 'counts':dict(Counter(r['status'] for r in rows)), 'product_hashes':hashes, 'checks':checks}, indent=2) + '\n')
print('Validated15product files,33terminal rows,4positive mechanic PG cases and all required local results.')
