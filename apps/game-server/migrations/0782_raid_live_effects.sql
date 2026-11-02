-- RAIDS-GLOBAL: a firing freezes its channel targets. A committed step remains
-- pending until the current channel owner acknowledges its idempotent live effect.
CREATE TABLE game_world_raid_scheduler_clocks (
    world_id UUID PRIMARY KEY,
    reset_epoch NUMERIC(20,0) NOT NULL
        CHECK (reset_epoch BETWEEN 1 AND 18446744073709551615),
    last_end_at_ms BIGINT NOT NULL CHECK (last_end_at_ms >= 0)
);

-- Current Channel/Activity publishers project source GameplayIdleState into portable
-- expiries. Missing current-window reports are unknown, never a fabricated zero.
CREATE TABLE game_world_raid_activity (
    world_id UUID NOT NULL,
    scope_key BYTEA PRIMARY KEY CHECK (octet_length(scope_key) = 33 AND get_byte(scope_key,0) IN (1,3)),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 1 AND 18446744073709551615),
    owner_generation NUMERIC(20,0) NOT NULL CHECK (owner_generation BETWEEN 1 AND 18446744073709551615),
    observed_at_ms BIGINT NOT NULL CHECK (observed_at_ms >= 0),
    players JSONB CHECK (players IS NULL OR (jsonb_typeof(players) = 'array' AND jsonb_array_length(players) <= 262144))
);

-- Legacy XML, modern Lua and calendar callbacks own separate running gates.
CREATE TABLE game_world_raid_schedule_sources (
    world_id UUID NOT NULL,
    raid_key TEXT NOT NULL,
    source_kind SMALLINT NOT NULL CHECK (source_kind IN (1, 2, 3)),
    PRIMARY KEY (world_id, raid_key),
    FOREIGN KEY (world_id, raid_key) REFERENCES game_world_raid_schedules(world_id, raid_key)
        ON DELETE RESTRICT
);

-- The Lua donor retains adaptive counters in KV across process restarts. They
-- belong to the existing World/raid schedule and commit with its firing.
CREATE TABLE game_world_raid_lua_states (
    world_id UUID NOT NULL,
    raid_key TEXT NOT NULL,
    state JSONB NOT NULL CHECK (jsonb_typeof(state) = 'object'),
    PRIMARY KEY (world_id, raid_key),
    FOREIGN KEY (world_id, raid_key) REFERENCES game_world_raid_schedules(world_id, raid_key)
        ON DELETE RESTRICT
);

CREATE TABLE game_world_raid_targets (
    firing_id UUID NOT NULL REFERENCES game_world_raid_firings(firing_id) ON DELETE RESTRICT,
    channel_id UUID NOT NULL,
    owner_generation NUMERIC(20,0) NOT NULL
        CHECK (owner_generation BETWEEN 1 AND 18446744073709551615),
    PRIMARY KEY (firing_id, channel_id)
);
CREATE INDEX game_world_raid_targets_channel ON game_world_raid_targets(channel_id, firing_id);

CREATE TABLE game_world_raid_effects (
    firing_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    step_ordinal INTEGER NOT NULL CHECK (step_ordinal >= 0),
    owner_generation NUMERIC(20,0) NOT NULL
        CHECK (owner_generation BETWEEN 1 AND 18446744073709551615),
    action_kind SMALLINT NOT NULL CHECK (action_kind IN (1, 2, 3, 4)),
    -- announcement, wave, complete, LOST cleanup
    announcement_text TEXT,
    announcement_advance BOOLEAN,
    wave SMALLINT CHECK (wave >= 0),
    boss_step BOOLEAN,
    cleanup BOOLEAN,
    state SMALLINT NOT NULL DEFAULT 1 CHECK (state IN (1, 2, 3)),
    -- pending, acknowledged, discarded by LOST
    settled_at_ms BIGINT CHECK (settled_at_ms >= 0),
    PRIMARY KEY (firing_id, channel_id, step_ordinal),
    FOREIGN KEY (firing_id, channel_id, step_ordinal)
        REFERENCES game_world_raid_step_receipts(firing_id, channel_id, step_ordinal) ON DELETE RESTRICT,
    CHECK ((state = 1) = (settled_at_ms IS NULL)),
    CHECK ((action_kind = 1) = (announcement_text IS NOT NULL)),
    CHECK ((action_kind = 1) = (announcement_advance IS NOT NULL)),
    CHECK (announcement_text IS NULL OR octet_length(announcement_text) BETWEEN 1 AND 1024),
    CHECK ((action_kind = 2) = (wave IS NOT NULL)),
    CHECK ((action_kind = 2) = (boss_step IS NOT NULL)),
    CHECK ((action_kind IN (3, 4)) = (cleanup IS NOT NULL)),
    CHECK (action_kind <> 4 OR cleanup)
);
CREATE UNIQUE INDEX game_world_raid_one_pending_effect
    ON game_world_raid_effects(firing_id, channel_id) WHERE state = 1;
CREATE INDEX game_world_raid_effects_channel_pending
    ON game_world_raid_effects(channel_id, firing_id) WHERE state = 1;

-- Every attempted 0329 spawn identity has exactly one retained placement result.
-- Failures are terminal attempts, rather than work that is retried indefinitely.
CREATE TABLE game_world_raid_creature_placements (
    firing_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    wave SMALLINT NOT NULL,
    entry SMALLINT NOT NULL,
    creature_index SMALLINT NOT NULL,
    placed BOOLEAN NOT NULL,
    recorded_at_ms BIGINT NOT NULL CHECK (recorded_at_ms >= 0),
    PRIMARY KEY (firing_id, channel_id, wave, entry, creature_index),
    FOREIGN KEY (firing_id, channel_id, wave, entry, creature_index)
        REFERENCES game_world_raid_creatures(firing_id, channel_id, wave, entry, creature_index)
        ON DELETE RESTRICT
);

CREATE FUNCTION game_raid_live_effect_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' OR
       (NEW.firing_id, NEW.channel_id, NEW.step_ordinal, NEW.owner_generation,
        NEW.action_kind, NEW.announcement_text, NEW.announcement_advance,
        NEW.wave, NEW.boss_step, NEW.cleanup) IS DISTINCT FROM
       (OLD.firing_id, OLD.channel_id, OLD.step_ordinal, OLD.owner_generation,
        OLD.action_kind, OLD.announcement_text, OLD.announcement_advance,
        OLD.wave, OLD.boss_step, OLD.cleanup) OR
       (OLD.state <> 1 AND (NEW.state, NEW.settled_at_ms) IS DISTINCT FROM
                           (OLD.state, OLD.settled_at_ms)) THEN
        RAISE EXCEPTION 'raid effect identity and settled result are immutable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER raid_live_effect_guard BEFORE UPDATE OR DELETE ON game_world_raid_effects
    FOR EACH ROW EXECUTE FUNCTION game_raid_live_effect_guard();

-- Legacy firings did not freeze a target set and never created live actors.
-- They cannot acquire the new live authority by guessing targets during upgrade.
UPDATE game_world_raid_firings SET state = 3 WHERE state = 1;
UPDATE game_world_raid_channel_runs SET state = 3 WHERE state = 1;
UPDATE game_world_raid_creatures SET state = 2 WHERE state = 1;

REVOKE ALL ON game_world_raid_scheduler_clocks, game_world_raid_schedule_sources, game_world_raid_activity, game_world_raid_lua_states, game_world_raid_targets, game_world_raid_effects,
    game_world_raid_creature_placements FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_raid_targets, game_world_raid_effects,
    game_world_raid_creature_placements TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_world_raid_scheduler_clocks TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_world_raid_schedule_sources TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_world_raid_activity TO oteryn_game_runtime;
GRANT UPDATE (reset_epoch, owner_generation, observed_at_ms, players) ON game_world_raid_activity TO oteryn_game_runtime;
GRANT UPDATE (reset_epoch, last_end_at_ms) ON game_world_raid_scheduler_clocks TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_world_raid_lua_states TO oteryn_game_runtime;
GRANT UPDATE (state) ON game_world_raid_lua_states TO oteryn_game_runtime;
GRANT UPDATE (state, settled_at_ms) ON game_world_raid_effects TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_raid_live_effect_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_raid_live_effect_guard() TO oteryn_game_runtime;

GRANT SELECT ON game_world_raid_scheduler_clocks, game_world_raid_schedule_sources, game_world_raid_activity, game_world_raid_lua_states, game_world_raid_targets, game_world_raid_effects,
    game_world_raid_creature_placements TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_raid_live_effect_guard() TO oteryn_game_control;
