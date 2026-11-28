-- The daily contents of the one native left chest belong to its Channel.
-- Character quest receipts still record who collected the letter and the fish.
CREATE TABLE game_captain_iglue_chest_stock (
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(channel_id)),
    collected_at BIGINT NOT NULL CHECK (collected_at >= 0),
    PRIMARY KEY (world_id, channel_id)
);
REVOKE ALL ON game_captain_iglue_chest_stock FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_captain_iglue_chest_stock TO oteryn_game_runtime;
GRANT SELECT ON game_captain_iglue_chest_stock TO oteryn_game_control;
