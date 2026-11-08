"""Summarize only terminal checks and exact final-tree PostgreSQL evidence."""
import collections
import hashlib
import json
from pathlib import Path

root = Path('.questpc')
read = lambda name: json.loads((root / name).read_text())
result = read('restart4-story-reward-pg-4.result.json')
proof = read('restart4-story-reward-pg-4.qualified.json')
assert result['exit'] == 0
assert result['qualified_cohorts'] == 3 and result['qualified_quests'] == 14
assert len(proof['qualified']) == 3
source = Path('crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs')
assert proof['test_source_sha256'] == hashlib.sha256(source.read_bytes()).hexdigest()
cohorts = proof['qualified']
assert sum(len(row['claims']) for row in cohorts.values()) == 46
assert sum(len(markers) for row in cohorts.values() for markers in row['claims'].values()) == 55
checks = {}
for label in ('initial-check', 'coverage-check', 'ledger-check', 'qualification-units',
              'governance', 'governance-units', 'fmt-check', 'mounted-fmt-check', 'clippy'):
    row = read(f'restart4-{label}.result.json')
    assert row['exit'] == 0
    checks[label] = row
files = read('restart4-changed-product-files.json')
assert len(files) == 6
coverage = json.loads(Path('docs/repository/QUEST_CALLBACK_COVERAGE.json').read_text())
counts = collections.Counter(ch['status'] for q in coverage['quests']
                            if '/' in q['quest'] and 'a' <= q['quest'].split('/')[-1][0] <= 'm'
                            for ch in q['chests'])
assert counts == {'dispatch_pg_qualified': 150, 'not_playable': 2}
summary = {
    'task_id': 'questpc',
    'status': 'PARTIAL: reward sources qualified; complete story/outfit/World paths remain unqualified',
    'changed_product_files': files,
    'checks': checks,
    'final_pg': result,
    'proof': '.questpc/restart4-story-reward-pg-4.qualified.json',
    'new_source_claims_qualified': 44,
    'reward_sets': 14,
    'final_test_claims': 46,
    'final_test_placements': 55,
    'canonical_a_m_inventory': dict(counts),
    'global_inventory': coverage['counts'],
    'source_sha256': {p: hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in files},
    'unfinished': [
        'Inherited aggregate NPC case catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic: command2603 hi to JeanPierre, Rejected!=Ok at classic_quests.rs168; authoritative calendar composition absent. Prior exact failed output retained; original test and guards unchanged; not rerun by this reward-only delta.',
        'Remaining complete NPC item/XP/outfit/story and real World access/admission paths; source claims do not certify these. See restart2-phase-b-c-blockers.md.',
        'QENGINE-owned shared movement/encounter/reset features and their quest composition; not implemented here.',
        'Coordinator staging/World-pin refresh, live migration allocation, independent exact-candidate review, CI and integration of the inherited authoring tree. No Git/index/publication mutation authorized or performed.',
    ],
}
(root / 'restart4-validation.json').write_text(json.dumps(summary, indent=2) + '\n')
lines = [
    '# Restart4 bounded reward-source qualification', '',
    'PARTIAL: 44 additional A..M reward claims qualified; full quests remain separately unqualified.', '',
    'Final3/3actual PostgreSQL cohorts:14reward sets,46claims,55native-source placements,allpositive-duration markers. Two Dreamer claims retain their older qualification locators. Global172qualified/162remaining/2excluded; canonicalA..M150qualified/0remaining/2Dawnport-excluded. No new whole-quest flag or World admission certificate.', '',
    'Changed product files (6):', '',
    *[f'- `{p}`' for p in files], '',
    'Scratch/checkpoint files: `.questpc/RESUME.md`, `restart4-*` scripts, manifests, logs, results, review and qualification evidence. No shared-fixture fix; `.questpc/SHARED-FIXES.md` unchanged.', '',
    'Exact final cargo command (CARGO_BUILD_JOBS=4):', '',
    '```sh', ' '.join(result['command']), '```', '',
    f'Final wall duration: {result["duration_seconds"]}s. Log: `.questpc/restart4-story-reward-pg-4.log`; every placement duration: `.questpc/restart4-story-reward-pg-4.qualified.json`.', '',
    'Qualified reward sets by named test (reward sources only):', '',
]
for name, row in cohorts.items():
    lines.append(f'- `{name}`: ' + ', '.join(q.split('/')[-1] for q in row['quests']))
lines += ['', 'Validation: dispatch all-target/all-feature Clippy -Dwarnings PASS; workspace and explicit mounted-file format checks PASS; qualification7/7 and governance129/129 PASS; both artifact validators PASS. Initial dispatch check passed before source edits. No new enum/field/trait/publicAPI, command/capability, migration, dependency or runtime change. Exact counts, timings and hashes: restart4-validation.json. Metadata structural comparison: restart4-promotion.json. Self-review and inherited review limits: restart4-review.md.', '', 'Unfinished:', '', *[f'- {x}' for x in summary['unfinished']], '', 'Local AUTHORING; no frozen SHA, current CI or mergeability claim.']
(root / 'restart4-validation.md').write_text('\n'.join(lines) + '\n')
print('Final report retained:3/3 PG,55positive placement markers,44newclaims;6productfiles.')
