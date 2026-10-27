-- BOSS-ENCOUNTER-1: shared clock, eligibility and per-character cooldown storage.
-- Decisions BOSS-RAID-0 §§5, 6.3, 9 and owner decisions 23/24.
-- No room is represented by a channel map area or a house identity.
-- Rollback is forward-only: preserve spent cooldowns and eligibility history.

CREATE TABLE game_boss_spawn_clocks (
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(channel_id)),
    spawn_key TEXT NOT NULL CHECK (octet_length(spawn_key) BETWEEN 1 AND 512),
    cycle NUMERIC(20,0) NOT NULL CHECK (cycle BETWEEN 0 AND 18446744073709551615),
    state SMALLINT NOT NULL CHECK (state IN (1,2,3)),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    due_at BIGINT NULL CHECK (due_at >= 0),
    owner_generation NUMERIC(20,0) NULL CHECK (owner_generation BETWEEN 1 AND 18446744073709551615),
    PRIMARY KEY (world_id, channel_id, spawn_key),
    CHECK ((state = 1 AND due_at IS NOT NULL AND owner_generation IS NULL)
        OR (state = 2 AND due_at IS NULL AND owner_generation IS NOT NULL)
        OR (state = 3 AND due_at IS NULL AND owner_generation IS NULL))
);

CREATE TABLE game_character_boss_cooldowns (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    boss_key TEXT COLLATE "C" NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    available_at BIGINT NOT NULL CHECK (available_at >= 0),
    admission UUID NOT NULL CHECK (game_character_is_uuid_v7(admission)),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    PRIMARY KEY (character_id, boss_key)
);

CREATE TABLE game_character_boss_eligibility (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    boss_key TEXT NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    eligibility_ref UUID NOT NULL CHECK (game_character_is_uuid_v7(eligibility_ref)),
    PRIMARY KEY (character_id, boss_key, eligibility_ref)
);

CREATE TABLE game_character_boss_open_world_eligibility (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    boss_key TEXT NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    next_eligible_at BIGINT NOT NULL CHECK (next_eligible_at >= 0),
    last_credited_epoch NUMERIC(20,0) NULL CHECK (last_credited_epoch BETWEEN 0 AND 18446744073709551615),
    PRIMARY KEY (character_id, boss_key)
);

REVOKE ALL ON game_boss_spawn_clocks, game_character_boss_cooldowns,
    game_character_boss_eligibility, game_character_boss_open_world_eligibility FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_boss_spawn_clocks TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_boss_cooldowns TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_boss_eligibility,
    game_character_boss_open_world_eligibility TO oteryn_game_runtime;
