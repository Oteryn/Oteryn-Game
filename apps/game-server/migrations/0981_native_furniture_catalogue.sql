-- SRX-1: read-only, current-House catalogue for native command 402 selection.
CREATE FUNCTION game_house_furniture_catalogue(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
RETURNS TABLE(item UUID,definition TEXT,revision NUMERIC,action SMALLINT,acl NUMERIC,target_key TEXT,target_revision TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE access RECORD;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 SELECT a.*,count(*) OVER() AS grants INTO access FROM game_house_access(p_world,p_house,p_character) a;
 IF NOT FOUND OR access.grants<>1 OR access.content_fenced IS DISTINCT FROM false
    OR access.role IS NULL OR access.acl_revision IS NULL OR access.role NOT IN('OWNER','SUBOWNER') THEN
  RAISE EXCEPTION 'WRAP_NO_ACCESS';
 END IF;
 RETURN QUERY
 WITH custody AS (
  SELECT h.item_instance_id,1::smallint AS kind
   FROM game_item_house_interior_locations h WHERE h.world_id=p_world AND h.house_key=p_house
  UNION ALL
  SELECT e.item_instance_id,2::smallint
   FROM game_item_container_entries e JOIN game_item_container_slots b
    ON b.item_instance_id=e.parent_item_instance_id AND b.world_id=e.world_id AND b.character_id=e.character_id
   WHERE e.world_id=p_world AND e.character_id=p_character
 )
 SELECT i.item_instance_id,i.definition_production_key,i.state_revision,c.kind,access.acl_revision,
   origin.target_key,origin.target_revision
 FROM custody c JOIN game_item_instances i ON i.item_instance_id=c.item_instance_id
 LEFT JOIN game_house_furniture_kit_origins origin ON origin.item_instance_id=i.item_instance_id
 WHERE i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1
   AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
 ORDER BY i.item_instance_id LIMIT 2021;
 IF NOT FOUND THEN
  RETURN QUERY SELECT NULL::uuid,NULL::text,NULL::numeric,NULL::smallint,access.acl_revision,NULL::text,NULL::text;
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_catalogue(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_furniture_catalogue(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_catalogue(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
