-- SRV6: retain title identity and owner; append Task Board/menu/exploration ids.
ALTER TABLE game_character_titles DROP CONSTRAINT game_character_titles_title_check;
ALTER TABLE game_character_titles ADD CONSTRAINT game_character_titles_title_check
 CHECK (title BETWEEN 1 AND 94);
ALTER TABLE game_character_title_selection DROP CONSTRAINT game_character_title_selection_title_check;
ALTER TABLE game_character_title_selection ADD CONSTRAINT game_character_title_selection_title_check
 CHECK (title IS NULL OR title BETWEEN 1 AND 94);
