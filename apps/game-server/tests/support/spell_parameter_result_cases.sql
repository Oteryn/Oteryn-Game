\set ON_ERROR_STOP on
SET search_path=spell_house_privacy_audit,pg_catalog;
-- Actual migration guards; immutable historical Character fixtures were seeded
-- separately. These cases do not construct current session/owner capabilities.
BEGIN;
INSERT INTO game_spell_parameter_results(game_session_id,command_id,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,cast_succeeded,created_xact_id)
VALUES('019a0000-0000-7000-8000-000000000030',100,'019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000002',1,decode(repeat('11',32),'hex'),'oteryn:spell.find-person','r20',decode('0802120408011001','hex'),sha256(decode('0802120408011001','hex')),decode('080212020802','hex'),sha256(decode('080212020802','hex')),false,'1');
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_spell_parameter_results WHERE command_id=100 AND created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'caller replaced parameter physical XID'; END IF;
END $$;
COMMIT;
DO $$ BEGIN
 BEGIN UPDATE game_spell_parameter_results SET result=decode('080212020801','hex'); RAISE EXCEPTION 'parameter result modified'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN DELETE FROM game_spell_parameter_results; RAISE EXCEPTION 'parameter result deleted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN INSERT INTO game_spell_parameter_results SELECT * FROM game_spell_parameter_results; RAISE EXCEPTION 'command result duplicate accepted'; EXCEPTION WHEN unique_violation THEN NULL; END;
 BEGIN
 INSERT INTO game_spell_parameter_results(game_session_id,command_id,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,cast_succeeded)
 SELECT game_session_id,101,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,decode(repeat('00',32),'hex'),result,result_digest,false FROM game_spell_parameter_results WHERE command_id=100;
 RAISE EXCEPTION 'tampered intent digest accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
 INSERT INTO game_spell_parameter_results(game_session_id,command_id,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,cast_succeeded)
 SELECT game_session_id,102,character_id,'019a0000-0000-7000-8000-000000000003',scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,false FROM game_spell_parameter_results WHERE command_id=100;
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'foreign World result accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
 INSERT INTO game_spell_parameter_results(game_session_id,command_id,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,cast_succeeded)
 SELECT game_session_id,103,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,true FROM game_spell_parameter_results WHERE command_id=100;
 RAISE EXCEPTION 'successful source without cost accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
 INSERT INTO game_spell_parameter_results(game_session_id,command_id,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,cast_succeeded,editor_id)
 SELECT game_session_id,104,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,false,'019a0000-0000-7000-8000-000000000051' FROM game_spell_parameter_results WHERE command_id=100;
 RAISE EXCEPTION 'failed source exposed editor'; EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
SELECT 'parameter outbox XID/immutable/duplicate/digest/World/cost/editor guards: PASS' AS result;
