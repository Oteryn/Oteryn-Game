"""Promote only exact reward selectors with completed native dispatch proof."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess

coverage_path = Path('docs/repository/QUEST_CALLBACK_COVERAGE.json')
ledger_path = Path('docs/agents/evidence/quest-claim-qualification.json')
before_coverage = json.loads(coverage_path.read_text())
before_ledger = json.loads(ledger_path.read_text())
manifest = json.loads(Path('.questpc/restart4-story-reward-cohorts.json').read_text())
qualified = {}
for suffix in ('', '-2', '-3'):
    proof = json.loads(Path(f'.questpc/restart4-story-reward-pg{suffix}.qualified.json').read_text())
    qualified.update(proof['qualified'])
assert set(qualified) == set(manifest)
new = {}
for name, row in manifest.items():
    assert set(row['claims']) == set(qualified[name]['claims'])
    for key, claim in row['claims'].items():
        assert len(claim['placements']) == len(qualified[name]['claims'][key])
        if claim['status'] == 'supported_unqualified':
            assert key not in new
            new[key] = name
assert len(new) == 44
producer = Path('tools/content-schema/quest-authoring/quest_callback_coverage.py')
source = producer.read_text()
anchor = '\nFALCON_PREFIX = '
assert anchor in source
block = '\n# Story and outfit reward sources: 44 additional claims passed all55 native\n'
block += '# source placements in three shared USE/replay/refusal cohorts. This does\n'
block += '# not certify the surrounding story, arena, hand-in or outfit path.\n'
block += 'for test_name, claims in (\n'
for name in manifest:
    block += f'    ({name!r}, (\n'
    for key in sorted(k for k, value in new.items() if value == name):
        block += f'        {key!r},\n'
    block += '    )),\n'
block += '):\n    QUALIFIED.update({claim: test_name for claim in claims})\n'
producer.write_text(source.replace(anchor, block + anchor, 1))
validator = Path('tools/content-schema/quest-authoring/check_quest_claim_qualification.py')
source = validator.read_text()
source = source.replace('# plus 88 Phase A claims and the Mintwallin prison key have source-selector\n# dispatch PostgreSQL proof.', '# plus 88 Phase A claims, the Mintwallin prison key and 44 further A..M\n# story/outfit reward sources have source-selector dispatch PostgreSQL proof.')
assert 'EXPECTED_DENOMINATOR = 206' in source
validator.write_text(source.replace('EXPECTED_DENOMINATOR = 206', 'EXPECTED_DENOMINATOR = 162'))
unit = Path('tools/content-schema/quest-authoring/test_quest_claim_qualification.py')
source = unit.read_text().replace('# 128 source claims have focused dispatch PG proof;', '# 172 source claims have focused dispatch PG proof;')
assert 'sum(counts.values()), 206' in source
unit.write_text(source.replace('sum(counts.values()), 206', 'sum(counts.values()), 162'))
subprocess.run(['python3', str(producer), '--canary', '.questpc/sources/canary',
                '--crystal', '.questpc/sources/crystal'], check=True)
subprocess.run(['python3', str(validator)], check=True)
after_coverage = json.loads(coverage_path.read_text())
after_ledger = json.loads(ledger_path.read_text())
expected = copy.deepcopy(before_coverage)
seen = set()
for quest in expected['quests']:
    for claim in quest['chests']:
        if claim['claim'] in new:
            assert claim['status'] == 'supported_unqualified'
            claim.update(status='dispatch_pg_qualified', reason=new[claim['claim']])
            seen.add(claim['claim'])
assert seen == set(new)
expected['counts']['supported_unqualified'] -= 44
expected['counts']['dispatch_pg_qualified'] += 44
assert expected == after_coverage, 'unexpected coverage recipe/placement/source/quest drift'
expected = copy.deepcopy(before_ledger)
for key in new:
    del expected['claims'][key]
expected['denominator'] = 162
expected['disposition_totals']['blocked']['WORLD_BINDING_AND_E2E_EVIDENCE_MISSING'] = 162
assert expected == after_ledger, 'unexpected remaining ledger changes'
Path('.questpc/restart4-promotion.json').write_text(json.dumps({
    'new_claims': new, 'new_claim_count': len(new),
    'before_counts': before_coverage['counts'], 'after_counts': after_coverage['counts'],
    'coverage_only_exact_status_and_reason_changes': True,
    'ledger_only_exact_proven_claims_removed_and_totals_updated': True,
    'source_placements_rewards_associations_unchanged': True,
    'coverage_sha256': hashlib.sha256(coverage_path.read_bytes()).hexdigest(),
    'ledger_sha256': hashlib.sha256(ledger_path.read_bytes()).hexdigest(),
}, indent=2) + '\n')
print(json.dumps(after_coverage['counts']))
