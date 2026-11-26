-- A reset can carry the current House catalogue through a same-content epoch
-- change. Each array element names one durable tile with its exact House key
-- and lowercase hex spatial-position bytes. Custody must remain on those tiles.
CREATE FUNCTION game_world_reset_house_qualified(p_world UUID,p_tiles JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql AS $$
BEGIN
 IF jsonb_typeof(p_tiles) IS DISTINCT FROM 'array' THEN RETURN FALSE; END IF;
 RETURN NOT EXISTS(
  WITH supplied AS (SELECT value AS tile FROM jsonb_array_elements(p_tiles)),
  installed AS (
   SELECT jsonb_build_object('house_key',house_key,'spatial_position',encode(spatial_position,'hex')) AS tile
   FROM game_house_tiles WHERE world_id=p_world
  )
  (SELECT tile FROM supplied EXCEPT ALL SELECT tile FROM installed)
  UNION ALL
  (SELECT tile FROM installed EXCEPT ALL SELECT tile FROM supplied)
 ) AND NOT EXISTS(
  SELECT 1 FROM game_house_ownership o WHERE o.world_id=p_world
   AND NOT EXISTS(SELECT 1 FROM game_house_tiles t WHERE (t.world_id,t.house_key)=(o.world_id,o.house_key))
 ) AND NOT EXISTS(
  SELECT 1 FROM game_item_house_interior_locations l WHERE l.world_id=p_world
   AND NOT EXISTS(SELECT 1 FROM game_house_tiles t
    WHERE (t.world_id,t.house_key,t.spatial_position)=(l.world_id,l.house_key,l.spatial_position))
 );
END $$;
REVOKE ALL ON FUNCTION game_world_reset_house_qualified(UUID,JSONB) FROM PUBLIC;

-- Reuse the same House guard from Rust without granting access to custody.
-- The runtime already has SELECT on the reset jobs returned here.
CREATE FUNCTION game_world_reset_house_preflight(p_world UUID,p_epoch NUMERIC)
RETURNS SETOF game_world_reset_jobs LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE job game_world_reset_jobs%ROWTYPE;
BEGIN
 SELECT * INTO job FROM game_world_reset_jobs WHERE world_id=p_world AND reset_epoch=p_epoch FOR UPDATE;
 IF NOT FOUND THEN RETURN; END IF;
 LOCK TABLE game_house_properties,game_house_ownership,game_house_tiles,game_item_house_interior_locations IN SHARE MODE;
 IF game_world_reset_house_qualified(p_world,job.target_house_tiles) THEN
  RETURN NEXT job;
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_house_preflight(UUID,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_house_preflight(UUID,NUMERIC) TO oteryn_game_runtime;

-- Extend the installed CREATE OR REPLACE definitions, retaining their transaction
-- boundaries, search paths, ACLs and the Berlin calendar change from 0562.
DO $$
DECLARE
 procedure REGPROCEDURE;
 definition TEXT;
 old_guard TEXT;
 new_guard TEXT;
 old_lock CONSTANT TEXT := 'LOCK TABLE game_item_house_interior_locations IN SHARE MODE;';
 new_lock CONSTANT TEXT := 'LOCK TABLE game_house_properties,game_house_ownership,game_house_tiles,game_item_house_interior_locations IN SHARE MODE;';
BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_house_qualified(UUID,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_world_reset_house_preflight(UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());

 FOREACH procedure IN ARRAY ARRAY[
  'game_world_reset_publish(uuid,numeric,bytea,text,bytea,bytea,bytea,jsonb,numeric)'::regprocedure,
  'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)'::regprocedure
 ] LOOP
  SELECT pg_get_functiondef(procedure) INTO STRICT definition;
  IF procedure='game_world_reset_publish(uuid,numeric,bytea,text,bytea,bytea,bytea,jsonb,numeric)'::regprocedure THEN
   old_guard := $guard$IF p_house_tiles IS DISTINCT FROM '[]'::jsonb OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=p_world)
  OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=p_world) THEN$guard$;
   new_guard := $guard$IF NOT game_world_reset_house_qualified(p_world,p_house_tiles)
  OR (p_house_tiles<>'[]'::jsonb AND (e.map_revision,e.server_digest,e.client_digest,e.frame_digest)
      IS DISTINCT FROM (p_map,p_server,p_client,p_frame)) THEN$guard$;
  ELSE
   old_guard := $guard$IF j.target_house_tiles<>'[]'::jsonb OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=p_world)
  OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=p_world) THEN$guard$;
   new_guard := $guard$IF NOT game_world_reset_house_qualified(p_world,j.target_house_tiles)
  OR (j.target_house_tiles<>'[]'::jsonb AND NOT EXISTS(
      SELECT 1 FROM game_world_reset_epochs predecessor
       WHERE predecessor.world_id=p_world AND predecessor.reset_epoch=p_epoch
       AND (predecessor.map_revision,predecessor.server_digest,predecessor.client_digest,predecessor.frame_digest)
       =(p_map,p_server,p_client,p_frame))) THEN$guard$;
  END IF;
  IF strpos(definition,old_guard)=0 OR strpos(definition,old_lock)=0 THEN
   RAISE EXCEPTION 'unexpected World reset House guard in %',procedure;
  END IF;
  EXECUTE replace(replace(definition,old_guard,new_guard),old_lock,new_lock);
 END LOOP;
END $$;

REVOKE ALL ON FUNCTION game_world_reset_publish(UUID,NUMERIC,BYTEA,TEXT,BYTEA,BYTEA,BYTEA,JSONB,NUMERIC) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_world_reset_activate(UUID,NUMERIC,BYTEA,BYTEA,TEXT,BYTEA,BYTEA,BYTEA,UUID,NUMERIC) FROM PUBLIC;
