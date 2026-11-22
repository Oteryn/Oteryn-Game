"""Record source-review scope and static evidence; do not import or run repo tests."""
from __future__ import annotations

import argparse
import ast
from collections import defaultdict
import csv
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
ROOT = args.repo.resolve()
OUT = args.output.resolve()
OUT.mkdir(parents=True, exist_ok=True)
EVIDENCE_SEED = Path(__file__).resolve().parent / 'content_tools_oracle_evidence.json'
BASE = '8e9f0e16631aab1beb8416f8d5ca707aa3031083'
INDEX = ROOT / 'docs/agents/evidence/OTV2-20261122-test-ci-value-audit'
rows = list(csv.DictReader((INDEX / 'test_value_inventory.tsv').open(), delimiter='\t'))
content = [r for r in rows if r['path'].startswith(('tools/content-schema/', 'tools/content-migration/'))
           and not r['path'].startswith('tools/content-schema/craft-authoring/')]
prior = defaultdict(list)
for row in csv.DictReader((INDEX / 'test_value_semantic_reviews.tsv').open(), delimiter='\t'):
    prior[row['path']].append(row['reviewed_scope'])

full_tests = {
    'tools/content-schema/quest-authoring/test_questpc2_albinius_rules.py': 'KEEP_DISTINCT_GENERATOR_AND_LITERAL_GAMEPLAY_GUARDS',
    'tools/content-schema/quest-authoring/test_questpc2_gerimor_rules.py': 'KEEP_DISTINCT_GENERATOR_AND_LITERAL_GAMEPLAY_GUARDS',
    'tools/content-schema/quest-authoring/test_npc_progress_retention.py': 'KEEP_DISTINCT_INFERRED_PROGRESS_FIXTURE',
    'tools/content-schema/quest-authoring/test_progress_enrichment.py': 'KEEP_DECLARED_PROGRESS_AND_WRITER_CONFLICT_GUARDS',
    'tools/content-migration/test_world_project_v2_to_tree.py': 'KEEP_ADMISSION_NEGATIVES_AND_CURRENT_SCHEDULER',
    'tools/content-migration/test_parallel_checks.py': 'KEEP_FAILURE_AGGREGATION_AND_INVENTORY_GUARDS',
    'tools/content-schema/quest-authoring/test_quest_reward_only_binding_candidates.py': 'KEEP_POLICY_ASSERTIONS; CONSIDER_SINGLE_OWNER_BYTE_REPLAY',
    'tools/content-schema/quest-authoring/test_questpc2_gnomus_rules.py': 'KEEP_DISTINCT_GENERATOR_AND_LITERAL_GAMEPLAY_GUARDS',
    'tools/content-schema/quest-authoring/test_questpc2_henricus_rules.py': 'KEEP_DISTINCT_GENERATOR_AND_LITERAL_GAMEPLAY_GUARDS',
    'tools/content-migration/test_native_world_canonicalization.py': 'KEEP_COMPONENT_BOUNDARY_FAILURES; RETAIN_REAL_NATIVE_CI_COMPLEMENT',
    'tools/content-migration/test_world_compiler_artifact.py': 'KEEP_ARTIFACT_IDENTITY_CHECKSUM_AND_NO_STALE_FALLBACK',
}
full_context = {
    'tools/content-schema/quest-authoring/run_checks.py': 'CURRENT_CHECKS_DONOR_AND_UNIT_PARTITION; CASE_ID_PRESERVATION',
    'tools/content-migration/parallel_checks.py': 'CURRENT_BOUNDED_CHECK_SCHEDULER',
    'tools/content-schema/item-authoring/check_tibiawiki165_historical_context.py': 'CURRENT_HISTORICAL_PIN_WRAPPER; DUPLICATED_WEAPON_CHECK',
    'tools/content-migration/native_world_canonicalization.py': 'CURRENT_STAGED_NATIVE_BOUNDARY_CONTRACT',
    '.github/workflows/quest-authoring-schema.yml': 'CURRENT_CI_CONSUMER',
}
partial_tests = {
    'tools/content-schema/item-authoring/test_engine_items.py': ([(1,170),(245,343),(459,542),(2959,3090),(3727,3752),(3811,3957)], 'REGISTER_OMITTED_FROST_CANNON_CASE; KEEP_PARSER_BOUNDARIES'),
    'tools/content-schema/item-authoring/test_lower_item_weapon_metadata_packet.py': ([(1,115)], 'KEEP_LITERAL_NUMERIC_AND_TYPE_BOUNDARIES'),
    'tools/content-schema/quest-authoring/test_converter_enrichment.py': ([(1,116),(260,398)], 'KEEP_LITERAL_LUA_PARSER_SCOPE_AND_LEXICAL_NEGATIVES'),
    'tools/content-migration/test_regenerate_content.py': ([(1,110),(489,605),(1081,1175)], 'KEEP_COMPONENT_PRODUCER_ORDER_AND_REFUSAL_GUARDS'),
    'tools/content-schema/item-authoring/test_appearance_membership.py': ('last13', 'NO_OMITTED_TEST_PROVEN; DYNAMIC_GLOBALS_DISCOVERY'),
    'tools/content-schema/item-authoring/test_lower_bed_packet.py': ('last13', 'NO_OMITTED_TEST_PROVEN; DYNAMIC_GLOBALS_DISCOVERY'),
    'tools/content-schema/item-authoring/test_lower_ranged_packet.py': ('last13', 'NO_OMITTED_TEST_PROVEN; DYNAMIC_GLOBALS_DISCOVERY'),
    'tools/content-schema/item-authoring/test_lower_timed_item_packet.py': ('last13', 'NO_OMITTED_TEST_PROVEN; DYNAMIC_GLOBALS_DISCOVERY'),
    'tools/content-schema/quest-authoring/test_quest_explore_area_candidates.py': ([(1,16),(116,123)], 'KEEP_DISTINCT_GENERATOR; OTHER_CASES_NOT_REVIEWED'),
    'tools/content-schema/quest-authoring/test_quest_talk_npc_candidates.py': ([(1,16),(111,118)], 'KEEP_DISTINCT_GENERATOR; OTHER_CASES_NOT_REVIEWED'),
}
partial_context = {
    'tools/content-schema/item-authoring/engine_items.py': ([(1656,1686)], 'CURRENT_RULE_9132_CONSUMER'),
    'tools/content-schema/quest-authoring/quest_reward_only_binding_candidates.py': ([(158,188)], 'CLI_CHECK_EQUALS_UNIT_BYTE_REPLAY'),
    '.github/workflows/content-tree-migration.yml': ([(160,212)], 'CURRENT_MIGRATION_PARALLEL_ENTRYPOINT_AND_MUTUALLY_EXCLUSIVE_FALLBACK'),
    '.github/workflows/item-authoring-schema.yml': ([(95,180),(185,218)], 'CURRENT_ENGINE_AND_WEAPON_CONSUMERS'),
    '.github/workflows/world-project-package.yml': ([(110,146)], 'REAL_NATIVE_REPRODUCTION_AND_IDEMPOTENCE_CONSUMER'),
    '.github/workflows/merge-group-gate.yml': ([(436,449)], 'CURRENT_COMPILER_ARTIFACT_TEST_CONSUMER'),
    '.github/workflows/integration-shards-dispatch.yml': ([(83,96)], 'CURRENT_COMPILER_ARTIFACT_TEST_CONSUMER'),
}

tree = {}
raw_tree = subprocess.check_output(['git','ls-tree','-r','-z',BASE], cwd=ROOT)
for entry in raw_tree.split(b'\0'):
    if not entry:
        continue
    header, path = entry.split(b'\t', 1)
    tree[path.decode()] = header.split()[2].decode()

scopes = []
def add(path, kind, ranges, disposition):
    raw = (ROOT / path).read_bytes()
    line_count = len(raw.decode().splitlines())
    if ranges == 'all':
        ranges = [(1,line_count)]
    elif ranges == 'last13':
        ranges = [(line_count-12,line_count)]
    assert all(1 <= a <= b <= line_count for a,b in ranges), (path,ranges,line_count)
    blob = hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
    assert tree.get(path) == blob, f'Reviewed source drifted from base: {path}'
    is_full = kind.startswith('FULL')
    scopes.append({
        'baseline':BASE, 'path':path, 'review_kind':kind,
        'reviewed_scope':'Whole source file, including fixtures/imports/entrypoint' if is_full else 'Only exact line ranges; remainder not semantically reviewed in this wave',
        'reviewed_lines':';'.join(f'{a}-{b}' for a,b in ranges),
        'total_lines':line_count, 'sha256':hashlib.sha256(raw).hexdigest(),
        'base_blob_matches':True, 'disposition':disposition,
        'runtime_validation':'BLOCKED_IMPORT_DEPENDENCY_RFC3339_VALIDATOR; NO_CASE_EXECUTED' if path.endswith('/test_engine_items.py') else 'SOURCE_REVIEW_ONLY',
        'unreviewed_remainder':'none within this source file; runtime behavior not executed' if is_full else 'all lines outside reviewed_lines; other transitive producers/fixtures not implied reviewed',
    })

for path, disposition in full_tests.items(): add(path,'FULL_TEST_SOURCE','all',disposition)
for path, disposition in full_context.items(): add(path,'FULL_CONTEXT_SOURCE','all',disposition)
for path, (ranges,disposition) in partial_tests.items(): add(path,'PARTIAL_TEST_SOURCE',ranges,disposition)
for path, (ranges,disposition) in partial_context.items(): add(path,'PARTIAL_CONTEXT_SOURCE',ranges,disposition)
scopes.sort(key=lambda x:x['path'])
with (OUT/'content_tools_scopes.tsv').open('w', newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(scopes[0]),delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n');w.writeheader();w.writerows(scopes)

selected = [r for r in content if r['path'].startswith(('tools/content-migration/','tools/content-schema/quest-authoring/','tools/content-schema/item-authoring/')) and r['path'].endswith('.py')]
body_groups = defaultdict(list)
static_files = []
parse_errors = []
for row in selected:
    raw=(ROOT/row['path']).read_bytes()
    static_files.append({'path':row['path'],'sha256':hashlib.sha256(raw).hexdigest()})
    try: parsed=ast.parse(raw)
    except Exception as e:
        parse_errors.append({'path':row['path'],'error':str(e)});continue
    for cls in parsed.body:
        if not isinstance(cls,ast.ClassDef): continue
        for method in cls.body:
            if not isinstance(method,(ast.FunctionDef,ast.AsyncFunctionDef)) or not method.name.startswith('test_'): continue
            key=ast.dump(ast.Module(body=method.body,type_ignores=[]),include_attributes=False)
            body_groups[key].append({'path':row['path'],'class':cls.name,'test':method.name,'line':method.lineno})
clusters=[members for members in body_groups.values() if len({m['path'] for m in members})>1]
static={
    'baseline':BASE,'scope':'Static AST of indexed Python test files in migration, quest and item; not semantic or runtime review',
    'files':static_files,'parse_errors':parse_errors,
    'cross_file_class_method_body_clusters':clusters,
    'methodology':'Compare test method bodies without location attributes or method names. Ignore imports/class setup for hinting; those must be manually checked before action. Top-level function clone search is not covered.',
    'manual_disposition':'All three clusters resolve to distinct imported generators or different class fixtures; no repeated identical execution proved by these clusters.',
}
(OUT/'content_tools_static_hints.json').write_text(json.dumps(static,indent=2)+'\n')

engine_path='tools/content-schema/item-authoring/test_engine_items.py'
parsed=ast.parse((ROOT/engine_path).read_text())
definitions={n.name:n.lineno for n in parsed.body if isinstance(n,ast.FunctionDef) and n.name.startswith('test_')}
main=next(n for n in parsed.body if isinstance(n,ast.FunctionDef) and n.name=='main')
assignment=next(n for n in main.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='tests' for t in n.targets))
selected_names=[n.id for n in assignment.value.elts]
omitted=sorted(set(definitions)-set(selected_names))
assert len(definitions)==108 and len(selected_names)==107 and omitted==['test_frost_cannon_routes_non_pickupable_blocking_prop']
# Preserve the recorded historical import failure; this source-only checkpoint
# does not repeat the runtime attempt or mark it as a current execution.
oracle=json.loads(EVIDENCE_SEED.read_text())
oracle.update({'baseline':BASE,'path':engine_path,'manual_main_line':main.lineno,'definition_lines':definitions,'selected_names':selected_names,
               'selection_proof':'AST of top-level test functions compared to the complete manually enumerated main tests list; not a test execution count.'})
(OUT/'content_tools_oracle_evidence.json').write_text(json.dumps(oracle,indent=2)+'\n')

partial_paths=set(partial_tests)
static_paths={r['path'] for r in selected}
remaining=[]
for row in content:
    path=row['path']
    if path in full_tests: continue
    remaining.append({
        'baseline':BASE,'path':path,'group':row['group'],'python_test_definitions':row['python_test_definitions'],
        'wave_status':'PARTIALLY_REVIEWED' if path in partial_paths else ('STATIC_AST_ONLY' if path in static_paths else 'INVENTORY_ONLY'),
        'wave_scope':next((s['reviewed_lines'] for s in scopes if s['path']==path),''),
        'prior_reviewed_scope':' || '.join(prior[path]),
        'next_step':'Review only unvisited cases/source, respecting earlier exact scopes; identify actual runner and independent expected behavior before any retirement',
    })
with (OUT/'content_tools_remainingfiles.tsv').open('w', newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(remaining[0]),delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n');w.writeheader();w.writerows(remaining)

groups=defaultdict(lambda:{'test_files':0,'python_test_definitions':0,'bytes':0})
for row in content:
    group=groups[row['group']];group['test_files']+=1;group['python_test_definitions']+=int(row['python_test_definitions']);group['bytes']+=int(row['bytes'])
full_defs=sum(int(r['python_test_definitions']) for r in content if r['path'] in full_tests)
summary={
    'observed_at':datetime.now(timezone.utc).isoformat(),'baseline':BASE,
    'scope':'content-schema and content-migration, excluding craft-authoring and repository',
    'indexed_test_files':len(content),'indexed_python_test_definitions':sum(int(r['python_test_definitions']) for r in content),
    'static_ast_files':len(selected),'static_ast_parse_errors':len(parse_errors),'static_method_body_hint_clusters':len(clusters),
    'full_test_source_files':len(full_tests),'literal_test_definitions_in_full_test_source_files':full_defs,
    'partial_test_source_files':len(partial_tests),'full_context_source_files':len(full_context),'partial_context_source_files':len(partial_context),
    'remaining_not_fully_read_in_this_wave':len(remaining),'all_scope_bytes_match_base':True,
    'remaining_file_semantics':'This wave removes only its 11 fully read test files. Prior exact scopes remain in a separate column, not erased or expanded to full-file review.',
    'runtime':'One focused case attempt blocked before execution at import by missing rfc3339-validator; no successful test execution and no full suite in this wave.',
    'groups':dict(sorted(groups.items())),
}
(OUT/'content_tools_review_summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='groups'},indent=2))
