-- Resolve the walk target via the same exact House session fence as display access.
CREATE FUNCTION game_house_podium_target_fenced(
 p_session UUID,p_character UUID,p_connection_generation NUMERIC,p_lease_generation NUMERIC,
 p_scope_generation NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID
) RETURNS BYTEA LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_house_podium_kind_fenced(p_session,p_character,p_connection_generation,p_lease_generation,
    p_scope_generation,p_node,p_node_revision,p_world,p_house,p_item) IS NULL THEN
  RETURN NULL;
 END IF;
 RETURN (SELECT h.spatial_position FROM game_item_house_interior_locations h
  WHERE h.item_instance_id=p_item AND h.world_id=p_world AND h.house_key=p_house);
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_podium_target_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_podium_target_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_podium_target_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID) TO oteryn_game_runtime;
