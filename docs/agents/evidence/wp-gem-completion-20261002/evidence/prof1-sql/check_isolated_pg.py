import subprocess,json,hashlib
from pathlib import Path
root=Path('/workspace/Oteryn-WP-Durable')
out=Path('/workspace/scratch/weapon-proficiency-execution/prof1-sql')
container='oteryn-wp-pg-20261001';db='oteryn_prof1_sql_syntax_20261001'
def pg(sql,database='postgres'):
 return subprocess.run(['docker','exec','-i',container,'psql','-X','-U','oteryn_test_admin','-d',database,'-v','ON_ERROR_STOP=1','-At'],input=sql,text=True,capture_output=True)
probe=pg(f"SELECT datname FROM pg_database WHERE datname='{db}';")
assert probe.returncode==0,probe.stderr
assert not probe.stdout.strip(),'Isolated DB already exists: refuse to reuse shared state'
r=pg(f'CREATE DATABASE {db};');assert r.returncode==0,r.stderr
baseline='''BEGIN;
CREATE TABLE _sqlx_migrations (
 version BIGINT PRIMARY KEY, description TEXT NOT NULL,
 installed_on TIMESTAMPTZ NOT NULL DEFAULT now(), success BOOLEAN NOT NULL,
 checksum BYTEA NOT NULL, execution_time BIGINT NOT NULL
);
'''
files=sorted((root/'apps/game-server/migrations').glob('*.sql'))
assert len(files)==30 and files[-1].name.startswith('0031_')
baseline+='\n'.join(p.read_text() for p in files)+'\nCOMMIT;\n'
r=pg(baseline,db);(out/'postgres-baseline.log').write_text(r.stdout+r.stderr)
assert r.returncode==0,'Baseline failed; inspect postgres-baseline.log (isolated DB retained)'
checks='''
SELECT 'server=' || current_setting('server_version');
SELECT 'tables=' || count(*) FROM pg_class
 WHERE oid IN ('game_character_proficiency'::regclass,
 'game_character_proficiency_receipts'::regclass,
 'game_character_proficiency_receipt_lines'::regclass);
SELECT 'guard_functions=' || count(*) FROM pg_proc WHERE proname IN
 ('game_character_proficiency_receipt_guard','game_character_proficiency_track_guard',
  'game_character_proficiency_row_guard','game_character_progression_consistency_guard');
SELECT 'fixed_paths=' || count(*) FROM pg_proc WHERE proname IN
 ('game_character_proficiency_receipt_guard','game_character_proficiency_track_guard',
  'game_character_proficiency_row_guard','game_character_progression_consistency_guard')
 AND 'search_path=public, pg_temp'=ANY(proconfig);
SELECT 'line_fk=' || pg_get_constraintdef(oid) FROM pg_constraint
 WHERE conrelid='game_character_proficiency_receipt_lines'::regclass AND contype='f';
SELECT 'per_track_index=' || pg_get_indexdef(indexrelid) FROM pg_index
 WHERE indrelid='game_character_proficiency_receipt_lines'::regclass AND indisunique;
SELECT 'canonical_arrays=' || ((array_ndims(ARRAY[NULL,0,2]::smallint[])=1
 AND array_lower(ARRAY[NULL,0,2]::smallint[],1)=1
 AND cardinality(ARRAY[NULL,0,2]::smallint[]) BETWEEN 1 AND 7
 AND array_remove(ARRAY[NULL,0,2]::smallint[],NULL::smallint)<@ARRAY[0,1,2]::smallint[])::text);
SELECT 'shape_zero_based_rejected=' || (NOT (array_lower('[0:0]={NULL}'::smallint[],1)=1))::text;
SELECT 'array_two_changed_slots=' || (((ARRAY[NULL,NULL]::smallint[])[1]
 IS DISTINCT FROM (ARRAY[0,1]::smallint[])[1])::integer
 + ((ARRAY[NULL,NULL]::smallint[])[2]
 IS DISTINCT FROM (ARRAY[0,1]::smallint[])[2])::integer)::text;
SELECT 'runtime_row_delete=' || has_table_privilege('oteryn_game_runtime','game_character_proficiency','DELETE');
SELECT 'runtime_row_update=' || has_table_privilege('oteryn_game_runtime','game_character_proficiency','UPDATE');
SELECT 'runtime_lines_update=' || has_table_privilege('oteryn_game_runtime','game_character_proficiency_receipt_lines','UPDATE');
SELECT 'runtime_guard_execute=' || has_function_privilege('oteryn_game_runtime','game_character_proficiency_track_guard()','EXECUTE');
ROLLBACK;
SELECT 'candidate_tables_after_rollback=' || count(*) FROM pg_class
 WHERE relname IN ('game_character_proficiency','game_character_proficiency_receipts','game_character_proficiency_receipt_lines');
'''
candidate=(out/'0032_character_proficiency.sql').read_text()
r=pg('BEGIN;\n'+candidate+'\n'+checks,db)
(out/'postgres-candidate-syntax.log').write_text(r.stdout+r.stderr)
assert r.returncode==0,'Candidate failed; inspect postgres-candidate-syntax.log'
for expected in ['tables=3','guard_functions=4','fixed_paths=4','canonical_arrays=true','shape_zero_based_rejected=true','array_two_changed_slots=2','runtime_row_delete=false','runtime_row_update=true','runtime_lines_update=false','runtime_guard_execute=false','candidate_tables_after_rollback=0']:
 assert expected in r.stdout,(expected,r.stdout)
report={'database':db,'container':container,'server':'17.6','baseline_migrations':len(files),'sql_sha256':hashlib.sha256(candidate.encode()).hexdigest(),'status':'ISOLATED_PG_DDL_AND_CATALOGUE_PASS','candidate_rolled_back':True,'candidate_tables_remaining':0,'shared_or_live_database_modified':False,'qualification_limit':'No writer/integrity/authority/trigger-behavior qualification; root owns actual cases. Base roles created by inherited0006 are NOLOGIN local test groups, not runtime activation.'}
(out/'postgres-qualification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
