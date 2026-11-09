-- M51: the existing fenced runtime owner performs shared pit entry/release.
-- Crystal201189f data-global/scripts/quests/svargrond_arena/
-- movements_arena_enter.lua:22-30 and movements_arena_pit.lua:52-64.
-- Keep gameplay custody unavailable to the control plane and PUBLIC.
REVOKE ALL ON game_svargrond_pit_leases FROM PUBLIC, oteryn_game_control;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_svargrond_pit_leases TO oteryn_game_runtime;
