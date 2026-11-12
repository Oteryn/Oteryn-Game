\set ON_ERROR_STOP on
\timing on
BEGIN;
CREATE SCHEMA qn4_m20_recipe_claim_case;
SET LOCAL search_path=qn4_m20_recipe_claim_case,pg_catalog;
SELECT 'QN4_M20_CLAIM_POSTGRES_READY',current_setting('server_version');
\i :claim_migration
DO $$
DECLARE actor UUID:='29020304-0506-7008-800a-0b0c0d0e0f29';
 session UUID:='32020304-0506-7008-800a-0b0c0d0e0f32';
 plan JSONB; k TEXT; expected TEXT:='oteryn:quest-exchange/60c01506ec37ab38d432b8fb9e77eb91954b5978623492d61bc265a5c091a789';
BEGIN
 plan:='{"storage":[{"track":"questline","minimum":-1,"maximum":4,"set":2,"at_least":true}],"state_ops":[],"participants":[],"recipe_source":"oil","carried_target":"dust","take":[{"item":"oil","count":1},{"item":"dust","count":1}],"give":{"item":"ointment","count":1}}'::jsonb;
 k:=game_quest_reward_mint_claim_key(actor,session,1,'source',plan);
 IF k IS DISTINCT FROM expected THEN RAISE EXCEPTION 'stateful recipe must use the existing per-command claim: %',k; END IF;
 IF k=game_quest_reward_mint_claim_key(actor,session,2,'source',plan) THEN RAISE EXCEPTION 'new commands need distinct claims'; END IF;
 IF k IS DISTINCT FROM game_quest_reward_mint_claim_key(actor,session,1,'source',plan) THEN RAISE EXCEPTION 'replay must keep its claim'; END IF;
 IF game_quest_reward_mint_claim_key(actor,session,1,'source',plan-'recipe_source'-'carried_target')<>'source' THEN RAISE EXCEPTION 'legacy stateful grants remain once-only'; END IF;
 IF game_quest_reward_mint_claim_key(actor,session,1,'source',jsonb_set(plan-'recipe_source'-'carried_target','{storage}','[]'::jsonb)) IS DISTINCT FROM expected THEN RAISE EXCEPTION 'legacy paid exchanges changed'; END IF;
 IF game_quest_reward_mint_claim_key(actor,session,1,'source',plan-'carried_target')<>'source' THEN RAISE EXCEPTION 'missing target is not a recipe'; END IF;
 IF game_quest_reward_mint_claim_key(actor,session,1,'source',jsonb_set(plan,'{carried_target}','"oil"'))<>'source' THEN RAISE EXCEPTION 'aliased operands are not a recipe'; END IF;
 IF game_quest_reward_mint_claim_key(actor,session,1,'source',jsonb_set(plan,'{participants}','[{}]'::jsonb))<>'source' THEN RAISE EXCEPTION 'participant rewards remain once-only'; END IF;
 RAISE NOTICE 'M20_RECIPE_CLAIM_CASES 8 PASS';
END $$;
ROLLBACK;
