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
