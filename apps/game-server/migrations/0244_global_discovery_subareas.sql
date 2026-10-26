-- DISCOVERY-1: character-private progress is keyed by admitted native subarea.
ALTER TABLE game_character_cyclopedia
 DROP CONSTRAINT game_character_cyclopedia_pkey,
 ADD COLUMN region BIGINT NOT NULL CHECK(region BETWEEN 1 AND 4294967295) DEFAULT 1,
 ADD COLUMN subarea BIGINT NOT NULL CHECK(subarea BETWEEN 1 AND 4294967295) DEFAULT 1,
 ADD PRIMARY KEY(character_id,subarea);

ALTER TABLE game_character_cyclopedia
 ALTER COLUMN region DROP DEFAULT,
 ALTER COLUMN subarea DROP DEFAULT;
