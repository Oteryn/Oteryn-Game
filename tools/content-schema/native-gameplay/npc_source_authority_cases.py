#!/usr/bin/env python3
"""Execute12 focused current-source NPC policy/authority cases in a fresh migrated DB.

These are policy/authority tests, not proof upstream quest prerequisites were earned.
Uses only the standard library and the installed PostgreSQL client. No overlays.
"""
from pathlib import Path
import argparse,hashlib,json,os,shutil,subprocess,tempfile,time,uuid
WRAPPER_CASES = r"""DO $$ DECLARE m TEXT;BEGIN
    BEGIN PERFORM game_npc_arm_source('00000000-0000-7041-8041-000000000041',1,decode(repeat('01',32),'hex'),'{}',decode(repeat('02',32),'hex'),'{}','{}');RAISE EXCEPTION 'unarmed wrapped arm accepted';
    EXCEPTION WHEN check_violation THEN GET STACKED DIAGNOSTICS m=MESSAGE_TEXT;IF m<>'NPC source arm lacks current Node/closed native observation' THEN RAISE EXCEPTION 'unexpected arm refusal: %',m;END IF;END;
    RAISE NOTICE 'PASS wrapped source arm preserves exact Node refusal';END $$;
DO $$ BEGIN
    IF has_function_privilege('oteryn_game_runtime','game_npc_arm_source_before_quest_children(uuid,numeric,bytea,jsonb,bytea,jsonb,jsonb)','EXECUTE') OR has_function_privilege('oteryn_game_runtime','game_npc_arm_source_before_actor_observation(uuid,numeric,bytea,jsonb,bytea,jsonb,jsonb)','EXECUTE') THEN RAISE EXCEPTION 'runtime can bypass wrappers';END IF;
    RAISE NOTICE 'PASS runtime cannot bypass exact source wrappers';END $$;
DO $$ DECLARE m TEXT;BEGIN
    BEGIN INSERT INTO game_npc_source_hazard_effects(npc_transaction_id,character_id,game_session_id,command_id,source_key,program_sha256,committed_character_revision,unlocked,selected_before,selected_after,result,prize)
    VALUES('00000000-0000-7091-8091-000000000091','00000000-0000-7041-8041-000000000041','00000000-0000-7050-8050-000000000050',1,'oteryn:npc-cosmetic/gnomadness/ripptor-select',decode(repeat('02',32),'hex'),2,6,1,2,decode('0602','hex'),FALSE);
    RAISE EXCEPTION 'unarmed hazardchild accepted';EXCEPTION WHEN raise_exception THEN GET STACKED DIAGNOSTICS m=MESSAGE_TEXT;IF m<>'NPC Hazard source/Node/before/selection proof mismatch' THEN RAISE EXCEPTION 'wrong hazardlayer: %',m;END IF;END;
    RAISE NOTICE 'PASS unarmed Hazard child rejected before FK';END $$;
DO $$ DECLARE m TEXT;BEGIN
    BEGIN INSERT INTO game_npc_primal_prize_claims(character_id,npc_transaction_id)VALUES('00000000-0000-7041-8041-000000000041','00000000-0000-7091-8091-000000000091');RAISE EXCEPTION 'unarmed prize accepted';
    EXCEPTION WHEN raise_exception THEN GET STACKED DIAGNOSTICS m=MESSAGE_TEXT;IF m<>'primal prize lacks exact NPC child' THEN RAISE EXCEPTION 'wrong prizelayer: %',m;END IF;END;
    RAISE NOTICE 'PASS unarmed prize child rejected before FK';END $$;
DO $$ DECLARE m TEXT;BEGIN
    BEGIN PERFORM game_npc_source_readonly_greet_capture();RAISE EXCEPTION 'unarmed readonlygreet accepted';
    EXCEPTION WHEN check_violation THEN GET STACKED DIAGNOSTICS m=MESSAGE_TEXT;IF m<>'read-only greeting lacks exact sealed registered callback' THEN RAISE EXCEPTION 'wrong readonlylayer: %',m;END IF;END;
    RAISE NOTICE 'PASS unarmed readonly Greet capture refused';END $$;
"""

def authority_sql(repository):
    jbytes=(repository/'content/cosmetics/npc-acquisitions.json').read_bytes();j=json.loads(jbytes)
    n=next(n for n in j['npcs'] if n['npc']=='oteryn:npc.dedoras');t=next(t for t in n['transitions'] if t['id']=='battle-mage-addon-1')
    e=next(e for e in t['effects'] if e['kind']=='set_track' and e['storage'].endswith('.Questlog'));binding=next(b for b in j['bindings'] if b['storage']==e['storage']);key=binding['key'];quest=binding['quest'];expr=dict(e['value']);expr['storage']=key
    q=lambda s:"'"+s.replace("'","''")+"'";js=lambda x:q(json.dumps(x,separators=(',',':')))+'::jsonb'
    sha=hashlib.sha256(jbytes).hexdigest();source=t['source_key'];character='00000000-0000-7041-8041-000000000041';parent='00000000-0000-7091-8091-000000000091'
    foreign={key:{'quest':quest,'value':expr}}
    before={'tracks':{},'quest_source_values':{},'quest_claims':{},'now':1800000000}
    delta={'key':key,'quest':quest,'before':-1,'after':1}
    child={'quest':quest,'writes_state':quest!='oteryn:quest.npc_cosmetic_final_acquisition','changes':[{'key':key,'quest':quest,'source_before':-1,'after':1}]}
    valid={'quest_changes':[delta],'quest_children':[child],'quest_noops':[]}
    call=lambda b,p:f"game_npc_quest_plan_valid(decode('{sha}','hex'),{q(source)},'{character}'::uuid,{js(b)},{js(p)})"
    checks=[]
    checks.append((call(before,valid),'pure actualsource delta can be completely covered',True))
    p=dict(valid);p['quest_children']=[];checks.append((call(before,p),'missing foreigndelta coverage rejected',False))
    p=dict(valid);p['quest_children']=[child,child];checks.append((call(before,p),'duplicated foreigndelta coverage rejected',False))
    p=json.loads(json.dumps(valid));p['quest_children'][0]['writes_state']=not child['writes_state'];checks.append((call(before,p),'wrong ownerstorage class rejected',False))
    b=dict(before);b['tracks']={key:1};d=dict(delta,before=1,after=1);p={'quest_changes':[d],'quest_children':[],'quest_noops':[{'key':key,'quest':quest,'value':1}]};checks.append((call(b,p),'absent actualDBtrack cannot satisfy claimednoop',False))
    checks.append((f"game_npc_final_claim_projection_current('{character}'::uuid,'oteryn:npc-cosmetic/avar_tar/demon-base','oteryn:quest-progress/quest/u7_24/the_annihilator/reward','{parent}'::uuid,1,-1)",'forged actualclaim projection rejected',False))
    s=f"INSERT INTO game_npc_source_programs VALUES(decode('{sha}','hex'),decode('{jbytes.hex()}','hex')) ON CONFLICT DO NOTHING;\n"
    s+=f"INSERT INTO game_npc_source_policies(program_sha256,source_key,npc_key,track_keys,writable_tracks,allowed_grant_keys,item_costs,effects,enabled)VALUES(decode('{sha}','hex'),{q(source)},{q(n['npc'])},ARRAY[{q(key)}], '{{}}',ARRAY[]::text[],'[]',{js(t['effects'])},TRUE) ON CONFLICT DO NOTHING;\n"
    s+=f"INSERT INTO game_npc_source_quest_policies VALUES(decode('{sha}','hex'),{q(source)},{js(foreign)}) ON CONFLICT DO NOTHING;\n"
    for call_,label,want in checks:s+=f"DO $$ BEGIN IF {call_} IS DISTINCT FROM {str(want).upper()} THEN RAISE EXCEPTION {q(label)}; END IF;RAISE NOTICE 'PASS {label}';END $$;\n"
    s+=f"""DO $$ DECLARE detail TEXT;BEGIN
        BEGIN INSERT INTO game_npc_quest_children(parent_transaction_id,quest_key,character_id,pinned_content_revision,base_definition_hash,evaluation_definition_hash,program_sha256,plan_sha256,committed_revision,transition_keys,native_operations,track_changes,completes)
        VALUES('{parent}',{q(quest)},'{character}','content-1',decode(repeat('01',32),'hex'),decode(repeat('01',32),'hex'),decode('{sha}','hex'),decode(repeat('02',32),'hex'),2,ARRAY['actual-source-policy-test'],ARRAY[]::text[],{js([delta])},FALSE);
        RAISE EXCEPTION 'unarmed child insert accepted';
        EXCEPTION WHEN check_violation THEN GET STACKED DIAGNOSTICS detail=MESSAGE_TEXT;
        IF detail<>'Quest child lacks exact current source parent' THEN RAISE EXCEPTION 'wrong failure layer: %',detail;END IF;
        END;RAISE NOTICE 'PASS unarmed child rejected by privateparent before FK';END $$;\n"""
    s += WRAPPER_CASES
    return s

def main():
    a=argparse.ArgumentParser(description=__doc__);a.add_argument('--repository',type=Path);a.add_argument('--log',type=Path);a.add_argument('--case-overlay',type=Path,action='append',default=[],help='replace built-in cases with current-migration focused case files');a.add_argument('--expected-cases',type=int,default=12);args=a.parse_args()
    repository=(args.repository or Path(__file__).resolve().parents[3]).resolve()
    admin=os.environ.get('OTERYN_TEST_POSTGRES_ADMIN_URL');psql=shutil.which('psql')
    if not admin or not psql:a.error('source repository environment for installed psql and PostgreSQL adminURL')
    migrations=sorted((repository/'apps/game-server/migrations').glob('*.sql'))
    if not migrations:a.error('server migrations missing')
    database='srv7_npc_authority_'+uuid.uuid4().hex;quoted='"'+database+'"'
    script='CREATE DATABASE '+quoted+';\n\\connect '+database+'\nBEGIN;\nCREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n'
    cases='\n'.join(p.read_text() for p in args.case_overlay) if args.case_overlay else authority_sql(repository)
    script+='\n'.join(p.read_text() for p in migrations)+'\nCOMMIT;\nBEGIN;\n'+cases+'\nROLLBACK;\n'
    started=time.perf_counter()
    try:
        with tempfile.NamedTemporaryFile('w',suffix='.sql',encoding='utf-8') as f:
            f.write(script);f.flush();result=subprocess.run([psql,admin,'-X','-v','ON_ERROR_STOP=1','-f',f.name],capture_output=True,text=True,check=False)
        output=result.stdout+result.stderr
        if args.log:args.log.write_text(output)
        if result.returncode:print('\n'.join(output.splitlines()[-40:]));return result.returncode
        count=output.count('NOTICE:  PASS ')
        if count!=args.expected_cases:raise ValueError(f'expected{args.expected_cases} executed cases; actual{count}')
        print(f'SRV7_CURRENT_NPC_AUTHORITY_PASSED cases={count} elapsed={time.perf_counter()-started:.3f}s');return 0
    finally:
        dropped=subprocess.run([psql,admin,'-X','-v','ON_ERROR_STOP=1','-c','DROP DATABASE IF EXISTS '+quoted+' WITH (FORCE)'],capture_output=True,text=True,check=False)
        if dropped.returncode:raise RuntimeError('scratch database cleanup failed: '+dropped.stderr)
if __name__=='__main__':raise SystemExit(main())
