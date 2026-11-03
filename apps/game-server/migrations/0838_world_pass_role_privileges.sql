-- World-owned passes run on the node's configured database login (oteryn_game_runtime in node
-- boot). PostgreSQL requires UPDATE privilege on a table whose rows are selected with a
-- row-locking clause, even when the statement only reads or deletes them:
-- * game_world_reset_active is locked FOR SHARE/FOR UPDATE by the daily boost pass;
-- * the PvP expiry (durability/pvp_expiry.rs) deletes expired unjustified points and revenge
--   marks through a bounded `... FOR UPDATE OF p|m` sub-select.
-- The column grants below only make those row locks legal. They never allow a real write:
-- * the BEFORE UPDATE receipt guard from 0081 still rejects every UPDATE of an unjustified
--   point or revenge mark;
-- * the reset pointer keeps no table-level UPDATE grant (planned_world_reset_source_map pins
--   it), and the guard below rejects an UPDATE by anyone but the table owner, so the pointer
--   still only moves through the SECURITY DEFINER game_world_reset_activate.
REVOKE ALL ON game_world_reset_active, game_character_unjustified_points FROM PUBLIC;

GRANT UPDATE (reset_epoch) ON game_world_reset_active TO oteryn_game_runtime, oteryn_game_control;
GRANT DELETE ON game_character_unjustified_points TO oteryn_game_control;
GRANT UPDATE (committed_at) ON game_character_unjustified_points TO oteryn_game_runtime;
GRANT UPDATE (expires_at) ON game_character_revenge_marks TO oteryn_game_runtime;

CREATE FUNCTION game_world_reset_active_owner_writes_only() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $$
BEGIN
  IF NOT pg_has_role(current_user, (SELECT relowner FROM pg_class WHERE oid = TG_RELID), 'MEMBER') THEN
    RAISE EXCEPTION 'the world reset pointer only moves through game_world_reset_activate'
      USING ERRCODE = '42501';
  END IF;
  RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_active_owner_writes_only() FROM PUBLIC;

CREATE TRIGGER game_world_reset_active_owner_writes_only BEFORE UPDATE ON game_world_reset_active
  FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_active_owner_writes_only();
