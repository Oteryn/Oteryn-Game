-- Personal admission and credited-death state for the existing committed World days.
CREATE TABLE game_daily_variant_warpath (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    last_entry_attempt_ms BIGINT NOT NULL CHECK (last_entry_attempt_ms >= 0)
);

CREATE TABLE game_daily_variant_spider_progress (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    visits BIGINT NOT NULL DEFAULT 0 CHECK (visits >= 0),
    deaths SMALLINT NOT NULL DEFAULT 0 CHECK (deaths BETWEEN 0 AND 3),
    PRIMARY KEY (character_id, world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days(world_id, reset_epoch)
);

CREATE TABLE game_daily_variant_entries (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    source_hash BYTEA NOT NULL CHECK (octet_length(source_hash) = 32),
    admitted BOOLEAN NOT NULL,
    PRIMARY KEY (game_session_id, command_id),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days(world_id, reset_epoch)
);

CREATE TABLE game_daily_variant_spider_deaths (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    occurrence BYTEA NOT NULL CHECK (octet_length(occurrence) = 52),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    PRIMARY KEY (world_id, occurrence),
    FOREIGN KEY (character_id, world_id, reset_epoch)
        REFERENCES game_daily_variant_spider_progress(character_id, world_id, reset_epoch)
);

REVOKE ALL ON game_daily_variant_warpath, game_daily_variant_spider_progress,
    game_daily_variant_entries, game_daily_variant_spider_deaths FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_daily_variant_warpath,
    game_daily_variant_spider_progress TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_daily_variant_entries, game_daily_variant_spider_deaths TO oteryn_game_runtime;
GRANT SELECT ON game_daily_variant_warpath, game_daily_variant_spider_progress,
    game_daily_variant_entries, game_daily_variant_spider_deaths TO oteryn_game_control;
