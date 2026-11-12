WITH RECURSIVE roots AS (
 SELECT item_instance_id,'EQUIPMENT'::text AS root,NULL::integer AS town_id,NULL::smallint AS depot_box,slot AS equipment_slot,'1'::text AS placement_ordinal
 FROM game_character_equipment_slots WHERE character_id=$1::text::uuid AND world_id=$2::text::uuid
 UNION ALL
 -- Native persistent equipment slot 9 is the main backpack (the same mapping
 -- used by character_death/items.rs). Singleton root placements have ordinal 1.
 SELECT item_instance_id,'EQUIPMENT',NULL,NULL,9::smallint,'1' FROM game_item_container_slots WHERE character_id=$1::text::uuid AND world_id=$2::text::uuid
 UNION ALL
 SELECT item_instance_id,'DEPOT',town_id,box_number,NULL,ordinal::text FROM game_item_character_depot_locations WHERE character_id=$1::text::uuid AND world_id=$2::text::uuid
 UNION ALL
 SELECT item_instance_id,'INBOX',NULL,NULL,NULL,ordinal::text FROM game_item_character_inbox_locations WHERE character_id=$1::text::uuid AND world_id=$2::text::uuid
), walk AS (
 SELECT r.*,NULL::uuid AS parent_item_instance_id,ARRAY[r.item_instance_id] AS path,1 AS depth,false AS cycle FROM roots r
 UNION ALL
 SELECT e.item_instance_id,w.root,w.town_id,w.depot_box,w.equipment_slot,e.placement_ordinal::text,e.parent_item_instance_id,w.path||e.item_instance_id,w.depth+1,e.item_instance_id=ANY(w.path)
 FROM walk w JOIN LATERAL (
 SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_container_entries
 WHERE parent_item_instance_id=w.item_instance_id AND character_id=$1::text::uuid AND world_id=$2::text::uuid
 UNION ALL
 SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries
 WHERE parent_item_instance_id=w.item_instance_id AND world_id=$2::text::uuid
 ) e ON true WHERE w.depth<=32 AND NOT w.cycle
)
SELECT w.item_instance_id::text,w.root,w.town_id,w.depot_box,w.equipment_slot,w.placement_ordinal,w.parent_item_instance_id::text,w.depth,w.cycle,
 i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity::bigint,i.lifecycle,i.world_id::text AS item_world_id
FROM walk w JOIN game_item_instances i USING(item_instance_id)
LIMIT 524289
