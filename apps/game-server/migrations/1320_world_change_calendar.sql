-- One calendar decision per committed world reset, shared by all Channels.
CREATE TABLE game_world_change_calendar (
    world_id uuid NOT NULL,
    reset_epoch numeric(20,0) NOT NULL,
    month smallint NOT NULL CHECK (month BETWEEN 1 AND 12),
    day smallint NOT NULL CHECK (day BETWEEN 1 AND 31),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_event_epochs(world_id, reset_epoch)
);
REVOKE ALL ON game_world_change_calendar FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_change_calendar TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_calendar TO oteryn_game_control;

CREATE TABLE game_world_change_states (
    world_id uuid NOT NULL,
    reset_epoch numeric(20,0) NOT NULL,
    change_key text NOT NULL CHECK (length(change_key) BETWEEN 1 AND 100),
    active boolean NOT NULL,
    counter bigint NOT NULL DEFAULT 0 CHECK (counter >= 0),
    PRIMARY KEY(world_id,reset_epoch,change_key),
    FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_calendar(world_id,reset_epoch)
);
REVOKE ALL ON game_world_change_states FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_world_change_states TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_states TO oteryn_game_control;
ALTER TABLE game_world_change_states
    ADD COLUMN creature_key text,
    ADD COLUMN kill_goal bigint CHECK (kill_goal > 0),
    ADD CONSTRAINT world_change_counter_rule CHECK ((creature_key IS NULL) = (kill_goal IS NULL));
CREATE INDEX world_change_creature ON game_world_change_states(world_id,creature_key,reset_epoch)
    WHERE creature_key IS NOT NULL;
