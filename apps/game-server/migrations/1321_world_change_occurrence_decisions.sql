-- A reset binds its complete native choices and rule catalogue, not only booleans.
CREATE TABLE game_world_change_occurrences (
    world_id uuid NOT NULL,
    reset_epoch numeric(20,0) NOT NULL,
    decision bytea NOT NULL CHECK (octet_length(decision) BETWEEN 1 AND 262144),
    decision_digest bytea NOT NULL CHECK (octet_length(decision_digest) = 32),
    PRIMARY KEY(world_id,reset_epoch),
    FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_calendar(world_id,reset_epoch)
);
REVOKE ALL ON game_world_change_occurrences FROM PUBLIC;
GRANT SELECT,INSERT ON game_world_change_occurrences TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_occurrences TO oteryn_game_control;
ALTER TABLE game_world_change_states
    ADD COLUMN definition bytea CHECK (definition IS NULL OR octet_length(definition) BETWEEN 1 AND 16384),
    ADD COLUMN goal_active boolean NOT NULL DEFAULT true;
