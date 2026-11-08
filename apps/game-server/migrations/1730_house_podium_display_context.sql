-- CLIENTGAP-SYSTEMS: one exact placed podium identity under the existing current fence.
CREATE FUNCTION game_house_podium_context_fenced(
 p_session UUID,p_character UUID,p_connection_generation NUMERIC,p_lease_generation NUMERIC,
 p_scope_generation NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID)
RETURNS TABLE(podium_kind SMALLINT,definition_key TEXT,definition_revision TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection_generation,p_lease_generation,
  p_scope_generation,p_node,p_node_revision,p_world,p_house);
 RETURN QUERY SELECT game_house_podium_item_kind(i.definition_production_key),
  i.definition_production_key,i.definition_revision_ref
 FROM game_item_instances i JOIN game_item_house_interior_locations h
  ON h.item_instance_id=i.item_instance_id AND h.world_id=i.world_id
 WHERE i.item_instance_id=p_item AND i.world_id=p_world AND h.house_key=p_house
  AND i.lifecycle=1 AND i.definition_family='Item'
  AND game_house_podium_item_kind(i.definition_production_key) IS NOT NULL
 LIMIT 1;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_podium_context_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_podium_context_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_podium_context_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)
 TO oteryn_game_runtime;
