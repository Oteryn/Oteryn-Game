-- P12a: append the source-backed titles in CharacterTitle::ALL; ids 1..43 stay stable.
-- The existing session-fenced title owner remains the only writer. No qualification
-- snapshot is persisted: every new predicate reads its existing native owner.
ALTER TABLE game_character_titles DROP CONSTRAINT game_character_titles_title_check;
ALTER TABLE game_character_titles ADD CONSTRAINT game_character_titles_title_check
 CHECK (title BETWEEN 1 AND 84);
ALTER TABLE game_character_title_selection DROP CONSTRAINT game_character_title_selection_title_check;
ALTER TABLE game_character_title_selection ADD CONSTRAINT game_character_title_selection_title_check
 CHECK (title IS NULL OR title BETWEEN 1 AND 84);

REVOKE ALL ON game_character_titles, game_character_title_selection FROM PUBLIC;
