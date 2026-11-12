\set ON_ERROR_STOP on
\timing on
BEGIN;
CREATE SCHEMA qn4_m20_recipe_case;
SET LOCAL search_path=qn4_m20_recipe_case,pg_catalog;
CREATE TABLE game_quest_reward_sources (
 character_id UUID,cause_id UUID,cause_ordinal NUMERIC,transition_key TEXT,plan JSONB,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
CREATE TABLE game_quest_callback_consumptions (
 character_id UUID,cause_id UUID,cause_ordinal NUMERIC,transition_key TEXT,
 item_instance_id UUID,before_item JSONB,quantity INTEGER,replacement TEXT,location_kind TEXT,
 proof BYTEA,created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
SELECT 'QN4_M20_POSTGRES_READY',current_setting('server_version');
\if :{?m20_migration}
\i :m20_migration
\else
\ir ../../../migrations/2460_quest_carried_recipes.sql
\endif
DO $$ BEGIN
 IF to_regprocedure('game_quest_recipe_use_complete()') IS NULL
   OR NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_quest_reward_sources'::regclass AND tgname='game_quest_recipe_use_consistent') THEN
  RAISE EXCEPTION 'M20 selected-input guard is unavailable';
 END IF;
END $$;
CREATE FUNCTION exercise(mode TEXT) RETURNS void LANGUAGE plpgsql AS $$
DECLARE actor UUID:='00000000-0000-7000-8000-000000000003';
 session UUID:='00000000-0000-7000-8000-000000000004';
 source UUID:='00000000-0000-7000-8000-000000000001';
 target UUID:='00000000-0000-7000-8000-000000000002';
 plan JSONB; ingress JSONB; source_proof JSONB; target_proof JSONB; rejected BOOLEAN:=false;
BEGIN
 DELETE FROM game_quest_reward_sources;
 DELETE FROM game_quest_callback_consumptions;
 plan:=jsonb_build_object('recipe_source','oteryn:item.tibia.i21505','carried_target','oteryn:item.tibia.i21507',
   'take',jsonb_build_array(jsonb_build_object('item','oteryn:item.tibia.i21505','count',1),jsonb_build_object('item','oteryn:item.tibia.i21507','count',1)),
   'give',jsonb_build_object('item','oteryn:item.tibia.i21506','count',1,'contents','[]'::jsonb));
 ingress:=jsonb_build_object('source',(SELECT jsonb_agg(get_byte(uuid_send(source),i) ORDER BY i) FROM generate_series(0,15)i),
   'recipe_target',(SELECT jsonb_agg(get_byte(uuid_send(target),i) ORDER BY i) FROM generate_series(0,15)i),'intent','[1,2,3]'::jsonb);
 IF mode='wrong_target' THEN ingress:=jsonb_set(ingress,'{recipe_target}','[0]'::jsonb); END IF;
 IF mode='wrong_source' THEN ingress:=jsonb_set(ingress,'{source}','[0]'::jsonb); END IF;
 IF mode='invalid_bytes' THEN ingress:=jsonb_set(ingress,'{intent}','[300]'::jsonb); END IF;
 source_proof:=jsonb_build_object('reward_callback',plan,'carried_use',ingress);
 target_proof:=source_proof;
 IF mode='different_intent' THEN target_proof:=jsonb_set(target_proof,'{carried_use,intent}','[9]'::jsonb); END IF;
 INSERT INTO game_quest_callback_consumptions(character_id,cause_id,cause_ordinal,transition_key,item_instance_id,before_item,quantity,location_kind,proof)
 VALUES(actor,session,1,'m20',source,jsonb_build_object('definition_production_key','oteryn:item.tibia.i21505'),1,'container',convert_to(source_proof::text,'UTF8'));
 IF mode<>'missing_target' THEN
  INSERT INTO game_quest_callback_consumptions(character_id,cause_id,cause_ordinal,transition_key,item_instance_id,before_item,quantity,location_kind,proof)
  VALUES(actor,session,1,'m20',target,jsonb_build_object('definition_production_key','oteryn:item.tibia.i21507'),CASE WHEN mode='wrong_count' THEN 2 ELSE 1 END,'container',convert_to(target_proof::text,'UTF8'));
 END IF;
 IF mode='legacy_marker' THEN plan:=plan||jsonb_build_object('carried_item','oteryn:item.tibia.i21505'); END IF;
 BEGIN
  INSERT INTO game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key,plan) VALUES(actor,session,1,'m20',plan);
  SET CONSTRAINTS game_quest_recipe_use_consistent IMMEDIATE;
 EXCEPTION WHEN check_violation THEN rejected:=true;
 END;
 IF (mode='ok' AND rejected) OR (mode<>'ok' AND NOT rejected) THEN
  RAISE EXCEPTION 'M20 case % rejected=%',mode,rejected;
 END IF;
 RAISE NOTICE 'M20_GUARD_CASE % PASS',mode;
END $$;
SELECT exercise(mode) FROM unnest(ARRAY['ok','wrong_target','wrong_source','invalid_bytes','different_intent','missing_target','wrong_count','legacy_marker']) mode;
ROLLBACK;
