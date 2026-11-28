-- Character Info reads the target's committed Inbox count.
REVOKE ALL ON TABLE game_character_inbox_counters FROM PUBLIC;
GRANT SELECT (character_id, committed) ON TABLE game_character_inbox_counters TO oteryn_game_runtime;
