CREATE TABLE game_character_quick_loot_obtain (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id)
        ON DELETE CASCADE,
    category SMALLINT NOT NULL CHECK (category BETWEEN -1 AND 4),
    container_id UUID,
    gold_pouch BOOLEAN NOT NULL,
    PRIMARY KEY (character_id, category),
    CHECK (NOT gold_pouch OR container_id IS NOT NULL)
);
REVOKE ALL ON game_character_quick_loot_obtain FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_quick_loot_obtain TO oteryn_game_runtime;
GRANT SELECT ON game_character_quick_loot_obtain TO oteryn_game_control;
