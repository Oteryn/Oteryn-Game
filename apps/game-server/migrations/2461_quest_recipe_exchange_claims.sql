-- Compose the native authored-claim owner and the carried-recipe extension
-- with main's counter, fee-repeatable, timed and per-occurrence identities.
-- The nested counter CASE is the exact anchor used by main's later 2701;
-- native claims retain their original higher-priority identity.
CREATE OR REPLACE FUNCTION game_quest_reward_mint_claim_key(v_character UUID,v_cause UUID,v_ordinal NUMERIC,
    v_transition TEXT,v_plan JSONB) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN game_quest_native_claim_plan(v_plan)
    THEN v_plan #>> '{world_claim,key}'
    ELSE (
    SELECT CASE WHEN v_plan ? 'counter' THEN
        v_transition || '/occurrence/' || replace(v_cause::text,'-','') || '/' || v_ordinal::text
    WHEN (
        v_plan->'repeatable'='true'::jsonb
        AND jsonb_typeof(v_plan->'give')='object'
        AND (v_plan #>> '{fee,gold}')::numeric BETWEEN 1 AND 1000000000000
        AND NOT coalesce((v_plan->>'completes')::boolean,false)
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(v_plan->'storage','[]'::jsonb)) s
            WHERE s->>'preserve' IS DISTINCT FROM 'true')
    ) OR v_plan->'per_occurrence_mint'='true'::jsonb OR (
        v_plan #>> '{world_callback,trigger}'='use'
        AND v_plan #>> '{world_callback,key}'=v_transition
        AND v_plan #> '{world_callback,state_operations}'=v_plan->'state_ops'
        AND EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb)) operation
            WHERE operation #>> '{from,kind}'='elapsed_at_least'
                AND (operation #>> '{from,seconds}')::numeric BETWEEN 1 AND 31622400
                AND operation->'effect'='{"kind":"set_now"}'::jsonb)
    ) OR (
        ((jsonb_array_length(v_plan->'storage')=0
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0)
        OR (jsonb_typeof(v_plan->'recipe_source')='string'
            AND jsonb_typeof(v_plan->'carried_target')='string'
            AND v_plan->>'recipe_source'<>v_plan->>'carried_target'
            AND NOT v_plan ? 'carried_item'
            AND jsonb_array_length(v_plan->'take')>=2))
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND jsonb_array_length(v_plan->'take')>0 AND jsonb_typeof(v_plan->'give')='object')
    THEN 'oteryn:quest-exchange/'||encode(sha256(
        convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
        ||uuid_send(v_character)||uuid_send(v_cause)
        ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
    ELSE v_transition END
    ) END
$function$;

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;
