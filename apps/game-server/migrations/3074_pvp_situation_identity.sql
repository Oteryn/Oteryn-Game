-- Identify an aggression occurrence independently of its relative timer.
ALTER TABLE game_character_pvp_aggression
 ADD COLUMN situation_id BIGINT GENERATED ALWAYS AS IDENTITY,
 ADD CONSTRAINT game_character_pvp_aggression_situation_unique UNIQUE (situation_id);
