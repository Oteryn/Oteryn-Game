-- PLAT-WIRE-1: public aggregate samples, never player/account text or identity.
CREATE TABLE game_public_world_deaths (
    world_id UUID PRIMARY KEY,
    player_deaths BIGINT NOT NULL CHECK (player_deaths >= 0)
);
INSERT INTO game_public_world_deaths
SELECT death_world_id, count(*) FROM game_character_death_receipts GROUP BY death_world_id;

CREATE FUNCTION game_public_world_death_touch() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_public_world_deaths VALUES (NEW.death_world_id, 1)
    ON CONFLICT (world_id) DO UPDATE
       SET player_deaths = game_public_world_deaths.player_deaths + 1;
    RETURN NEW;
END $$;
CREATE TRIGGER game_public_world_death_touch AFTER INSERT ON game_character_death_receipts
FOR EACH ROW EXECUTE FUNCTION game_public_world_death_touch();

CREATE TABLE game_public_world_totals_heads (
    world_id UUID PRIMARY KEY,
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    projection_revision BIGINT NOT NULL CHECK (projection_revision >= 0),
    sampled_at BIGINT NOT NULL CHECK (sampled_at >= 0)
);
CREATE TABLE game_public_world_totals_samples (
    world_id UUID NOT NULL REFERENCES game_public_world_totals_heads(world_id),
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    source_observed_at BIGINT NOT NULL CHECK (source_observed_at >= 0),
    online_players BIGINT NOT NULL CHECK (online_players >= 0),
    player_deaths BIGINT NOT NULL CHECK (player_deaths >= 0),
    acked BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (world_id, projection_epoch, projection_revision)
);
CREATE INDEX game_public_world_totals_pending ON game_public_world_totals_samples
    (projection_epoch, source_observed_at, world_id, projection_revision) WHERE NOT acked;
CREATE INDEX game_public_world_totals_worlds ON game_character_roots(world_id);

-- Counter and source query are derived state owned by this function. Runtime has
-- EXECUTE, not write access to the counter, heads or sample contents.
CREATE FUNCTION game_public_world_totals_sample(p_world UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_head game_public_world_totals_heads%ROWTYPE;
    v_epoch BIGINT;
    v_now BIGINT;
BEGIN
    SELECT projection_epoch INTO STRICT v_epoch FROM game_public_projection_epoch
     WHERE epoch_scope = 1 FOR SHARE;
    INSERT INTO game_public_world_totals_heads VALUES (p_world, v_epoch, 0, 0)
    ON CONFLICT DO NOTHING;
    SELECT * INTO STRICT v_head FROM game_public_world_totals_heads
     WHERE world_id = p_world FOR UPDATE;
    v_now := floor(extract(epoch FROM clock_timestamp()))::BIGINT;
    IF v_head.projection_revision > 0 AND v_head.projection_epoch = v_epoch
       AND v_now < v_head.sampled_at + 10 THEN
        RETURN;
    END IF;
    -- A single statement snapshot covers both counts and its observation time,
    -- after waiting for the per-World lock. No sequence/clock-based commit order.
    INSERT INTO game_public_world_totals_samples
        (world_id, projection_epoch, projection_revision, source_observed_at,
         online_players, player_deaths)
    SELECT p_world, v_epoch, v_head.projection_revision + 1,
           floor(extract(epoch FROM clock_timestamp()))::BIGINT,
           (SELECT count(*) FROM game_character_roots r
             WHERE r.world_id = p_world AND r.lifecycle = 1
               AND EXISTS (SELECT 1 FROM game_durability_admission_character_guards a
                 JOIN game_durability_reconnect_sessions s
                   ON s.game_session_id = a.holder_game_session_id
                WHERE a.character_id = r.character_id AND a.eligible
                  AND a.world_id = r.world_id AND s.character_id = r.character_id
                  AND s.world_id = r.world_id AND s.session_state = 2
                  AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL)),
           coalesce((SELECT player_deaths FROM game_public_world_deaths
                      WHERE world_id = p_world), 0)
    RETURNING source_observed_at INTO v_now;
    UPDATE game_public_world_totals_heads SET projection_epoch = v_epoch,
        projection_revision = v_head.projection_revision + 1, sampled_at = v_now
     WHERE world_id = p_world;
END $$;

CREATE TRIGGER game_public_world_deaths_no_truncate BEFORE TRUNCATE
ON game_public_world_deaths EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_public_world_totals_heads_no_truncate BEFORE TRUNCATE
ON game_public_world_totals_heads EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_public_world_totals_samples_no_truncate BEFORE TRUNCATE
ON game_public_world_totals_samples EXECUTE FUNCTION game_character_reject_truncate();
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_public_world_death_touch() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_public_world_totals_sample(uuid) SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON game_public_world_deaths, game_public_world_totals_heads,
    game_public_world_totals_samples FROM PUBLIC;
REVOKE ALL ON FUNCTION game_public_world_death_touch(), game_public_world_totals_sample(uuid) FROM PUBLIC;
GRANT SELECT ON game_public_world_deaths, game_public_world_totals_heads,
    game_public_world_totals_samples TO oteryn_game_runtime, oteryn_game_control;
GRANT UPDATE (acked) ON game_public_world_totals_samples TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_public_world_totals_sample(uuid) TO oteryn_game_runtime;
-- Rollback is a forward migration dropping these derived objects and the trigger;
-- death receipts remain authoritative. No history pruning is enabled by V1.

-- Extend the existing owner-only resync, preserving its epoch/guard chain.
ALTER FUNCTION game_public_projection_resync(text, boolean)
    RENAME TO game_public_projection_resync_before_world_totals;
CREATE FUNCTION game_public_projection_resync(p_family TEXT, p_raise_epoch BOOLEAN)
RETURNS BIGINT LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE v_epoch BIGINT;
BEGIN
    IF p_family = 'world_totals' AND p_raise_epoch IS FALSE THEN
        SELECT projection_epoch INTO STRICT v_epoch FROM game_public_projection_epoch
         WHERE epoch_scope=1 FOR UPDATE;
    ELSE
        v_epoch := game_public_projection_resync_before_world_totals(p_family, p_raise_epoch);
    END IF;
    IF p_family IN ('all', 'world_totals') THEN
        UPDATE game_public_world_totals_samples SET acked=false WHERE projection_epoch=v_epoch;
    END IF;
    RETURN v_epoch;
END $$;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_public_projection_resync(text, boolean) SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_public_projection_resync(text, boolean) FROM PUBLIC;
