import os,subprocess,json,hashlib,re,time
from pathlib import Path
ROOT=Path('/workspace/Oteryn-WP-Durable')
OUT=Path('/workspace/scratch/weapon-proficiency-execution/prof1-sql')
DB='oteryn_prof1_semantics_20261001_'+str(os.getpid())
CONTAINER='oteryn-wp-pg-20261001'
env={k:v for k,v in os.environ.items() if k not in ('DOCKER_HOST','DOCKER_CONTEXT','DOCKER_TLS','DOCKER_TLS_VERIFY','DOCKER_CERT_PATH')}
def pg(sql,db=None):
 db=db or DB
 return subprocess.run(['docker','--host=unix:///var/run/docker.sock','exec','-i',CONTAINER,'psql','-X','-U','oteryn_test_admin','-d',db,'-v','ON_ERROR_STOP=1','-v','VERBOSITY=verbose','-At'],input=sql,text=True,capture_output=True,env=env)
def must(sql,db=None):
 r=pg(sql,db)
 if r.returncode: raise RuntimeError(r.stderr)
 return r.stdout
CHAR="'10000000-0000-7000-8000-000000000001'"
ACCOUNT="'20000000-0000-7000-8000-000000000001'"
WORLD="'30000000-0000-7000-8000-000000000001'"
def occ(n): return "'40000000-0000-7000-8000-"+f'{n:012d}'+"'"
def ins(table,d): return f"INSERT INTO {table} ({','.join(d)}) VALUES ({','.join(map(str,d.values()))});\n"
CONTEXT={k:"'v1'" for k in ['profile_revision','ruleset_revision','content_revision','simulation_revision','evidence_revision','declaration_revision','policy_revision','reward_revision']}
SEED=ins('game_character_account_guards',dict(account_id=ACCOUNT))+ins('game_character_roots',dict(character_id=CHAR,account_id=ACCOUNT,world_id=WORLD,lifecycle=1,character_revision=1,profile_revision="'v1'",ruleset_revision="'v1'",content_revision="'v1'",starter_template_revision="'v1'",name="'Sql Fixture'"))+ins('game_character_progression_state',dict(character_id=CHAR,character_revision=1,level=1,total_experience=0,**CONTEXT))+'SET CONSTRAINTS ALL IMMEDIATE; SET CONSTRAINTS ALL DEFERRED;\n'
def header(n=2,cause='training',**changes):
 cause=cause.strip("'")
 d=dict(proficiency_occurrence_id=occ(n),character_id=CHAR,original_character_revision=n-1,committed_character_revision=n,cause="'"+cause+"'",command_binding="decode('01','hex')",policy_digest="decode(repeat('01',32),'hex')",level_before=1,level_after=1,experience_before=0,experience_after=0,**CONTEXT,committed_at=100)
 d.update(changes); return ins('game_character_proficiency_receipts',d)
def line(n=2,cause='training',**changes):
 cause=cause.strip("'")
 d=dict(proficiency_occurrence_id=occ(n),character_id=CHAR,committed_character_revision=n,cause="'"+cause+"'",item_key="'oteryn:item.sword'",definition_key_before="'oteryn:proficiency.sword'",definition_revision_before="'d1'",definition_key_after="'oteryn:proficiency.sword'",definition_revision_after="'d1'",progress_before=0,progress_after=1,selections_before='ARRAY[NULL,NULL]::smallint[]',selections_after='ARRAY[NULL,NULL]::smallint[]')
 if cause=='perk_selection': d.update(progress_after=0,selections_after='ARRAY[0,NULL]::smallint[]')
 if cause=='migration': d.update(progress_after=0,definition_revision_after="'d2'")
 d.update(changes);return ins('game_character_proficiency_receipt_lines',d)
def row(n=2,**changes):
 d=dict(character_id=CHAR,item_key="'oteryn:item.sword'",definition_key="'oteryn:proficiency.sword'",definition_revision="'d1'",progress=1,selections='ARRAY[NULL,NULL]::smallint[]',committed_character_revision=n,last_proficiency_occurrence_id=occ(n))
 d.update(changes);return ins('game_character_proficiency',d)
def advance(n=2,**changes):
 d=dict(character_revision=n);d.update(changes)
 return f'UPDATE game_character_roots SET character_revision={n} WHERE character_id={CHAR};\n'+f"UPDATE game_character_progression_state SET {','.join(k+'='+str(v) for k,v in d.items())} WHERE character_id={CHAR};\n"
def training(n=2,**lc):
 return header(n)+line(n,**lc)+(row(n) if n==2 else f"UPDATE game_character_proficiency SET progress={n-1},committed_character_revision={n},last_proficiency_occurrence_id={occ(n)} WHERE character_id={CHAR};\n")+advance(n)
CASES=[]
def case(name,body,state=None,message=None): CASES.append(dict(name=f'{len(CASES)+1:03d}_'+name,sql='BEGIN;\n'+SEED+body+'\nSET CONSTRAINTS ALL IMMEDIATE;\nROLLBACK;\n',expected_state=state,expected_message=message))
case('seed_independent_authority','')
case('training_valid',training())
case('selection_first_valid',header(cause='perk_selection')+line(cause='perk_selection')+row(progress=0,selections='ARRAY[0,NULL]::smallint[]')+advance())
case('migration_first_valid',header(cause='migration')+line(cause='migration')+row(progress=0,definition_revision="'d2'")+advance())
case('second_training_valid',training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+training(3,progress_before=1,progress_after=2))
# Header CHECK/FK boundaries: only changed input, all other facts valid.
for field,value in [('proficiency_occurrence_id',"'40000000-0000-4000-8000-000000000002'"),('original_character_revision',0),('committed_character_revision',3),('cause',"'invalid'"),('command_binding',"decode('','hex')"),('command_binding',"decode(repeat('01',1025),'hex')"),('policy_digest',"decode(repeat('01',31),'hex')"),('level_after',2),('experience_after',1),('committed_at',-1)]+[(k,"'invalid revision'") for k in CONTEXT]:
 case('header_'+field+'_'+str(value)[:20],header(**{field:value})+line()+row()+advance(),'23514')
case('header_unknown_character',header(character_id="'10000000-0000-7000-8000-000000000099'")+'SELECT 1;','23503')
case('header_without_lines',header()+advance(),'23514','invalid changed-track line count')
case('header_future_root',header()+line()+row(),'23514')
case('root_without_receipt',advance(),'23514','revision/receipt chain')
case('state_missing_successor',header()+line()+row()+f'UPDATE game_character_roots SET character_revision=2 WHERE character_id={CHAR};','23514','revision/receipt chain')
case('initial_proficiency_row',row(),'23514')
case('selection_two_lines',header(cause='perk_selection')+line(cause='perk_selection')+line(cause='perk_selection',item_key="'oteryn:item.axe'")+row(progress=0,selections='ARRAY[0,NULL]::smallint[]')+row(item_key="'oteryn:item.axe'",progress=0,selections='ARRAY[0,NULL]::smallint[]')+advance(),'23514','invalid changed-track line count')
# Immediate line FK binding substitutions.
for field,value in [('proficiency_occurrence_id',occ(99)),('character_id',"'10000000-0000-7000-8000-000000000099'"),('committed_character_revision',3),('cause',"'migration'")]:
 # migration substitution needs direction-valid values to reach header FK
 changes={field:value}
 if field=='cause': changes.update(progress_after=0,definition_revision_after="'d2'")
 case('line_header_binding_'+field,header()+line(**changes)+row()+advance(),'23503')
for field,value in [('item_key',"'other:item.sword'"),('item_key',"'oteryn:item.'"),('definition_key_before',"'other:proficiency.sword'"),('definition_key_before',"'oteryn:proficiency.'"),('definition_revision_before',"'bad revision'"),('definition_key_after',"'oteryn:proficiency.axe'"),('progress_before',-1)]:
 changes={field:value}
 if field=='definition_key_before': changes['definition_key_after']=value
 case('line_source_'+field+'_'+str(value)[:25],header()+line(**changes)+row()+advance(),'23514')
for cause,changes in [('training',dict(progress_after=0)),('training',dict(definition_revision_after="'d2'")),('training',dict(selections_after='ARRAY[0,NULL]::smallint[]')),('perk_selection',dict(selections_after='ARRAY[NULL,NULL]::smallint[]')),('perk_selection',dict(selections_after='ARRAY[0,1]::smallint[]')),('perk_selection',dict(progress_after=1)),('perk_selection',dict(definition_revision_after="'d2'")),('perk_selection',dict(selections_after='ARRAY[0,NULL,NULL]::smallint[]')),('migration',dict(progress_after=1)),('migration',dict(definition_revision_after="'d1'"))]:
 case('direction_'+cause+'_'+next(iter(changes)),header(cause=cause)+line(cause=cause,**changes)+row()+advance(),'23514','cause_direction')
for field in ['selections_before','selections_after']:
 for tag,arr,state in [('null','NULL','23502'),('empty','ARRAY[]::smallint[]','23514'),('zero_lower',"'[0:1]={NULL,NULL}'::smallint[]",'23514'),('two_lower',"'[2:3]={NULL,NULL}'::smallint[]",'23514'),('two_dimensions','ARRAY[[NULL,NULL],[NULL,NULL]]::smallint[]','23514'),('eight','array_fill(NULL::smallint,ARRAY[8])','23514'),('invalid3','ARRAY[3,NULL]::smallint[]','23514'),('negative','ARRAY[-1,NULL]::smallint[]','23514')]:
  case('line_array_'+field+'_'+tag,header()+line(selections_before=arr,selections_after=arr)+row()+advance(),state)
for tag,arr in [('one','ARRAY[NULL]::smallint[]'),('seven','array_fill(NULL::smallint,ARRAY[7])')]:
 case('array_'+tag+'_valid',header()+line(selections_before=arr,selections_after=arr)+row(selections=arr)+advance())
case('array_choice2_valid',header(cause='perk_selection')+line(cause='perk_selection',selections_after='ARRAY[2,NULL]::smallint[]')+row(progress=0,selections='ARRAY[2,NULL]::smallint[]')+advance())
case('first_nonzero_progress',header()+line(progress_before=1,progress_after=2)+row(progress=2)+advance(),'23514','first proficiency line')
case('first_prefilled_choices',header()+line(selections_before='ARRAY[0,NULL]::smallint[]',selections_after='ARRAY[0,NULL]::smallint[]')+row(selections='ARRAY[0,NULL]::smallint[]')+advance(),'23514','first proficiency line')
case('line_without_row',header()+line()+advance(),'23514','row does not equal')
case('row_without_line',header()+row()+advance(),'23514')
for field,value in [('definition_key',"'oteryn:proficiency.axe'"),('definition_revision',"'d2'"),('progress',2),('selections','ARRAY[0,NULL]::smallint[]'),('committed_character_revision',3),('last_proficiency_occurrence_id',occ(99))]:
 case('row_tip_'+field,header()+line()+row(**{field:value})+advance(),'23514','row does not equal')
for field,value in [('definition_key_before',"'oteryn:proficiency.axe'"),('definition_revision_before',"'d2'"),('progress_before',0),('selections_before','ARRAY[0,NULL]::smallint[]')]:
 changes=dict(progress_before=1,progress_after=2);changes[field]=value
 if field=='definition_key_before': changes['definition_key_after']=value
 if field=='definition_revision_before': changes['definition_revision_after']=value
 if field=='selections_before': changes['selections_after']=value
 case('continuity_'+field,training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+training(3,**changes),'23514','before values do not follow')
for table in ['game_character_proficiency_receipts','game_character_proficiency_receipt_lines']:
 for op in ['UPDATE','DELETE','TRUNCATE']:
  sql=f'UPDATE {table} SET cause=cause;' if op=='UPDATE' else f'{op} '+('FROM ' if op=='DELETE' else '')+table+(' CASCADE;' if op=='TRUNCATE' else ';')
  case('immutable_'+table+'_'+op,training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+sql,'23514')
for tag,sql in [('delete','DELETE FROM game_character_proficiency;'),('rekey',"UPDATE game_character_proficiency SET item_key='oteryn:item.axe';"),('definition',"UPDATE game_character_proficiency SET definition_key='oteryn:proficiency.axe';"),('decrease','UPDATE game_character_proficiency SET progress=0;'),('truncate','TRUNCATE game_character_proficiency;'),('unexplained_update','UPDATE game_character_proficiency SET progress=2;')]:
 case('current_'+tag,training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+sql,'23514')


# All inherited shared-guard arms: independently valid receipts and owned projections.
def legacy(kind,n=2,xp=0,**changes):
 d=dict(character_id=CHAR,command_binding="decode('01','hex')",original_character_revision=n-1,committed_character_revision=n,level_before=1,level_after=1,experience_before=xp,experience_after=xp,**CONTEXT,committed_at=100)
 extra='';after={}
 if kind=='xp':
  table='game_character_xp_receipts';d.update(reward_occurrence_id=occ(n),policy_digest="decode(repeat('01',32),'hex')",experience_after=xp+1,experience_awarded=1);after={'total_experience':xp+1}
 elif kind=='death':
  table='game_character_death_receipts';d.update(death_occurrence_id=occ(n),policy_digest="decode(repeat('01',32),'hex')",experience_lost=0,blessings_before='ARRAY[]::text[]',blessings_after='ARRAY[]::text[]',lost_item_ids='ARRAY[]::uuid[]',death_world_id=WORLD,death_channel_id=WORLD,death_spatial_position="decode('01','hex')",death_map_revision="'v1'",respawn_position="decode('01','hex')",death_policy_revision="'v1'")
  extra=ins('game_character_pending_respawns',dict(character_id=CHAR,death_occurrence_id=occ(n),respawn_position="decode('01','hex')"))
 elif kind=='stance':
  table='game_character_stance_receipts';d.update(stance_occurrence_id=occ(n),policy_digest="decode(repeat('01',32),'hex')",stance_before='NULL',stance_after="'stance1'")
  extra=ins('game_character_stance',dict(character_id=CHAR,stance_key="'stance1'",committed_character_revision=n,last_stance_occurrence_id=occ(n)))
 elif kind=='bestiary':
  table='game_character_bestiary_kill_receipts';d.update(bestiary_occurrence_id=occ(n),race_digest="decode(repeat('01',32),'hex')",race_key="'oteryn:creature.rat'",race_definition_revision="'v1'",final_kill_threshold=10,kill_count_before=0,kill_count_after=1)
  extra=ins('game_character_bestiary_progress',dict(character_id=CHAR,race_key="'oteryn:creature.rat'",kill_count=1,committed_character_revision=n,last_bestiary_occurrence_id=occ(n)))
 elif kind=='charm':
  table='game_character_charm_receipts';d.update(charm_occurrence_id=occ(n),command_binding="decode(repeat('01',33),'hex')",catalogue_digest="decode(repeat('01',32),'hex')",catalogue_revision="'v1'",command_kind=1,charm_key="'oteryn:charm.test'",charm_category=1,stage_before=0,stage_after=1,stage_cost=1)
  extra=ins('game_character_charm_unlocks',dict(character_id=CHAR,charm_key="'oteryn:charm.test'",unlocked_stage=1,committed_character_revision=n,last_charm_occurrence_id=occ(n)))
 elif kind=='monk':
  table='game_character_monk_state_receipts';d.update(monk_state_occurrence_id=occ(n),command_binding="decode(repeat('01',33),'hex')",harmony_before=0,harmony_after=1,serene_forced_remaining_micros_before=0,serene_forced_remaining_micros_after=0);after={'harmony':1}
 elif kind=='build':
  table='game_character_build_receipts';d.update(build_occurrence_id=occ(n),policy_digest="decode(repeat('01',32),'hex')",cause="'training'",vocation_before="'none'",vocation_after="'none'",magic_level_before=0,magic_level_after=0,mana_spent_before=0,mana_spent_after=1)
  build=dict(character_id=CHAR,vocation="'none'",magic_level=0,mana_spent=1,committed_character_revision=n,last_build_occurrence_id=occ(n))
  for skill in ['fist','club','sword','axe','distance','shielding','fishing']:
   for suffix,v in [('level',10),('tries',0)]:
    d[skill+'_'+suffix+'_before']=v;d[skill+'_'+suffix+'_after']=v;build[skill+'_'+suffix]=v
  extra=ins('game_character_build_state',build)
 else: raise ValueError(kind)
 d.update(changes)
 return ins(table,d)+extra+advance(n,**after)
for kind in ['xp','death','stance','bestiary','charm','monk','build']:
 case('legacy_'+kind+'_valid',legacy(kind))
 case('legacy_'+kind+'_tip_context_substitution',legacy(kind,content_revision="'v2'"),'23514','revision/receipt chain')
 case('legacy_'+kind+'_duplicate_wp_revision',legacy(kind)+header()+line()+row(),'23514','revision/receipt chain')
 case('wp_then_'+kind+'_valid',training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+legacy(kind,3))
# XP -> WP binds the immutable receipt's before facts to independently committed XP.
case('xp_then_wp_valid',legacy('xp')+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+header(3,experience_before=1,experience_after=1)+line(3)+row(3)+advance(3))
case('xp_then_wp_discontinuous',legacy('xp')+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+header(3)+line(3)+row(3)+advance(3),'23514','revision/receipt chain')
# Current liveness/stale history: an old header accepts no late new Item line.
case('append_line_to_old_receipt',training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+legacy('xp',3)+line(item_key="'oteryn:item.axe'")+row(item_key="'oteryn:item.axe'") ,'23514','older or future receipt')

# Current row shape rejects independently, before deferred row projection matching.
for tag,arr,state in [('null','NULL','23502'),('empty','ARRAY[]::smallint[]','23514'),('zero_lower',"'[0:1]={NULL,NULL}'::smallint[]",'23514'),('two_lower',"'[2:3]={NULL,NULL}'::smallint[]",'23514'),('two_dimensions','ARRAY[[NULL,NULL],[NULL,NULL]]::smallint[]','23514'),('eight','array_fill(NULL::smallint,ARRAY[8])','23514'),('invalid3','ARRAY[3,NULL]::smallint[]','23514'),('negative','ARRAY[-1,NULL]::smallint[]','23514')]:
 case('row_array_'+tag,header()+line()+row(selections=arr)+advance(),state)
case('runtime_training_valid','SET LOCAL ROLE oteryn_game_runtime;'+training())
case('runtime_selection_valid',"SET LOCAL ROLE oteryn_game_runtime;"+header(cause='perk_selection')+line(cause='perk_selection')+row(progress=0,selections='ARRAY[0,NULL]::smallint[]')+advance())
case('runtime_migration_valid',"SET LOCAL ROLE oteryn_game_runtime;"+header(cause='migration')+line(cause='migration')+row(progress=0,definition_revision="'d2'")+advance())
case('training_two_tracks_valid',header()+line()+line(item_key="'oteryn:item.axe'")+row()+row(item_key="'oteryn:item.axe'")+advance())
case('row_only_definition_revision',training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+"UPDATE game_character_proficiency SET definition_revision='d2';",'23514','row does not equal')
case('row_only_occurrence',training()+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+f"UPDATE game_character_proficiency SET last_proficiency_occurrence_id={occ(99)};",'23514','row does not equal')

for kind in ['death','stance','bestiary','charm','monk','build']:
 case(kind+'_then_wp_valid',legacy(kind)+'SET CONSTRAINTS ALL IMMEDIATE;SET CONSTRAINTS ALL DEFERRED;'+header(3)+line(3)+row(3)+advance(3))
case('wp_illegal_harmony_change',header()+line()+row()+advance(harmony=1),'23514','Harmony and the forced Serene')
case('wp_illegal_serene_change',header()+line()+row()+advance(serene_forced_remaining_micros=1),'23514','Harmony and the forced Serene')
case('duplicate_item_in_receipt',header()+line()+line()+row()+advance(),'23505')
case('duplicate_receipt_occurrence',header()+header()+line()+row()+advance(),'23505')
case('row_progress_i64_max_valid',header()+line(progress_after=9223372036854775807)+row(progress=9223372036854775807)+advance())
case('row_progress_i64_overflow',header()+line(progress_after=9223372036854775808)+row(progress=9223372036854775808)+advance(),'22003')
# All eight context fields must match current independently declared progression.
for context in CONTEXT:
 case('header_current_context_'+context,header(**{context:"'v2'"})+line()+row()+advance(),'23514','revision/receipt chain')

def main():
 candidate=(OUT/'0032_character_proficiency.sql').read_text();digest=hashlib.sha256(candidate.encode()).hexdigest()
 assert digest=='9bde3bfcc946e18600913e295a983998321f4fdfd9dd244bf5b28a28e04142b5'
 must('CREATE DATABASE '+DB+';', 'postgres')
 files=sorted((ROOT/'apps/game-server/migrations').glob('*.sql'));assert len(files)==30
 baseline='BEGIN; CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY,description TEXT NOT NULL, installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),success BOOLEAN NOT NULL,checksum BYTEA NOT NULL,execution_time BIGINT NOT NULL);\n'+'\n'.join(f.read_text() for f in files)+'\nCOMMIT;'
 must(baseline);must('BEGIN;\n'+candidate+'\nCOMMIT;')
 run_cases()
def run_cases():
 result=[]
 for i,c in enumerate(CASES):
  r=pg(c['sql']);state=None
  if r.returncode:
   m=re.search(r'(?:ERROR|FATAL):\s+([A-Z0-9]{5}):',r.stderr);state=m.group(1) if m else 'UNKNOWN'
  passed=(state==c['expected_state'] and (not c['expected_message'] or c['expected_message'] in r.stderr))
  result.append(dict(name=c['name'],passed=passed,expected_state=c['expected_state'],actual_state=state,expected_message=c['expected_message'],error=r.stderr.strip()))
  if not passed: print('FAIL',c['name'],state,r.stderr)
  if not passed and c['expected_state'] is None: raise RuntimeError('Positive control failed: '+c['name']+' '+r.stderr)
 (OUT/'semantic-cases.json').write_text(json.dumps(CASES,indent=2)+'\n')
 (OUT/'semantic-results.json').write_text(json.dumps(dict(database=DB,server=must('SHOW server_version;').strip(),candidate_sha256=hashlib.sha256((OUT/'0032_character_proficiency.sql').read_bytes()).hexdigest(),cases=len(result),passed=sum(x['passed'] for x in result),results=result),indent=2)+'\n')
 print(json.dumps(dict(database=DB,cases=len(result),passed=sum(x['passed'] for x in result))))
if __name__=='__main__': main()
