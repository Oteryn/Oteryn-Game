\set ON_ERROR_STOP on
\timing on
BEGIN;
CREATE SCHEMA qn6_tool_guard;
SET LOCAL search_path=qn6_tool_guard,public;
SELECT 'QN6_POSTGRES_READY creature_tool', current_setting('server_version');
CREATE TABLE game_quest_reward_sources (
 character_id uuid, cause_id uuid, cause_ordinal numeric, transition_key text,
 plan jsonb, created_xact_id xid8 DEFAULT pg_current_xact_id()
);
CREATE TABLE game_quest_callback_consumptions (
 character_id uuid, cause_id uuid, cause_ordinal numeric, transition_key text,
 quantity integer, replacement jsonb, before_item jsonb, item_instance_id uuid,
 proof bytea, created_xact_id xid8 DEFAULT pg_current_xact_id()
);
\ir ../../migrations/1005_quest_carried_use_proofs.sql
INSERT INTO game_quest_callback_consumptions
 SELECT '00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',1,'tool',1,NULL,
 '{"definition_production_key":"oteryn:item.tibia.i17673"}', '00000000-0000-0000-0000-000000000003',
 convert_to(jsonb_build_object('reward_callback',p,'carried_use',jsonb_build_object('source',
 '[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3]'::jsonb,'intent','[8,1]'::jsonb))::text,'UTF8'),pg_current_xact_id()
 FROM (SELECT '{"carried_item":"oteryn:item.tibia.i17673","creature_target":"oteryn:creature.rotworm","take":[{"item":"oteryn:item.tibia.i17673","count":1}],"give":{"item":"oteryn:item.tibia.i17674","count":1,"contents":[]}}'::jsonb p) s;
-- RED: the old production trigger rejects the otherwise complete transformation.
DO $$ BEGIN
 BEGIN
 INSERT INTO game_quest_reward_sources SELECT character_id,cause_id,cause_ordinal,transition_key,
 convert_from(proof,'UTF8')::jsonb->'reward_callback',pg_current_xact_id() FROM game_quest_callback_consumptions;
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'old guard unexpectedly accepted output';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'QN6_RED old carried output rejected';
 END;
END $$;
\ir ../../migrations/2500_quest_creature_tool_transform.sql
INSERT INTO game_quest_reward_sources SELECT character_id,cause_id,cause_ordinal,transition_key,
 convert_from(proof,'UTF8')::jsonb->'reward_callback',pg_current_xact_id() FROM game_quest_callback_consumptions;
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 IF (SELECT count(*) FROM game_quest_reward_sources) <> 1 THEN RAISE EXCEPTION 'output not accepted'; END IF;
 IF game_quest_carried_output_valid('{}') THEN RAISE EXCEPTION 'missing output accepted'; END IF;
 IF game_quest_carried_output_valid('{"give":{"item":"oteryn:item.tibia.i17674","count":1}}') THEN RAISE EXCEPTION 'unbound output accepted'; END IF;
 IF game_quest_carried_output_valid('{"creature_target":"oteryn:creature.rotworm","give":{"item":"oteryn:item.tibia.i17674","count":2}}') THEN RAISE EXCEPTION 'multiple outputs accepted'; END IF;
 RAISE NOTICE 'QN6_GREEN transformed tool retains exact input and ingress';
END $$;
ROLLBACK;
