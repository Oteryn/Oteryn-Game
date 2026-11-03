-- World-owned passes run on the node's configured database login. PostgreSQL requires UPDATE
-- privilege even when game_world_reset_active is only selected with a row-locking clause.
REVOKE ALL ON game_world_reset_active, game_character_unjustified_points FROM PUBLIC;

GRANT UPDATE ON game_world_reset_active TO oteryn_game_runtime, oteryn_game_control;
GRANT DELETE ON game_character_unjustified_points TO oteryn_game_control;
