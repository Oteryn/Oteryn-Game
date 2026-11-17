-- M51: exclusive shared-pit custody; source Crystal201189f
-- data-global/lib/quests/svargrond_arena.lua:193-256 and
-- scripts/quests/svargrond_arena/movements_arena_enter.lua:22-30.
-- Character progress, fees and receipts stay with the existing arena owner.
CREATE TABLE game_svargrond_pit_leases (
    world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK(game_character_is_uuid_v7(channel_id)),
    pit SMALLINT NOT NULL CHECK(pit BETWEEN 1 AND 10),
    character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
    run_id UUID NOT NULL CHECK(game_character_is_uuid_v7(run_id)),
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
    expires_at_ms BIGINT NOT NULL CHECK(expires_at_ms>0),
    PRIMARY KEY(world_id,channel_id,pit)
);
