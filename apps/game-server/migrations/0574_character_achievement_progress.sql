-- P11: per-Character progress toward a counted Achievement. Crystal 201189f8
-- `data/scripts/lib/register_achievements.lua:696` keeps one count per player and
-- Achievement, adds one per event, grants at the total and stops counting there.
-- The row is written by `durability::account_achievement::add_progress` in the
-- counted event's own fenced transaction; the grant goes through the existing
-- account achievement owner (0021).
CREATE TABLE game_character_achievement_progress (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    achievement_key TEXT NOT NULL
        CHECK (octet_length(achievement_key) <= 160
               AND achievement_key ~ '^oteryn:achievement/[a-z0-9]+(_[a-z0-9]+)*$'),
    progress BIGINT NOT NULL CHECK (progress BETWEEN 1 AND 100000),
    PRIMARY KEY (character_id, achievement_key)
);
REVOKE ALL ON game_character_achievement_progress FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_achievement_progress TO oteryn_game_runtime;
GRANT SELECT ON game_character_achievement_progress TO oteryn_game_control;
