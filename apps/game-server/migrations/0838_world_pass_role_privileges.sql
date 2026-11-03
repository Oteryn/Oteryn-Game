-- World-owned passes run on the node's configured database login (oteryn_game_runtime in node
-- boot). PostgreSQL requires UPDATE privilege on a table whose rows are selected with a
-- row-locking clause, even when the statement only reads or deletes them:
-- * game_world_reset_active is locked FOR UPDATE by the reset pass;
-- * the PvP expiry (durability/pvp_expiry.rs) deletes expired unjustified points and revenge
--   marks through a bounded `... FOR UPDATE OF p|m` sub-select.
-- The column grants below only make those row locks legal: the BEFORE UPDATE receipt guard from
-- 0081 still rejects every UPDATE of an unjustified point or revenge mark.
REVOKE ALL ON game_world_reset_active, game_character_unjustified_points FROM PUBLIC;

GRANT UPDATE ON game_world_reset_active TO oteryn_game_runtime, oteryn_game_control;
GRANT DELETE ON game_character_unjustified_points TO oteryn_game_control;
GRANT UPDATE (committed_at) ON game_character_unjustified_points TO oteryn_game_runtime;
GRANT UPDATE (expires_at) ON game_character_revenge_marks TO oteryn_game_runtime;
