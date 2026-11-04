-- Crystal 201189f8 data/scripts/actions/items/fluids.lua:66-81 creates item 2886 and
-- schedules its removal after 10 seconds when a filled container is used on the ground.
CREATE TABLE game_item_fluid_splashes (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    item_instance_id UUID NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    placement BYTEA NOT NULL CHECK (octet_length(placement) BETWEEN 1 AND 512),
    fluid_subtype SMALLINT NOT NULL CHECK (fluid_subtype BETWEEN 1 AND 255),
    splash_item_id INTEGER NOT NULL CHECK (splash_item_id = 2886),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK (expires_at = created_at + interval '10 seconds'),
    FOREIGN KEY (transaction_id, item_instance_id)
        REFERENCES game_item_fluid_transitions(transaction_id, item_instance_id)
);
CREATE INDEX game_item_fluid_splashes_expiry
    ON game_item_fluid_splashes(world_id, channel_id, expires_at);
REVOKE ALL ON game_item_fluid_splashes FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_item_fluid_splashes TO oteryn_game_runtime;
GRANT SELECT ON game_item_fluid_splashes TO oteryn_game_control;
