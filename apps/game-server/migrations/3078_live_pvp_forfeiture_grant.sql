-- The session-fenced live harm writer sets this flag with the PvP deadlines.
-- The PvP state trigger still forbids clearing a recorded forfeiture.
GRANT UPDATE (adventurer_forfeited) ON game_character_pvp_state TO oteryn_game_runtime;
