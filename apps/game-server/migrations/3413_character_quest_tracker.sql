CREATE TABLE game_character_quest_tracker (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    mission_tracking BOOLEAN NOT NULL,
    selection JSONB NOT NULL CHECK (
        jsonb_typeof(selection) = 'array'
        AND jsonb_array_length(selection) <= CASE WHEN mission_tracking THEN 25 ELSE 10 END
    ),
    PRIMARY KEY (character_id, mission_tracking)
);

REVOKE ALL ON game_character_quest_tracker FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_quest_tracker TO oteryn_game_runtime;
GRANT SELECT ON game_character_quest_tracker TO oteryn_game_control;
