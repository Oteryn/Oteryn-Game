-- Identify an aggression occurrence independently of its relative timer.
ALTER TABLE game_character_pvp_aggression
 ADD COLUMN IF NOT EXISTS situation_id BIGINT GENERATED ALWAYS AS IDENTITY;
CREATE UNIQUE INDEX IF NOT EXISTS game_character_pvp_aggression_situation_unique
 ON game_character_pvp_aggression (situation_id);
