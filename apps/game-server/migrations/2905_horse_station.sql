CREATE TABLE game_horse_stations (
    world_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(world_id)),
    loose_until_ms BIGINT CHECK (loose_until_ms >= 0)
);

REVOKE ALL ON game_horse_stations FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_horse_stations TO oteryn_game_runtime;
GRANT SELECT ON game_horse_stations TO oteryn_game_control;
