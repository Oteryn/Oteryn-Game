CREATE TABLE game_character_cyclopedia_tracker (
    character_id uuid NOT NULL REFERENCES game_characters(character_id) ON DELETE CASCADE,
    tracker_kind smallint NOT NULL CHECK (tracker_kind IN (1, 2)),
    creature_key text NOT NULL CHECK (length(creature_key) BETWEEN 1 AND 128),
    PRIMARY KEY (character_id, tracker_kind, creature_key)
);
CREATE INDEX game_character_cyclopedia_tracker_character_idx
    ON game_character_cyclopedia_tracker(character_id);
REVOKE ALL ON game_character_cyclopedia_tracker FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_character_cyclopedia_tracker TO oteryn_game_runtime;
GRANT SELECT ON game_character_cyclopedia_tracker TO oteryn_game_control;
