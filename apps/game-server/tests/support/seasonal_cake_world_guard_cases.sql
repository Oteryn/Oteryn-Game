-- Focused SRV7 Cake World SQL security and restart cases.
-- Run only through seasonal_cake_world_guard_cases.py: it allocates and drops a
-- fresh database. Administrative replica-mode writes below seed historical
-- predecessors; every behavioral operation uses the actual runtime SQL role.
-- Fixed 11/22/etc byte strings are isolated test fixture keys, never credentials.
\timing on
BEGIN;
INSERT INTO game_node_bootstrap_authorizations(authorization_digest,launch_binding,issued_at)
 VALUES(sha256(decode(repeat('11',32),'hex')),'srv7-cake-world-guard',1);
SELECT game_node_register(decode(repeat('11',32),'hex'),'srv7-cake-world-guard','00000000-0000-7000-8000-000000000001');
SET LOCAL session_replication_role=replica;
INSERT INTO game_runtime_scope_assignments(scope_key,world_id,channel_id,ownership_generation,state,holder_node_id,holder_registration_revision,source_revision,decision_identity,operation_key,decided_at)
 VALUES('\x01'::bytea||uuid_send('00000000-0000-7000-8000-000000000042')||uuid_send('00000000-0000-7000-8000-000000000043'),
 '00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,1,'00000000-0000-7000-8000-000000000001',1,1,'srv7-cake-fixture',decode(repeat('22',32),'hex'),1);
INSERT INTO game_runtime_scope_assignment_receipts(operation_key,command,scope_key,ownership_generation,state,holder_node_id,holder_registration_revision,source_revision,decision_identity,decided_at)
 SELECT operation_key,'\x01',scope_key,ownership_generation,state,holder_node_id,holder_registration_revision,source_revision,decision_identity,decided_at FROM game_runtime_scope_assignments;
UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water=1;
SET LOCAL session_replication_role=origin;
COMMIT;
SELECT floor(extract(epoch FROM '2026-02-21 06:00:00 Europe/Berlin'::timestamptz))::bigint AS event \gset
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+1);
INSERT INTO game_seasonal_cake_worlds VALUES('00000000-0000-7000-8000-000000000042',:event,:event+518400,
 jsonb_build_object('stage',1,'substances',0,'golems',0,'layer',1,'layer_progress',0,'window_start',NULL,'window_end',NULL,'bonus_from',NULL,'bonus_until',NULL,'revision',1));
-- ON CONFLICT DO NOTHING must not create a fictitious second World write.
INSERT INTO game_seasonal_cake_worlds SELECT * FROM game_seasonal_cake_worlds ON CONFLICT DO NOTHING;
COMMIT;

DO $$ DECLARE before JSONB:='{"schema":1,"conditions":[]}'::jsonb; after JSONB;
BEGIN
 after:='{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb;
 IF game_cake_food_conditions_valid(before,after) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'first food failed'; END IF;
 RAISE NOTICE 'PASS first native Cake food adds240s';
 IF game_cake_food_conditions_valid(before,jsonb_set(after,'{conditions,0,ends_at}','480000000')) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'double food accepted'; END IF;
 RAISE NOTICE 'PASS double food rejected';
 IF game_cake_food_conditions_valid(before,jsonb_set(after,'{conditions,0,definition,values,FoodRegeneration,added_ms}','240001')) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'wrong food accepted'; END IF;
 RAISE NOTICE 'PASS wrong food definition rejected';
 IF game_cake_food_conditions_valid(after,jsonb_set(after,'{conditions,0,ends_at}','480000000')) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'second food failed'; END IF;
 RAISE NOTICE 'PASS second immediate native food adds240s';
 IF game_cake_food_conditions_valid(jsonb_set(after,'{conditions,0,ends_at}','960000000'),jsonb_set(after,'{conditions,0,ends_at}','1200000000')) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'full food accepted'; END IF;
 RAISE NOTICE 'PASS fifth immediate food refused atomically';
 IF game_cake_food_conditions_valid(jsonb_set(after,'{conditions,0,ends_at}','900000000'),jsonb_set(after,'{conditions,0,ends_at}','1140000000')) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'aged food refused'; END IF;
 RAISE NOTICE 'PASS fifth food after60seconds accepted';
 IF game_cake_food_conditions_valid(before,after||'{"conditions":null}'::jsonb) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'null food accepted'; END IF;
 RAISE NOTICE 'PASS null food rejected';
 IF game_cake_food_conditions_valid(before||'{"conditions":[{"definition":{"key":"other","values":{}}}]}'::jsonb,after) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'lost nonfood accepted'; END IF;
 RAISE NOTICE 'PASS Cake food preserves every existing nonfood condition';
 IF game_cake_edible_goal()<>1066 THEN RAISE EXCEPTION 'goal drift'; END IF;
 RAISE NOTICE 'PASS actual source footprint goal1066';
END $$;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+600);
INSERT INTO game_seasonal_cake_stage_polls(world_id,event_start,poll_slot,observed_at,opened_bridge)
 VALUES('00000000-0000-7000-8000-000000000042',:event,1,:event+600,FALSE);
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS due unready poll commits without fictitious World revision'; END $$;
BEGIN;
SET LOCAL session_replication_role=replica;
UPDATE game_seasonal_cake_worlds SET state=state||'{"substances":200,"golems":200}'::jsonb;
COMMIT;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+1200);
INSERT INTO game_seasonal_cake_stage_polls(world_id,event_start,poll_slot,observed_at,opened_bridge)
 VALUES('00000000-0000-7000-8000-000000000042',:event,2,:event+1200,TRUE);
UPDATE game_seasonal_cake_worlds SET state=state||'{"stage":2,"revision":2}'::jsonb;
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS bridge opens only on ready scheduled source poll'; END $$;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+3000);
INSERT INTO game_seasonal_cake_windows(world_id,event_start,window_start,layer,observed_at)
 VALUES('00000000-0000-7000-8000-000000000042',:event,:event+3000,1,:event+3000);
UPDATE game_seasonal_cake_worlds SET state=state||jsonb_build_object('window_start',:event+3000,'window_end',:event+4800,'revision',3);
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS first eating window anchored1800seconds after bridge'; END $$;

-- Complete live Node/Character/Content predecessors are administrative fixture setup.
BEGIN;
SET LOCAL session_replication_role=replica;
INSERT INTO game_character_account_guards(account_id) VALUES('00000000-0000-7000-8000-000000000045');
INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name)
 VALUES('00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000045','00000000-0000-7000-8000-000000000042',1,1,'r1','r1','r1','r1','Cake Tester');
INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,
 control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,session_state)
 VALUES('00000000-0000-7000-8000-000000000040','00000000-0000-7000-8000-000000000045','00000000-0000-7000-8000-000000000041',
 '00000000-0000-7000-8000-000000000042',1,'00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,0,1,1,1,1,2);
INSERT INTO game_durability_admission_character_guards(character_id,account_id,world_id,eligible,lease_generation,holder_game_session_id,publication_revision,source_authority,source_revision,decision_identity,source_observed_at,clock_uncertainty_seconds,change_json)
 VALUES('00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000045','00000000-0000-7000-8000-000000000042',TRUE,1,'00000000-0000-7000-8000-000000000040',1,'fixture',1,'fixture',0,0,'{}');
INSERT INTO game_durability_admission_account_guards(account_id,presence_character_id,holder_game_session_id,publication_revision,source_authority,source_revision,decision_identity,source_observed_at,clock_uncertainty_seconds,change_json)
 VALUES('00000000-0000-7000-8000-000000000045','00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',1,'fixture',1,'fixture',0,0,'{}');
INSERT INTO game_content_activations(world_id,channel_id,activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest,issued_by,issued_at)
 VALUES('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,decode(repeat('33',32),'hex'),decode(repeat('44',32),'hex'),decode(repeat('55',32),'hex'),'fixture',1);
COMMIT;

BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+3001);
DO $$ DECLARE e BIGINT; before JSONB:='{"schema":1,"conditions":[]}'::jsonb;
BEGIN SELECT event_start INTO e FROM game_seasonal_cake_worlds;
 BEGIN
  PERFORM game_cake_arm_food_observation('00000000-0000-7000-8000-000000000001',1,decode(repeat('99',32),'hex'),
   '00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',1,1,decode(repeat('aa',16),'hex'),1,1,decode(repeat('33',32),'hex'),before,'{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb);
  RAISE EXCEPTION 'wrong Node observation accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS food observation re-proves actual Node secret'; END;
 PERFORM game_cake_arm_food_observation('00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),
  '00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',1,1,decode(repeat('aa',16),'hex'),1,1,decode(repeat('33',32),'hex'),before,'{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb);
 RAISE NOTICE 'PASS food observation binds actual current actor lease';
 BEGIN
  PERFORM game_cake_arm_food_observation('00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),
   '00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',2,1,decode(repeat('aa',16),'hex'),1,1,decode(repeat('33',32),'hex'),before,'{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb);
  RAISE EXCEPTION 'replaced actor observation accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS observed predecessor immutable in same transaction'; END;
 BEGIN
  DELETE FROM game_cake_food_observations;
  RAISE EXCEPTION 'private observation delete accepted';
 EXCEPTION WHEN insufficient_privilege THEN RAISE NOTICE 'PASS runtime cannot mutate private food observation'; END;
END $$;
ROLLBACK;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+3001);
SELECT game_cake_apply_terrain_tool_transform('00000000-0000-7000-8000-000000000040',1,'00000000-0000-7000-8000-000000000041',NULL,'\x01',
 '00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',decode(repeat('00',32)||'000082167d430701','hex'),
 'native-test',decode(repeat('33',32),'hex'),0,0,12085,0,'{}','{}');
UPDATE game_seasonal_cake_worlds SET state=state||'{"layer_progress":1,"revision":4}'::jsonb;
INSERT INTO game_seasonal_cake_tiles(world_id,event_start,layer,x,y,floor,appearance,window_start,removed)
 VALUES('00000000-0000-7000-8000-000000000042',:event,1,33302,32067,7,12085,:event+3000,TRUE);
INSERT INTO game_seasonal_cake_bites(character_id,world_id,event_start,window_start,bites,last_bite)
 VALUES('00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000042',:event,:event+3000,1,:event+3001);
INSERT INTO game_seasonal_cake_bite_receipts(game_session_id,command_id,character_id,world_id,channel_id,event_start,layer,x,y,floor,window_start,appearance,placement,world_revision_before,world_revision_after)
 VALUES('00000000-0000-7000-8000-000000000040',1,'00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000042',
 '00000000-0000-7000-8000-000000000043',:event,1,33302,32067,7,:event+3000,12085,decode(repeat('00',32)||'000082167d430701','hex'),3,4);
DO $$ BEGIN BEGIN
INSERT INTO game_seasonal_cake_food_uses(game_session_id,command_id,character_id,world_id,event_start,content_digest,actor_identity,movement_revision,state_revision,character_lease_generation,conditions_before,conditions_after,conditions_sha256)
 SELECT b.game_session_id,b.command_id,b.character_id,b.world_id,b.event_start,decode(repeat('33',32),'hex'),decode(repeat('aa',16),'hex'),1,1,1,
 '{"schema":1,"conditions":[]}',after,sha256(convert_to(after::text,'UTF8')) FROM game_seasonal_cake_bite_receipts b CROSS JOIN LATERAL
 (SELECT '{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb after) a;
 RAISE EXCEPTION 'unobserved food accepted'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS raw food cannot reuse genuine bite without actor observation'; END; END $$;
SELECT game_cake_arm_food_observation('00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),
 '00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',1,1,decode(repeat('aa',16),'hex'),1,1,decode(repeat('33',32),'hex'),'{"schema":1,"conditions":[]}', '{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb);
DO $$ BEGIN BEGIN
INSERT INTO game_seasonal_cake_food_uses(game_session_id,command_id,character_id,world_id,event_start,content_digest,actor_identity,movement_revision,state_revision,character_lease_generation,conditions_before,conditions_after,conditions_sha256)
 SELECT b.game_session_id,b.command_id,b.character_id,b.world_id,b.event_start,decode(repeat('33',32),'hex'),decode(repeat('aa',16),'hex'),1,1,1,
 '{"schema":1,"conditions":[]}',jsonb_set(after,'{conditions,0,tick_debt}','1',TRUE),sha256(convert_to((jsonb_set(after,'{conditions,0,tick_debt}','1',TRUE))::text,'UTF8')) FROM game_seasonal_cake_bite_receipts b CROSS JOIN LATERAL
 (SELECT '{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb after) a;
 RAISE EXCEPTION 'changed kernel successor accepted'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS food successor metadata must equal sealed native kernel result'; END; END $$;
INSERT INTO game_seasonal_cake_food_uses(game_session_id,command_id,character_id,world_id,event_start,content_digest,actor_identity,movement_revision,state_revision,character_lease_generation,conditions_before,conditions_after,conditions_sha256)
 SELECT b.game_session_id,b.command_id,b.character_id,b.world_id,b.event_start,decode(repeat('33',32),'hex'),decode(repeat('aa',16),'hex'),1,1,1,
 '{"schema":1,"conditions":[]}',after,sha256(convert_to(after::text,'UTF8')) FROM game_seasonal_cake_bite_receipts b CROSS JOIN LATERAL
 (SELECT '{"schema":1,"conditions":[{"definition":{"key":"food.regeneration","values":{"FoodRegeneration":{"added_ms":240000,"interval_ms":1000}}},"ends_at":240000000}]}'::jsonb after) a;
DO $$ BEGIN BEGIN
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'food without condition snapshot accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS food without same-transaction condition snapshot rejected'; END; END $$;

INSERT INTO game_character_condition_snapshots(character_id,character_lease_generation,content_digest,snapshot)
 SELECT character_id,character_lease_generation,content_digest,conditions_after FROM game_seasonal_cake_food_uses;
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN BEGIN
 UPDATE game_character_condition_snapshots SET snapshot=jsonb_set(snapshot,'{conditions,0,ends_at}','480000000');
 RAISE EXCEPTION 'snapshot changed after early validation';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS early constraint validation cannot permit later snapshot rewrite'; END; END $$;

COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS native CAS World mark food receipt and snapshot commit atomically'; END $$;

BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+3002);
DO $$ DECLARE e BIGINT; BEGIN SELECT event_start INTO e FROM game_seasonal_cake_worlds;
 BEGIN
  INSERT INTO game_seasonal_cake_stage_polls VALUES('00000000-0000-7000-8000-000000000042',e,5,e+3002,FALSE,pg_current_xact_id());
  RAISE EXCEPTION 'stage2 poll accepted'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS stagepoll cannot reopen harvesting after bridge'; END;
 BEGIN
  INSERT INTO game_seasonal_cake_windows VALUES('00000000-0000-7000-8000-000000000042',e,e+3002,1,e+3002,pg_current_xact_id());
  RAISE EXCEPTION 'offslot window accepted'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS player-time window cannot replace fixed source slot'; END;
 BEGIN
  UPDATE game_seasonal_cake_worlds SET state=state||'{"layer_progress":2,"revision":5}'::jsonb;
  SET CONSTRAINTS ALL IMMEDIATE;
  RAISE EXCEPTION 'raw World progress accepted'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS observed Node alone cannot forge additional bite progress'; END;
 BEGIN
  PERFORM game_cake_apply_terrain_tool_transform('00000000-0000-7000-8000-000000000040',2,'00000000-0000-7000-8000-000000000041',NULL,'\x01',
   '00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',decode(repeat('00',32)||'000082077d410700','hex'),
   'native-test',decode(repeat('33',32),'hex'),0,0,12085,0,'{}','{}');
  RAISE EXCEPTION 'nonedible coordinate accepted'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PASS actual footprint excludes decorative or nonexistent Cake cells'; END;
 BEGIN
  DELETE FROM game_cake_edible_cells;
  RAISE EXCEPTION 'footprint modification accepted'; EXCEPTION WHEN insufficient_privilege THEN RAISE NOTICE 'PASS runtime cannot change source footprint or goal'; END;
END $$;
ROLLBACK;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+4800);
INSERT INTO game_seasonal_cake_windows VALUES('00000000-0000-7000-8000-000000000042',:event,:event+4800,1,:event+4800,pg_current_xact_id());
UPDATE game_seasonal_cake_tiles SET removed=FALSE WHERE removed;
UPDATE game_seasonal_cake_worlds SET state=state||jsonb_build_object('layer_progress',0,'window_start',:event+4800,'window_end',:event+6600,'revision',5);
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS failed window restores current layer and opens only next fixed slot atomically'; END $$;
BEGIN;SET LOCAL session_replication_role=replica;
UPDATE game_seasonal_cake_worlds SET state=state||'{"layer_progress":1066}'::jsonb;
COMMIT;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+5000);
UPDATE game_seasonal_cake_worlds SET state=state||'{"layer":2,"layer_progress":0,"window_start":null,"window_end":null,"revision":6}'::jsonb;
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS actual1066th edible bite completes the round without eating corners'; END $$;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+5001);
DO $$ DECLARE e BIGINT; BEGIN SELECT event_start INTO e FROM game_seasonal_cake_worlds;
 BEGIN
 INSERT INTO game_seasonal_cake_windows VALUES('00000000-0000-7000-8000-000000000042',e,e+4800,2,e+5001,pg_current_xact_id());
 RAISE EXCEPTION 'early completed next layer window accepted'; EXCEPTION WHEN check_violation OR unique_violation THEN RAISE NOTICE 'PASS completed round waits next1800second slot'; END;
END $$;
ROLLBACK;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+6600);
INSERT INTO game_seasonal_cake_windows VALUES('00000000-0000-7000-8000-000000000042',:event,:event+6600,2,:event+6600,pg_current_xact_id());
UPDATE game_seasonal_cake_worlds SET state=state||jsonb_build_object('window_start',:event+6600,'window_end',:event+8400,'revision',7);
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS next layer uses next fixed eating window'; END $$;
BEGIN;SET LOCAL session_replication_role=replica;
UPDATE game_seasonal_cake_worlds SET state=state||'{"layer":4,"layer_progress":1066}'::jsonb;
COMMIT;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+7000);
UPDATE game_seasonal_cake_worlds SET state=state||jsonb_build_object('stage',3,'window_start',NULL,'window_end',NULL,'bonus_from',:event+86400,'bonus_until',:event+86400+604800,'revision',8);
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS fourth actual edible goal starts seven-day bonus at nextBerlin06save'; END $$;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
SELECT game_cake_arm_world_node('00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,
 '00000000-0000-7000-8000-000000000001',1,decode(repeat('11',32),'hex'),:event,:event+518400);
UPDATE game_seasonal_cake_worlds SET state=state||'{"stage":0,"revision":9}'::jsonb;
COMMIT;
DO $$ BEGIN RAISE NOTICE 'PASS annual cleanup preserves earned seven-day bonus'; END $$;

SET enable_seqscan=off;
DO $$ DECLARE plan JSONB; matching BOOLEAN;
BEGIN
 EXECUTE 'EXPLAIN (FORMAT JSON,COSTS OFF) SELECT 1 FROM game_seasonal_cake_food_uses WHERE character_id=''00000000-0000-7000-8000-000000000041''::uuid AND created_xact_id=pg_current_xact_id()' INTO plan;
 RAISE NOTICE 'FOOD_HISTORY_EXPLAIN %',plan;
 IF NOT EXISTS(SELECT 1 FROM pg_index i WHERE i.indrelid='game_seasonal_cake_food_uses'::regclass AND i.indisvalid AND i.indpred IS NULL
 AND ARRAY(SELECT a.attname::text FROM unnest(i.indkey::smallint[]) WITH ORDINALITY k(attnum,n) JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.attnum WHERE n<=2 ORDER BY n)=ARRAY['character_id','created_xact_id']) THEN
 RAISE EXCEPTION 'FOOD_CURRENT_XID_LOOKUP_UNINDEXED' USING ERRCODE='23514'; END IF;
 SELECT coalesce(bool_or(value::text LIKE '%character_id%' AND value::text LIKE '%created_xact_id%'),FALSE) INTO matching
 FROM jsonb_path_query(plan,'$.**."Index Cond"') value;
 IF NOT matching THEN RAISE EXCEPTION 'FOOD_CURRENT_XID_EXPLAIN_MISSING_BOTH_INDEX_CONDITIONS' USING ERRCODE='23514'; END IF;
 RAISE NOTICE 'FOOD_HISTORY_INDEX_PROOF_PASSED leading_character_xact=true exact_index_conditions=true';
END $$;
RESET enable_seqscan;
