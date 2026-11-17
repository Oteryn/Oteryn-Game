CREATE TABLE game_thornfire_progress (
    world_id UUID PRIMARY KEY,
    reset_epoch NUMERIC(20,0) NOT NULL,
    elf_homes BIGINT NOT NULL CHECK (elf_homes BETWEEN 0 AND 18014398509481983),
    requested_at_save BOOLEAN NOT NULL,
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_epochs
);
REVOKE ALL ON game_thornfire_progress FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_thornfire_progress TO oteryn_game_runtime;
GRANT SELECT ON game_thornfire_progress TO oteryn_game_control;

CREATE TABLE game_thornfire_wolf_spawns (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    fire_placement BIGINT NOT NULL CHECK (fire_placement BETWEEN 1 AND 281474976710655),
    fire_revision BIGINT NOT NULL CHECK (fire_revision >= 0),
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation > 0),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id > 0),
    spawn_id UUID NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    floor SMALLINT NOT NULL,
    realized BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (world_id, reset_epoch, fire_placement, fire_revision),
    UNIQUE (game_session_id, command_id),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_epochs
);
REVOKE ALL ON game_thornfire_wolf_spawns FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_thornfire_wolf_spawns TO oteryn_game_runtime;
GRANT SELECT ON game_thornfire_wolf_spawns TO oteryn_game_control;
