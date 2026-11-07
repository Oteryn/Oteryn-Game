-- Reuse QENGINE 1004's claim identity without bypassing once-only MINT claims.
-- The exact paid CommandRef becomes the opportunity; replay retains the same ID.
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
    ELSE v_transition END
$function$;
DO $migration$
DECLARE body TEXT; old TEXT:='m.claim_production_key=NEW.transition_key';
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
    IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
        RAISE EXCEPTION 'quest MINT completion proof shape changed'; END IF;
    EXECUTE replace(body,old,
        'm.claim_production_key=game_quest_reward_mint_claim_key(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key,NEW.plan)');
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;
