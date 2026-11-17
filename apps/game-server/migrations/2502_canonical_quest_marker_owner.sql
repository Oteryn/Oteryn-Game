-- QMECH 2070 now owns marker creation, replay, World binding and client projection.
-- Preserve the earlier qn6 table as read-only audit history; do not run two producers.
DO $migration$
BEGIN
 IF to_regclass('game_character_quest_map_marks') IS NULL OR NOT EXISTS (
   SELECT 1 FROM pg_trigger WHERE tgrelid='game_quest_reward_sources'::regclass
     AND tgname='game_quest_source_map_mark_complete' AND NOT tgisinternal
 ) THEN RAISE EXCEPTION 'canonical personal map marker owner is absent'; END IF;
END $migration$;
DROP TRIGGER game_quest_marker_reward_complete ON game_quest_reward_sources;
REVOKE INSERT ON game_character_quest_markers FROM oteryn_game_runtime;
