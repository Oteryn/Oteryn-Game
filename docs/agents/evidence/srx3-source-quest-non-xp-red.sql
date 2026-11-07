\connect crs_native_activity_wire_1793956128963709318
\timing on
\set ON_ERROR_STOP on
\set VERBOSITY verbose
BEGIN;
CREATE TEMP TABLE srx3_source_event_gate (LIKE game_character_soul_war_receipts INCLUDING DEFAULTS INCLUDING CONSTRAINTS) ON COMMIT DROP;
CREATE TRIGGER srx3_source_event_gate BEFORE INSERT ON srx3_source_event_gate FOR EACH ROW EXECUTE FUNCTION game_soul_war_death_source_guard();
DO $probe$ BEGIN
 BEGIN
 INSERT INTO srx3_source_event_gate(character_id,occurrence,request_binding,result,source_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',decode('0101010101010101010101010101010101010101010101010101010101010101','hex'),decode('01','hex'),decode('01','hex'),decode('0000000000007000800000000000002a0000000000007000800000000000002b0000000000000001000000010000000000000001','hex'),'greed',false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v');
 RAISE EXCEPTION 'Expected native-source non-XP participant rejection did not occur';
 EXCEPTION WHEN check_violation THEN
  IF SQLERRM <> 'Soul War boss gain requires its matching native death and member receipt' THEN RAISE; END IF;
  RAISE NOTICE 'EXPECTED_SOURCE_NON_XP_REJECTION SQLSTATE=% MESSAGE=%', SQLSTATE, SQLERRM;
 END;
END $probe$;
 INSERT INTO srx3_source_event_gate(character_id,occurrence,request_binding,result,source_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',decode('0101010101010101010101010101010101010101010101010101010101010101','hex'),decode('01','hex'),decode('01','hex'),NULL,NULL,false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v');
SELECT count(*)=1 AS ordinary_non_death_control_accepted FROM srx3_source_event_gate;
ROLLBACK;
