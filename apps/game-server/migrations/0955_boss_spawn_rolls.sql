-- BOSS-SPAWN-1, owner decision 2026-11-05. Channel clocks still own actor
-- recovery/death; these rows serialize world-wide admission and retain daily outcomes.
CREATE TABLE game_boss_world_spawns (
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    boss_key TEXT NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    active BOOLEAN NOT NULL DEFAULT FALSE,
    channel_id UUID CHECK (channel_id IS NULL OR game_node_is_uuid_v7(channel_id)),
    spawn_key TEXT CHECK (octet_length(spawn_key) BETWEEN 1 AND 512),
    PRIMARY KEY (world_id, boss_key),
    CHECK (NOT active OR (channel_id IS NOT NULL AND spawn_key IS NOT NULL))
);
CREATE INDEX game_boss_world_spawns_clock ON game_boss_world_spawns(world_id, channel_id, spawn_key);

CREATE TABLE game_boss_spawn_rolls (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    spawn_key TEXT NOT NULL CHECK (octet_length(spawn_key) BETWEEN 1 AND 512),
    boss_key TEXT NOT NULL,
    won BOOLEAN NOT NULL,
    blocked BOOLEAN NOT NULL,
    spawned BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (world_id, reset_epoch, spawn_key),
    FOREIGN KEY (world_id, boss_key) REFERENCES game_boss_world_spawns(world_id, boss_key),
    CHECK (NOT spawned OR (won AND NOT blocked))
);
CREATE INDEX game_boss_spawn_rolls_boss ON game_boss_spawn_rolls(world_id, boss_key);

REVOKE ALL ON game_boss_world_spawns, game_boss_spawn_rolls FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_boss_world_spawns, game_boss_spawn_rolls TO oteryn_game_runtime;
GRANT SELECT ON game_boss_world_spawns, game_boss_spawn_rolls TO oteryn_game_control;
