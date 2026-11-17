-- M52: Crystal 9f5a72c globalevents/others/grimvale.lua:8-23.
-- Chance is read before its replacement draw; Active survives boss completion.
CREATE TABLE game_world_feroxa_occurrences (
    world_id uuid NOT NULL,
    reset_epoch numeric(20,0) NOT NULL,
    chance smallint NOT NULL DEFAULT -1 CHECK (chance = -1 OR chance BETWEEN 1 AND 10),
    phase smallint NOT NULL DEFAULT 0 CHECK (phase BETWEEN 0 AND 3),
    selected_at_ms bigint,
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_epochs(world_id, reset_epoch),
    CHECK ((phase = 0 AND selected_at_ms IS NULL)
        OR (phase > 0 AND selected_at_ms IS NOT NULL AND selected_at_ms >= 0))
);
REVOKE ALL ON game_world_feroxa_occurrences FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_world_feroxa_occurrences TO oteryn_game_runtime;
GRANT SELECT ON game_world_feroxa_occurrences TO oteryn_game_control;
