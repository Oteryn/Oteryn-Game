-- P12a: admit Big Boss as stable title id 95.
ALTER TABLE game_character_titles DROP CONSTRAINT game_character_titles_title_check;
ALTER TABLE game_character_titles ADD CONSTRAINT game_character_titles_title_check
 CHECK (title BETWEEN 1 AND 95);
ALTER TABLE game_character_title_selection DROP CONSTRAINT game_character_title_selection_title_check;
ALTER TABLE game_character_title_selection ADD CONSTRAINT game_character_title_selection_title_check
 CHECK (title IS NULL OR title BETWEEN 1 AND 95);

REVOKE ALL ON game_character_titles, game_character_title_selection FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_titles TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_title_selection TO oteryn_game_runtime;
GRANT SELECT ON game_character_titles, game_character_title_selection TO oteryn_game_control;
