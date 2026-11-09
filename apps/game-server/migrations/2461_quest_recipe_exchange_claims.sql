-- Each paid occurrence remains a one-shot claim in the existing MINT owner.
-- Fixed-width UUIDs and a colon-terminated decimal CommandId make the hash
-- unambiguous over the complete unsigned CommandId domain.
CREATE OR REPLACE FUNCTION game_quest_reward_mint_claim_key(v_character UUID,v_cause UUID,v_ordinal NUMERIC,
    v_transition TEXT,v_plan JSONB) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN ((jsonb_array_length(v_plan->'storage')=0
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0)
        OR (jsonb_typeof(v_plan->'recipe_source')='string'
            AND jsonb_typeof(v_plan->'carried_target')='string'
            AND v_plan->>'recipe_source'<>v_plan->>'carried_target'
            AND NOT v_plan ? 'carried_item'
            AND jsonb_array_length(v_plan->'take')>=2))
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND jsonb_array_length(v_plan->'take')>0 AND jsonb_typeof(v_plan->'give')='object'
    THEN 'oteryn:quest-exchange/'||encode(sha256(
        convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
        ||uuid_send(v_character)||uuid_send(v_cause)
        ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
    ELSE v_transition END
$function$;

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;
