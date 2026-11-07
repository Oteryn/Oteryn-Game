-- DAILY-1 run 1c: committed World-reset history and the claim/state core.
-- History belongs to the World-reset owner. Runtime may read it, never create
-- epochs from wall time, restart, channel or session-generation changes.
-- Joker ledger, complete reward lines and item bindings are subsequent slices;
-- capability 32 remains unoffered until the complete transaction is composed.
CREATE TABLE game_world_reset_history (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 1 AND 18446744073709551615),
    reset_day BIGINT NOT NULL,
    reset_at_unix_ms BIGINT NOT NULL CHECK (reset_at_unix_ms >= 0),
    committed_at_unix_ms BIGINT NOT NULL CHECK (committed_at_unix_ms >= reset_at_unix_ms),
    PRIMARY KEY (world_id, reset_epoch),
    UNIQUE (world_id, reset_day),
    UNIQUE (world_id, reset_epoch, reset_day),
    CHECK (reset_day = ((to_timestamp(reset_at_unix_ms / 1000.0)
        AT TIME ZONE 'Europe/Berlin') - INTERVAL '10 hours')::date - DATE '1970-01-01')
);

CREATE TABLE game_character_daily_rewards (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
    world_id UUID NOT NULL,
    lane_day SMALLINT NOT NULL CHECK (lane_day BETWEEN 1 AND 7),
    streak INTEGER NOT NULL CHECK (streak BETWEEN 0 AND 100000),
    last_claim_epoch NUMERIC(20,0),
    last_claim_day BIGINT,
    PRIMARY KEY (character_id, world_id),
    CHECK ((last_claim_epoch IS NULL AND last_claim_day IS NULL AND streak = 0 AND lane_day = 1)
        OR (last_claim_epoch IS NOT NULL AND last_claim_epoch BETWEEN 1 AND 18446744073709551615
            AND last_claim_day IS NOT NULL AND streak > 0)),
    CHECK ((last_claim_epoch IS NULL) = (last_claim_day IS NULL))
);

CREATE TABLE game_character_daily_reward_claims (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    reset_day BIGINT NOT NULL,
    transaction_id UUID NOT NULL UNIQUE,
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    lane_day SMALLINT NOT NULL CHECK (lane_day BETWEEN 1 AND 7),
    streak_before INTEGER NOT NULL CHECK (streak_before BETWEEN 0 AND 100000),
    streak_after INTEGER NOT NULL CHECK (streak_after BETWEEN 1 AND 100000),
    jokers_spent SMALLINT NOT NULL CHECK (jokers_spent BETWEEN 0 AND 3),
    premium BOOLEAN NOT NULL CHECK (NOT premium), -- Free until PREM-3
    PRIMARY KEY (character_id, world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch, reset_day)
        REFERENCES game_world_reset_history(world_id, reset_epoch, reset_day) ON DELETE RESTRICT,
    FOREIGN KEY (character_id, world_id)
        REFERENCES game_character_daily_rewards(character_id, world_id)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE game_character_daily_rewards ADD CONSTRAINT daily_reward_last_claim_fk
    FOREIGN KEY (character_id, world_id, last_claim_epoch)
    REFERENCES game_character_daily_reward_claims(character_id, world_id, reset_epoch)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

-- Primary keys cover exact claim replay and latest World/Character epoch lookup.
-- The day key also supports bounded committed-month history lookups by its owner.
CREATE INDEX game_daily_reward_claims_world_epoch_idx
    ON game_character_daily_reward_claims(world_id, reset_epoch, character_id);

CREATE FUNCTION game_daily_reward_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    RAISE EXCEPTION 'daily reward history is immutable' USING ERRCODE = '23514';
END $$;

CREATE TRIGGER daily_reward_claims_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON game_character_daily_reward_claims
    FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER world_reset_history_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON game_world_reset_history
    FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER daily_reward_state_no_delete
    BEFORE DELETE OR TRUNCATE ON game_character_daily_rewards
    FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();

CREATE FUNCTION game_world_reset_history_ordered() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE previous public.game_world_reset_history%ROWTYPE;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:world-reset:' || NEW.world_id::text, 0));
    SELECT * INTO previous FROM public.game_world_reset_history
        WHERE world_id = NEW.world_id ORDER BY reset_epoch DESC LIMIT 1;
    IF FOUND AND (NEW.reset_epoch <= previous.reset_epoch
        OR NEW.reset_day <= previous.reset_day
        OR NEW.reset_at_unix_ms <= previous.reset_at_unix_ms
        OR NEW.committed_at_unix_ms < previous.committed_at_unix_ms) THEN
        RAISE EXCEPTION 'World reset history must advance' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER world_reset_history_ordered BEFORE INSERT ON game_world_reset_history
    FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_ordered();

-- O(1) final-state proof: a claim cannot commit without its matching projection,
-- nor can a projection move backwards or retain a different day/lane/streak.
CREATE FUNCTION game_daily_reward_projection_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    state public.game_character_daily_rewards%ROWTYPE;
    claim public.game_character_daily_reward_claims%ROWTYPE;
BEGIN
    SELECT * INTO state FROM public.game_character_daily_rewards
        WHERE character_id = NEW.character_id AND world_id = NEW.world_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'daily reward state missing' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO claim FROM public.game_character_daily_reward_claims
        WHERE character_id = NEW.character_id AND world_id = NEW.world_id
        ORDER BY reset_epoch DESC LIMIT 1;
    IF FOUND THEN
        IF state.last_claim_epoch IS DISTINCT FROM claim.reset_epoch
            OR state.last_claim_day IS DISTINCT FROM claim.reset_day
            OR state.streak <> claim.streak_after
            OR state.lane_day <> (CASE WHEN claim.lane_day = 7 THEN 1 ELSE claim.lane_day + 1 END) THEN
            RAISE EXCEPTION 'daily reward projection mismatch' USING ERRCODE = '23514';
        END IF;
    ELSIF state.last_claim_epoch IS NOT NULL THEN
        RAISE EXCEPTION 'daily reward claim missing' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_reward_state_final
    AFTER INSERT OR UPDATE ON game_character_daily_rewards DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_daily_reward_projection_guard();
CREATE CONSTRAINT TRIGGER daily_reward_claim_final
    AFTER INSERT ON game_character_daily_reward_claims DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_daily_reward_projection_guard();

REVOKE ALL ON game_world_reset_history, game_character_daily_rewards,
    game_character_daily_reward_claims FROM PUBLIC;
GRANT SELECT ON game_world_reset_history TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_daily_rewards, game_character_daily_reward_claims
    TO oteryn_game_runtime;
GRANT UPDATE (lane_day, streak, last_claim_epoch, last_claim_day)
    ON game_character_daily_rewards TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_daily_reward_immutable(), game_world_reset_history_ordered(),
    game_daily_reward_projection_guard() FROM PUBLIC;
