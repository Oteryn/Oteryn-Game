-- P22: Cyclopedia Bestiary (kind 1) and Bosstiary (kind 2) tracker lists, one row per
-- tracked creature (Crystal player_bosstiary.tracker / player_charms.tracker_list).
CREATE TABLE game_character_cyclopedia_tracker (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    tracker_kind SMALLINT NOT NULL CHECK (tracker_kind IN (1, 2)),
    creature_key TEXT COLLATE "C" NOT NULL
        CHECK (creature_key ~ '^[A-Za-z0-9][A-Za-z0-9_./:-]{0,127}$'),
    PRIMARY KEY (character_id, tracker_kind, creature_key)
);

REVOKE ALL ON game_character_cyclopedia_tracker FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_character_cyclopedia_tracker TO oteryn_game_runtime;
GRANT SELECT ON game_character_cyclopedia_tracker TO oteryn_game_control;
