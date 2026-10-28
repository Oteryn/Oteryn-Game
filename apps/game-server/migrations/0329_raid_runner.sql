-- RAID-RUN-1: durable per-channel action cursor and idempotency receipts.
CREATE TABLE game_world_raid_channel_runs (
    firing_id UUID NOT NULL REFERENCES game_world_raid_firings(firing_id) ON DELETE RESTRICT,
    channel_id UUID NOT NULL,
    owner_generation NUMERIC(20,0) NOT NULL CHECK (owner_generation BETWEEN 1 AND 18446744073709551615),
    committed_steps INTEGER NOT NULL DEFAULT 0 CHECK (committed_steps >= 0),
    state SMALLINT NOT NULL DEFAULT 1 CHECK (state IN (1, 2, 3)), -- RUNNING, DONE, LOST
    updated_at_ms BIGINT NOT NULL CHECK (updated_at_ms >= 0),
    PRIMARY KEY (firing_id, channel_id)
);

CREATE TABLE game_world_raid_step_receipts (
    firing_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    step_ordinal INTEGER NOT NULL CHECK (step_ordinal >= 0),
    action_kind SMALLINT NOT NULL CHECK (action_kind IN (1, 2, 3)), -- announcement, wave, complete
    committed_at_ms BIGINT NOT NULL CHECK (committed_at_ms >= 0),
    PRIMARY KEY (firing_id, channel_id, step_ordinal),
    FOREIGN KEY (firing_id, channel_id) REFERENCES game_world_raid_channel_runs(firing_id, channel_id)
        ON DELETE RESTRICT
);

-- A realization is the durable, replay-safe identity of one raid creature.  The
-- runtime actor is ephemeral; recovery never inserts this source twice.
CREATE TABLE game_world_raid_creatures (
    firing_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    wave SMALLINT NOT NULL CHECK (wave >= 0),
    entry SMALLINT NOT NULL CHECK (entry >= 0),
    creature_index SMALLINT NOT NULL CHECK (creature_index >= 0),
    creature_key TEXT NOT NULL CHECK (creature_key <> ''),
    anchor_key TEXT NOT NULL CHECK (anchor_key <> ''),
    state SMALLINT NOT NULL DEFAULT 1 CHECK (state IN (1, 2)), -- ALIVE, CLEANED
    PRIMARY KEY (firing_id, channel_id, wave, entry, creature_index),
    FOREIGN KEY (firing_id, channel_id) REFERENCES game_world_raid_channel_runs(firing_id, channel_id)
        ON DELETE RESTRICT
);

-- A channel generation never takes over an older run. It marks that run LOST
-- instead (BOSS-RAID-0 §4.4); terminal runs cannot reacquire runtime authority.
CREATE FUNCTION game_raid_channel_run_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    IF (NEW.firing_id, NEW.channel_id, NEW.owner_generation) IS DISTINCT FROM
       (OLD.firing_id, OLD.channel_id, OLD.owner_generation)
       OR NEW.committed_steps < OLD.committed_steps
       OR NEW.updated_at_ms < OLD.updated_at_ms
       OR (OLD.state <> 1 AND
           (NEW.state, NEW.committed_steps) IS DISTINCT FROM (OLD.state, OLD.committed_steps)) THEN
        RAISE EXCEPTION 'raid run identity, progress and terminal state are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER raid_channel_run_immutable BEFORE UPDATE ON game_world_raid_channel_runs
    FOR EACH ROW EXECUTE FUNCTION game_raid_channel_run_immutable();

REVOKE ALL ON game_world_raid_channel_runs, game_world_raid_step_receipts,
    game_world_raid_creatures FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_raid_channel_runs, game_world_raid_step_receipts,
    game_world_raid_creatures TO oteryn_game_runtime;
GRANT UPDATE (committed_steps, state, updated_at_ms)
    ON game_world_raid_channel_runs TO oteryn_game_runtime;
GRANT UPDATE (state) ON game_world_raid_creatures TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_raid_channel_run_immutable() FROM PUBLIC;
