-- TOOLS2FIX: keep the durable custody proof aligned with TerrainTool::Machete.
-- Both the writer and the deferred removal receipt use this function.
CREATE OR REPLACE FUNCTION game_field_machete_carried(wanted_character UUID,wanted_world UUID,wanted_item UUID)
 RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 WITH RECURSIVE path(item,depth) AS (
  SELECT wanted_item,0
  UNION ALL
  SELECT e.parent_item_instance_id,path.depth+1 FROM game_item_nested_entries e
   JOIN path ON e.item_instance_id=path.item WHERE path.depth<16 AND e.world_id=wanted_world
 )
 SELECT EXISTS(SELECT 1 FROM game_item_instances tool
  WHERE tool.item_instance_id=wanted_item AND tool.lifecycle=1 AND tool.world_id=wanted_world
   AND tool.definition_family='Item' AND tool.definition_production_key IN(
    'oteryn:item.tibia.i3308','oteryn:item.tibia.i3330',
    'oteryn:item.tibia.i9594','oteryn:item.tibia.i9596','oteryn:item.tibia.i9598')
   AND EXISTS(SELECT 1 FROM path JOIN game_item_container_entries e ON e.item_instance_id=path.item
    JOIN game_item_container_slots b ON b.item_instance_id=e.parent_item_instance_id
    WHERE e.character_id=wanted_character AND b.character_id=wanted_character
     AND e.world_id=wanted_world AND b.world_id=wanted_world))
$$;

-- CREATE OR REPLACE preserves the existing owner and runtime-only execute grant.
