-- BOOSTED-1: one immutable creature/boss choice per World server-save epoch.
CREATE TABLE IF NOT EXISTS game_world_daily_boosts (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20, 0) NOT NULL CHECK (reset_epoch >= 0),
    creature_key TEXT NOT NULL CHECK (octet_length(creature_key) BETWEEN 1 AND 255),
    boss_key TEXT NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 255),
    selected_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch)
        REFERENCES game_world_reset_epochs(world_id, reset_epoch)
        ON DELETE RESTRICT
);

REVOKE ALL ON game_world_daily_boosts FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_daily_boosts TO oteryn_game_runtime;
GRANT SELECT ON game_world_daily_boosts TO oteryn_game_control;
