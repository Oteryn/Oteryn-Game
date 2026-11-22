-- The Store fulfillment owner can provision this projection independently of
-- NPC purchases. The gameplay runtime only reads it and cannot grant copies.
CREATE TABLE game_character_store_blessing_copies (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    blessing_key TEXT NOT NULL CHECK (blessing_key ~ '^oteryn:blessing[.][a-z_]{1,111}$'),
    copies SMALLINT NOT NULL CHECK (copies BETWEEN 1 AND 255),
    PRIMARY KEY (character_id, blessing_key)
);

REVOKE ALL ON game_character_store_blessing_copies FROM PUBLIC;
GRANT SELECT ON game_character_store_blessing_copies TO oteryn_game_runtime;
