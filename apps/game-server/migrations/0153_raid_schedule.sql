-- EVENTS-RAID-1 metadata only. No runtime activation or channel run authority.
CREATE TABLE game_world_raid_schedules (
    world_id UUID NOT NULL,
    raid_key TEXT NOT NULL CHECK (octet_length(raid_key) BETWEEN 3 AND 512),
    revision NUMERIC(20,0) NOT NULL CHECK (revision BETWEEN 1 AND 18446744073709551615),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 1 AND 18446744073709551615),
    check_ordinal NUMERIC(20,0) NOT NULL CHECK (check_ordinal BETWEEN 0 AND 18446744073709551615),
    next_firing_ordinal NUMERIC(20,0) NOT NULL CHECK (next_firing_ordinal BETWEEN 1 AND 18446744073709551615),
    next_check_at_ms BIGINT NOT NULL CHECK (next_check_at_ms >= 60000),
    last_fired_at_ms BIGINT CHECK (last_fired_at_ms >= 0 AND last_fired_at_ms < next_check_at_ms),
    last_fixed_slot_ms BIGINT CHECK (last_fixed_slot_ms >= 0 AND last_fixed_slot_ms < next_check_at_ms),
    retired BOOLEAN NOT NULL,
    PRIMARY KEY (world_id, raid_key)
);

CREATE TABLE game_world_raid_firings (
    world_id UUID NOT NULL,
    raid_key TEXT NOT NULL,
    ordinal NUMERIC(20,0) NOT NULL CHECK (ordinal BETWEEN 1 AND 18446744073709551614),
    firing_id UUID NOT NULL UNIQUE CHECK (substring(firing_id::text, 15, 1) = '7'
        AND substring(firing_id::text, 20, 1) IN ('8', '9', 'a', 'b')),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 1 AND 18446744073709551615),
    fired_at_ms BIGINT NOT NULL CHECK (fired_at_ms >= 0),
    deadline_ms BIGINT NOT NULL CHECK (deadline_ms >= fired_at_ms),
    bundle_digest BYTEA NOT NULL CHECK (octet_length(bundle_digest) = 32),
    state SMALLINT NOT NULL CHECK (state IN (1, 2, 3)), -- RUNNING, DONE, CANCELLED
    PRIMARY KEY (world_id, raid_key, ordinal),
    FOREIGN KEY (world_id, raid_key) REFERENCES game_world_raid_schedules(world_id, raid_key)
        ON DELETE RESTRICT
);
CREATE UNIQUE INDEX game_world_raid_one_running
    ON game_world_raid_firings(world_id, raid_key) WHERE state = 1;

CREATE FUNCTION game_raid_no_delete() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    RAISE EXCEPTION 'raid metadata history cannot be deleted' USING ERRCODE = '23514';
END $$;
CREATE TRIGGER raid_schedules_no_delete BEFORE DELETE OR TRUNCATE ON game_world_raid_schedules
    FOR EACH STATEMENT EXECUTE FUNCTION game_raid_no_delete();
CREATE TRIGGER raid_firings_no_delete BEFORE DELETE OR TRUNCATE ON game_world_raid_firings
    FOR EACH STATEMENT EXECUTE FUNCTION game_raid_no_delete();

CREATE FUNCTION game_raid_firing_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    IF (NEW.world_id, NEW.raid_key, NEW.ordinal, NEW.firing_id, NEW.reset_epoch,
        NEW.fired_at_ms, NEW.deadline_ms, NEW.bundle_digest) IS DISTINCT FROM
       (OLD.world_id, OLD.raid_key, OLD.ordinal, OLD.firing_id, OLD.reset_epoch,
        OLD.fired_at_ms, OLD.deadline_ms, OLD.bundle_digest)
       OR (OLD.state <> 1 AND NEW.state <> OLD.state) THEN
        RAISE EXCEPTION 'raid firing facts and terminal state are immutable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER raid_firing_immutable BEFORE UPDATE ON game_world_raid_firings
    FOR EACH ROW EXECUTE FUNCTION game_raid_firing_immutable();

REVOKE ALL ON game_world_raid_schedules, game_world_raid_firings FROM PUBLIC;
GRANT SELECT ON game_world_raid_schedules, game_world_raid_firings TO oteryn_game_runtime;
GRANT INSERT (world_id, raid_key, revision, reset_epoch, check_ordinal,
    next_firing_ordinal, next_check_at_ms, last_fired_at_ms, last_fixed_slot_ms, retired)
    ON game_world_raid_schedules TO oteryn_game_runtime;
GRANT UPDATE (revision, reset_epoch, check_ordinal, next_firing_ordinal,
    next_check_at_ms, last_fired_at_ms, last_fixed_slot_ms, retired)
    ON game_world_raid_schedules TO oteryn_game_runtime;
GRANT INSERT (world_id, raid_key, ordinal, firing_id, reset_epoch, fired_at_ms,
    deadline_ms, bundle_digest, state) ON game_world_raid_firings TO oteryn_game_runtime;
GRANT UPDATE (state) ON game_world_raid_firings TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_raid_no_delete(), game_raid_firing_immutable() FROM PUBLIC;
