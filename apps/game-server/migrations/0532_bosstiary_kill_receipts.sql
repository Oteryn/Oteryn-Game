-- PKG-P4/P02: reuse the character-owned counter from 0445. One receipt per
-- (Character, death occurrence), atomically committed with the count and points.
CREATE TABLE game_character_bosstiary_kill_receipts (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 occurrence_id UUID NOT NULL,
 boss_key TEXT COLLATE "C" NOT NULL,
 category TEXT NOT NULL CHECK (category IN ('bane', 'archfoe', 'nemesis')),
 PRIMARY KEY (character_id, occurrence_id),
 CHECK (boss_key ~ '^[A-Za-z0-9][A-Za-z0-9_.-]*:[A-Za-z0-9_./-]+$' AND octet_length(boss_key) <= 512)
);

REVOKE ALL ON game_character_bosstiary_kill_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_bosstiary_kill_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_bosstiary_kill_receipts TO oteryn_game_control;

CREATE TRIGGER game_character_bosstiary_kill_receipts_immutable
 BEFORE UPDATE OR DELETE ON game_character_bosstiary_kill_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_bosstiary_kill_receipts_no_truncate
 BEFORE TRUNCATE ON game_character_bosstiary_kill_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
