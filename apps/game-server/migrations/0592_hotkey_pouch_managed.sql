-- REVV18B I17a/I17b. Extend installed proof chains; retain all older owner arms.
-- Crystal 63b93e2c src/game/game.cpp:3747-3755 searches the inbox pouch and
-- returns toggled equipment through internalCollectManagedItems.
DO $migration$
DECLARE body TEXT; amended TEXT;
BEGIN
 body:=pg_get_functiondef('game_equipment_nested_removal_guard()'::regprocedure);
 amended:=replace(body,'AND o.owner_character=r.character_id AND o.depth BETWEEN 1 AND 7',
 $arm$AND o.owner_character=r.character_id AND (o.depth BETWEEN 1 AND 7 OR
  (o.depth=0 AND EXISTS(SELECT 1 FROM game_item_character_inbox_locations l
   JOIN game_item_instances p USING(item_instance_id)
   WHERE l.item_instance_id=e.parent_item_instance_id AND l.character_id=r.character_id
    AND l.world_id=r.world_id AND p.world_id=r.world_id AND p.lifecycle=1
    AND p.definition_production_key='oteryn:item.tibia.i23721')))$arm$);
 IF amended=body THEN RAISE EXCEPTION 'equipment source owner anchor missing'; END IF;
 EXECUTE amended;
END $migration$;

-- Exact final nested destination, authenticated by the same equipment receipt.
CREATE FUNCTION game_equipment_managed_placement(r game_character_equipment_receipts)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT r.operation=1 AND r.from_slot IS NOT NULL AND r.to_slot IS NULL
  AND r.swap_item_instance_id IS NULL AND r.created_xact_id=pg_current_xact_id()
  AND convert_from(r.intent,'UTF8')::jsonb -> 'managed_destination' = jsonb_build_object(
   'parent',to_jsonb(ARRAY(SELECT get_byte(uuid_send(r.backpack_item_instance_id),n) FROM generate_series(0,15) n)),
   'ordinal',r.backpack_ordinal)
  AND EXISTS(
   WITH RECURSIVE carried(item,depth) AS (
    SELECT s.item_instance_id,0 FROM game_item_container_slots s WHERE s.character_id=r.character_id
    UNION ALL SELECT e.item_instance_id,c.depth+1 FROM carried c
     JOIN (SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries
      UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries) e
      ON e.parent_item_instance_id=c.item WHERE c.depth<9
   ) SELECT 1 FROM (SELECT count(*) AS items,max(depth) AS depth,
     bool_or(item=r.backpack_item_instance_id) AS has_destination
    FROM (SELECT * FROM carried LIMIT 502) bounded) proof
   WHERE proof.items<=501 AND proof.depth<=8 AND proof.has_destination)
  AND EXISTS(SELECT 1 FROM game_item_nested_entries e
   JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
   JOIN game_item_instances p ON p.item_instance_id=e.parent_item_instance_id
   CROSS JOIN LATERAL game_item_loop_owner(p.item_instance_id) o
   WHERE e.item_instance_id=r.item_instance_id AND e.parent_item_instance_id=r.backpack_item_instance_id
    AND e.world_id=r.world_id AND e.placement_ordinal=r.backpack_ordinal
    AND e.placed_transaction_id=r.transaction_id
    AND i.lifecycle=1 AND i.world_id=r.world_id AND i.last_transaction_id=r.transaction_id
    AND i.state_revision=r.state_revision_before+1
    AND p.lifecycle=1 AND p.world_id=r.world_id
    AND o.owner_character=r.character_id AND o.depth BETWEEN 1 AND 7
    AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=i.item_instance_id)
    AND (SELECT count(*) FROM game_item_nested_entries c WHERE c.parent_item_instance_id=p.item_instance_id)<=20)
$$;

DO $migration$
DECLARE body TEXT; amended TEXT; at_begin INT;
BEGIN
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 at_begin:=strpos(body,E'\nBEGIN');
 IF at_begin=0 THEN RAISE EXCEPTION 'nested entry guard BEGIN missing'; END IF;
 amended:=substring(body FROM 1 FOR at_begin+5)||$arm$
 IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_character_equipment_receipts r
  WHERE r.transaction_id=NEW.placed_transaction_id AND r.item_instance_id=NEW.item_instance_id
   AND r.backpack_item_instance_id=NEW.parent_item_instance_id AND r.world_id=NEW.world_id
   AND r.backpack_ordinal=NEW.placement_ordinal AND game_equipment_managed_placement(r))
 THEN RETURN NULL; END IF;
$arm$||substring(body FROM at_begin+6);
 EXECUTE amended;
 body:=pg_get_functiondef('game_equipment_receipt_proven()'::regprocedure);
 amended:=replace(body,
 $old$OR (NEW.from_slot IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e
                    WHERE e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.backpack_item_instance_id
                      AND e.placement_ordinal=NEW.backpack_ordinal AND e.item_instance_id=i.item_instance_id))$old$,
 $new$OR (NEW.from_slot IS NOT NULL AND (game_equipment_managed_placement(NEW) OR EXISTS(SELECT 1 FROM game_item_container_entries e
                    WHERE e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.backpack_item_instance_id
                      AND e.placement_ordinal=NEW.backpack_ordinal AND e.item_instance_id=i.item_instance_id)))$new$);
 IF amended=body THEN RAISE EXCEPTION 'equipment destination successor anchor missing'; END IF;
 EXECUTE amended;
END $migration$;

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_equipment_nested_removal_guard','game_item_nested_entry_proven',
  'game_equipment_receipt_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_equipment_managed_placement(game_character_equipment_receipts) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_equipment_managed_placement(game_character_equipment_receipts) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_equipment_managed_placement(game_character_equipment_receipts)
 TO oteryn_game_runtime,oteryn_game_control;
