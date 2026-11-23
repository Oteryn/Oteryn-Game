"""Source/predicate demonstrations only. Does not compile Rust, connect to PG, or alter repository files."""
import argparse, hashlib, json, re, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--repo',required=True,type=Path)
p.add_argument('--output',required=True,type=Path)
a=p.parse_args();root=a.repo.resolve()
base='8e9f0e16631aab1beb8416f8d5ca707aa3031083'
seen={}
def source(path):
    raw=(root/path).read_bytes()
    actual=hashlib.sha1(f'blob {len(raw)}\0'.encode()+raw).hexdigest()
    expected=subprocess.check_output(['git','-C',str(root),'rev-parse',f'{base}:{path}'],text=True).strip()
    assert actual==expected,path
    seen[path]=actual
    return raw.decode()
d='crates/gameplay/durability/src/durability/'
contract=source(d+'schema_contract_tests.rs')
journal=source(d+'admission_journal.rs')
core=source(d+'core_modules.rs')
visibility={
    'original_three_contains_predicates':[
        'pub async fn reconcile(' in journal,
        'pub const fn legacy(' in core,
        'pub async fn reconcile(' in core],
    'public_module_export': 'pub mod admission_journal;' in core,
    'public_type_reexport':'pub use admission_journal::AdmissionReconnectJournal;' in core,
    'claim':'Original source oracle accepts these unconditionally public entry spellings; not a runtime vulnerability claim'}
bank=source('apps/game-server/tests/support/bank_postgres_cases.rs')
crash=bank.split('fn a_crash_at_any_step_leaves_nothing_and_an_ambiguous_commit_resolves_by_replay()',1)[1].split('async fn expect_guard',1)[0]
loop=crash.split('for table in [',1)[1].split('// The commit succeeds',1)[0]
tables=re.findall(r'"(game_\w+)"',loop)
assert len(tables)==5
abort={
    'loop_table_observations':tables,
    'complete_runtime_transaction_before_select_count': loop.index('runtime_transaction(&harness, &request)')<loop.index('SELECT count(*)'),
    'forced_error_after_complete_operation_and_count':loop.index('SELECT count(*)')<loop.index('SELECT 1/0'),
    'claim':'Five observations share the same post-full-writer fault boundary; actual DB paths not run'}
db=source(d+'db.rs')
constant=db.split('const ADMISSION_RELATIONS_LOCK: &str = "',1)[1].split('";',1)[0]
expected=re.findall(r'game_durability_\w+',constant)
assert len(expected)==17 and expected==sorted(expected)
replacement='game_durability_native_source_registration'
assert f'CREATE TABLE {replacement}' in source('apps/game-server/migrations/0004_native_admission_source.sql')
mutant=sorted([x for x in expected if x!='game_durability_admission_account_guards']+[replacement])
lock={
    'expected_from_production_sql':expected,
    'hypothetical_removed_relation':'game_durability_admission_account_guards',
    'hypothetical_replacement_existing_relation':replacement,
    'mutated_expected_count':len(mutant),
    'original_count_and_sort_predicates_accept':len(mutant)==17 and all(a<b for a,b in zip(mutant,mutant[1:])),
    'equality_predicate_if_pg_locks_matches_mutated_sql':mutant==list(mutant),
    'claim':'Predicate model only; no mutated SQL or PostgreSQL run. Does not establish a failure of the whole suite'}
economy=source('crates/gameplay/economy/src/bank.rs')
code=economy.split('pub fn code(self) -> i16 {',1)[1].split('pub fn from_code',1)[0]
mapping={name:int(number) for name,number in re.findall(r'Self::(\w+) => (\d+)',code)}
assert sorted(mapping.values())==list(range(9))
mutant_mapping=dict(mapping)
mutant_mapping['JuniorAccount'],mutant_mapping['RecipientCannotReceiveTransfers']=mutant_mapping['RecipientCannotReceiveTransfers'],mutant_mapping['JuniorAccount']
def roundtrip_accepts(m):
    for code in range(9):
        variant=next((name for name,value in m.items() if value==code),None)
        if variant is None or m[variant]!=code:return False
    return next((name for name,value in m.items() if value==9),None) is None
result_codes={
    'production_decoder_uses_same_code_method':'.find(|result| result.code() == code)' in economy,
    'original_mapping':mapping,
    'hypothetical_mapping':mutant_mapping,
    'original_predicates_accept_baseline':roundtrip_accepts(mapping),
    'original_predicates_accept_swapped_meanings':roundtrip_accepts(mutant_mapping),
    'independent_named_mapping_accepts_swap':mapping==mutant_mapping,
    'claim':'Model of this one unit predicate; no Rust, migration, or full-suite mutation execution'}
linkage=source(d+'core_modules_linkage_tests.rs')
conditional={
    'registration_if_let_present':'if let RegistrationError::Unavailable(inner)' in linkage,
    'assignment_if_let_present':'if let AssignmentError::Unavailable(inner)' in linkage,
    'successful_fixture_errors_return_without_failure':'let Ok(actor) = ControlActor::new("operator") else {\n            return;\n        };' in linkage,
    'claim':'Nonmatching outer variants bypass inner assertions. Fixture decoder failures skip subsequent encoding assertions'}
mounts=[]
for path in sorted(root.glob('apps/game-server/tests/*.rs')):
    if '#[path = "../../../crates/gameplay/durability/src/character_bootstrap_intent.rs"]' in path.read_text():mounts.append(str(path.relative_to(root)))
result={'baseline':base,'mode':'READ_ONLY_SOURCE_AND_PREDICATE_MODEL; NOT_RUST_EXECUTION; NOT_PG_EXECUTION','source_blobs_verified':seen,'DUR-01':visibility,'DUR-02':abort,'DUR-03':lock,'DUR-05':conditional,'DUR-07':result_codes,'DUR-08':{'top_level_mount_count':len(mounts),'top_level_mount_paths':mounts,'canonical_targets_with_duplicate_source':['durability_postgres','native_admission_source_postgres','runtime_scope_assignment_postgres'],'owning_parser_literal_test_functions':5,'compiled_or_executed_duplicate_case_count':'UNKNOWN'}}
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'verified_source_files':len(seen),'top_level_parser_mounts':len(mounts),'model_only':True,'output':str(a.output)},indent=2))
