//! Test-only private pending-arrival schema, reused by native unit qualification.
//! This file declares no production migration, role grant or integration writer.
#![allow(dead_code)]

pub const TEST_SCHEMA: &str = "CREATE TABLE game_character_pending_arrivals (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    occurrence_id UUID NOT NULL, world_id UUID NOT NULL,
    tile_x INTEGER NOT NULL, tile_y INTEGER NOT NULL, tile_floor SMALLINT NOT NULL
)";
