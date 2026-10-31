-- PAR-P12a: the stable Game title catalogue grows to ids 1..43 (CharacterTitle::ALL).
-- Ids 1..5 retain their meanings; 6..43 follow the Global title table order.
ALTER TABLE game_character_titles DROP CONSTRAINT game_character_titles_title_check;
ALTER TABLE game_character_titles ADD CONSTRAINT game_character_titles_title_check CHECK (title BETWEEN 1 AND 43);
ALTER TABLE game_character_title_selection DROP CONSTRAINT game_character_title_selection_title_check;
ALTER TABLE game_character_title_selection ADD CONSTRAINT game_character_title_selection_title_check CHECK (title IS NULL OR title BETWEEN 1 AND 43);

REVOKE ALL ON game_character_titles, game_character_title_selection FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_titles TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_title_selection TO oteryn_game_runtime;
GRANT SELECT ON game_character_titles, game_character_title_selection TO oteryn_game_control;
