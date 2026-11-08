-- Preserve runtime reset privileges (0148/0838). PostgreSQL FOR SHARE
-- requires UPDATE on a column of each locked relation; expose
-- only the existing read/lock operation, never reset-owner write privileges.
CREATE FUNCTION game_world_reset_read_locked(p_world UUID)
RETURNS TABLE(reset_epoch NUMERIC, activated_at_unix_ms BIGINT)
LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
 SELECT a.reset_epoch, floor(extract(epoch FROM e.activated_at)*1000)::bigint
 FROM game_world_reset_active a
 JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 WHERE a.world_id=p_world FOR SHARE OF a,e
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_read_locked(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_world_reset_read_locked(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_read_locked(UUID) TO oteryn_game_runtime;
