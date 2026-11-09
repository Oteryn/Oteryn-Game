-- M49 needs native elemental corpses dragged onto an altar. A corpse's own
-- MINT proves this source; it never admits that non-pickupable body to inventory.
CREATE FUNCTION game_item_loop_native_corpse_source(p_item UUID) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_instances i
   JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id AND m.transaction_id=i.minted_transaction_id
   JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
   WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
     AND i.definition_revision_ref='definition-r1'
     AND i.definition_production_key IN('oteryn:item.tibia.i8139','oteryn:item.tibia.i8140','oteryn:item.tibia.i8141','oteryn:item.tibia.i8106')
     AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
     AND i.world_id=m.death_world_id AND g.world_id=m.death_world_id
     AND g.scope_kind=1 AND g.channel_id=m.death_channel_id)
$$;
DO $migration$
DECLARE body TEXT; predecessor TEXT:='game_item_loop_map_mint_source(NEW.item_instance_id)'; at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing native corpse Ground predecessor'; END IF;
 body:=replace(body,predecessor,predecessor||' OR game_item_loop_native_corpse_source(NEW.item_instance_id)');
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing native corpse line entry'; END IF;
 body:=overlay(body placing E'BEGIN\n'||$arm$
   IF NEW.from_kind=3 AND game_item_loop_native_corpse_source(NEW.item_instance_id)
     AND (NEW.to_kind<>3 OR NEW.quantity_before<>1 OR NEW.quantity_after<>1) THEN
     RAISE EXCEPTION 'native elemental corpses only move whole between Ground cells' USING ERRCODE='23514';
   END IF;
 $arm$ FROM at FOR 5);
 EXECUTE body;
 EXECUTE format('ALTER FUNCTION game_item_loop_native_corpse_source(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_loop_native_corpse_source(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_loop_native_corpse_source(UUID) TO oteryn_game_runtime;
