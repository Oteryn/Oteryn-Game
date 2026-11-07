-- BOSSTIARY-1: character-owned boss kills and points. The composite key prevents a
-- second truth for a boss, while non-negative checks make corrupt progress fail at write time.
CREATE TABLE game_character_bosstiary_progress (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 boss_key TEXT COLLATE "C" NOT NULL,
 kill_count BIGINT NOT NULL DEFAULT 0 CHECK (kill_count >= 0),
 boss_points BIGINT NOT NULL DEFAULT 0 CHECK (boss_points >= 0),
 PRIMARY KEY (character_id, boss_key),
 CHECK (boss_key ~ '^[A-Za-z0-9][A-Za-z0-9_.-]*:[A-Za-z0-9_./-]+$' AND octet_length(boss_key) <= 512)
);

REVOKE ALL ON game_character_bosstiary_progress FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_bosstiary_progress TO oteryn_game_runtime;
GRANT SELECT ON game_character_bosstiary_progress TO oteryn_game_control;
