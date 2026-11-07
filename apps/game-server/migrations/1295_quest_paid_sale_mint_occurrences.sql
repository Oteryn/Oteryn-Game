-- Extend the shared occurrence helper only for repeatable monetary sales.
-- The original shared barter/one-shot helper and all its proofs stay unchanged.
ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB)
    RENAME TO game_quest_reward_mint_claim_key_qengine_base;
CREATE FUNCTION game_quest_reward_mint_claim_key(v_character UUID,v_cause UUID,v_ordinal NUMERIC,
    v_transition TEXT,v_plan JSONB) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN v_plan->'repeatable'='true'::jsonb
        AND jsonb_typeof(v_plan->'fee')='object'
        AND (v_plan #>> '{fee,gold}')::bigint BETWEEN 1 AND 1000000000000
        AND jsonb_typeof(v_plan->'give')='object'
    THEN 'oteryn:quest-exchange/'||encode(sha256(
        convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
        ||uuid_send(v_character)||uuid_send(v_cause)
        ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
    ELSE game_quest_reward_mint_claim_key_qengine_base(v_character,v_cause,v_ordinal,v_transition,v_plan) END
$function$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;
