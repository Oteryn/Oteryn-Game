-- SELECT FOR SHARE in the existing fenced Quest writer requires UPDATE.
-- The immutable-row trigger continues to reject actual marker changes.
GRANT UPDATE ON game_character_quest_map_marks TO oteryn_game_runtime;
