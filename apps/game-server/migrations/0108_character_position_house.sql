-- HOUSE-RUNTIME-1c; CP D1210 / D1168.
-- Historical same-row House binding, not entry or placement authority. This
-- migration must ship with the common native Channel upsert clearing both
-- columns in the same write as cell, vitals and the strictly increasing key.
ALTER TABLE game_character_last_positions
    ADD COLUMN house_world_id UUID NULL,
    ADD COLUMN house_key TEXT NULL,
    ADD CONSTRAINT game_character_last_positions_house CHECK (
        (house_world_id IS NULL AND house_key IS NULL)
        OR (
            house_world_id IS NOT NULL
            AND house_key IS NOT NULL
            AND house_world_id = world_id
            AND game_house_scope_key_valid(house_key)
        )
    );
