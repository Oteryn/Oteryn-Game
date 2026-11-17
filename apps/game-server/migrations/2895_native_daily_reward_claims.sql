CREATE TABLE game_native_daily_reward_claims (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    claim_key TEXT NOT NULL CHECK (octet_length(claim_key) BETWEEN 1 AND 128),
    last_claimed_at BIGINT NOT NULL CHECK (last_claimed_at >= 0),
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
