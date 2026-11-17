CREATE TABLE game_native_daily_reward_claims (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    claim_key TEXT NOT NULL CHECK (octet_length(claim_key) BETWEEN 1 AND 128),
    last_claimed_at BIGINT NOT NULL CHECK (last_claimed_at >= 0),
    mint_key TEXT NOT NULL CHECK (octet_length(mint_key) BETWEEN 1 AND 128),
    claimed_xact_id XID8 NOT NULL,
    PRIMARY KEY (character_id, claim_key)
);

-- The database supplies the clock for the claim transaction.
CREATE FUNCTION game_native_daily_reward_clock() RETURNS timestamptz
LANGUAGE sql STABLE SET search_path = pg_catalog AS $$
    SELECT statement_timestamp()
$$;

REVOKE ALL ON game_native_daily_reward_claims FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_native_daily_reward_claims TO oteryn_game_runtime;
GRANT SELECT ON game_native_daily_reward_claims TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_native_daily_reward_clock() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_native_daily_reward_clock() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_native_daily_reward_clock() TO oteryn_game_control;

-- Keep the existing receipt proof and resolve daily mint identities from the
-- admission row written by the same fenced transaction.
DO $migration$
DECLARE body TEXT; anchor TEXT;
BEGIN
    SELECT pg_get_functiondef('game_quest_native_claim_plan(JSONB)'::regprocedure) INTO STRICT body;
    anchor := $anchor$AND (p->'world_claim')-ARRAY['key','revision','authority_sha256']::TEXT[]='{}'::jsonb$anchor$;
    IF strpos(body, anchor) = 0 THEN
        RAISE EXCEPTION 'native daily claim requires the existing world claim shape proof';
    END IF;
    EXECUTE replace(body, anchor, $proof$AND (p->'world_claim')-ARRAY['key','revision','authority_sha256','daily']::TEXT[]='{}'::jsonb
        AND (NOT (p->'world_claim') ? 'daily' OR p #> '{world_claim,daily}' = 'true'::jsonb)$proof$);

    SELECT pg_get_functiondef('game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB)'::regprocedure)
        INTO STRICT body;
    anchor := $anchor$CASE WHEN game_quest_native_claim_plan(v_plan)$anchor$;
    IF strpos(body, anchor) = 0 THEN
        RAISE EXCEPTION 'native daily claim requires the existing quest MINT identity proof';
    END IF;
    body := replace(body, 'IMMUTABLE', 'STABLE');
    EXECUTE replace(body, anchor, $proof$CASE WHEN game_quest_native_claim_plan(v_plan)
        AND v_plan #> '{world_claim,daily}' = 'true'::jsonb THEN (
            SELECT mint_key FROM game_native_daily_reward_claims
            WHERE character_id=v_character AND claim_key=v_plan #>> '{world_claim,key}'
              AND claimed_xact_id=pg_current_xact_id()
        ) WHEN game_quest_native_claim_plan(v_plan)$proof$);
END $migration$;
