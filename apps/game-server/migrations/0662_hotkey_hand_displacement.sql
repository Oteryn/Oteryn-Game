-- HOTKEYC I17c: one extra conflicting hand belongs to the existing hotkey receipt.
-- The ordinary target-slot swap and every historical proof arm remain unchanged.
CREATE TABLE game_equipment_hand_displacements (
 transaction_id UUID PRIMARY KEY REFERENCES game_character_equipment_receipts(transaction_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 from_slot SMALLINT NOT NULL CHECK(from_slot IN (5,6)),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 1 AND 18446744073709551614),
 backpack_ordinal NUMERIC(20,0) NOT NULL CHECK(backpack_ordinal BETWEEN 1 AND 18446744073709551615),
 quantity_before BIGINT NOT NULL CHECK(quantity_before>0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_equipment_hand_displacements_item_idx ON game_equipment_hand_displacements(item_instance_id);
REVOKE ALL ON game_equipment_hand_displacements FROM PUBLIC;
GRANT SELECT,INSERT ON game_equipment_hand_displacements TO oteryn_game_runtime;

-- Capture the actual equipped predecessor before any custody edge changes.
CREATE FUNCTION game_equipment_hand_displacement_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN
  RAISE EXCEPTION 'equipment hand displacement is immutable' USING ERRCODE='23514';
 END IF;
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
  SELECT 1 FROM game_character_equipment_receipts r
   JOIN game_character_equipment_slots s ON s.character_id=r.character_id
   JOIN game_item_instances i ON i.item_instance_id=s.item_instance_id
  WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=NEW.created_xact_id
   AND r.operation=1 AND r.from_slot IS NULL AND r.to_slot IN (5,6)
   AND r.to_slot<>NEW.from_slot AND s.slot=NEW.from_slot AND s.item_instance_id=NEW.item_instance_id
   AND s.world_id=r.world_id AND i.world_id=r.world_id AND i.lifecycle=1
   AND i.state_revision=NEW.state_revision_before AND i.quantity=NEW.quantity_before
   AND i.item_instance_id<>r.item_instance_id AND i.item_instance_id IS DISTINCT FROM r.swap_item_instance_id
   AND NEW.backpack_ordinal>r.backpack_ordinal
   AND (r.swap_backpack_ordinal IS NULL OR NEW.backpack_ordinal>r.swap_backpack_ordinal)
   AND (convert_from(r.intent,'UTF8')::jsonb #> '{intent,Hotkey}' IS NOT NULL
    OR convert_from(r.intent,'UTF8')::jsonb #> '{intent,HotkeyMove}' IS NOT NULL)
   AND convert_from(r.intent,'UTF8')::jsonb -> 'hand_displacement'=jsonb_build_object(
    'item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.item_instance_id),n) FROM generate_series(0,15) n)),
    'from_slot',NEW.from_slot,'state_revision',NEW.state_revision_before,'quantity',NEW.quantity_before,
    'parent',to_jsonb(ARRAY(SELECT get_byte(uuid_send(r.backpack_item_instance_id),n) FROM generate_series(0,15) n)),
    'ordinal',NEW.backpack_ordinal)
 ) THEN RAISE EXCEPTION 'equipment displacement lacks exact hotkey predecessor' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_equipment_hand_displacement_admit BEFORE INSERT OR UPDATE OR DELETE
 ON game_equipment_hand_displacements FOR EACH ROW EXECUTE FUNCTION game_equipment_hand_displacement_admit();

-- Deferred successor proof, shared by the existing item/location/receipt guard chains.
CREATE FUNCTION game_equipment_hand_displacement_complete(p_transaction UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM game_equipment_hand_displacements d
  JOIN game_character_equipment_receipts r USING(transaction_id)
  JOIN game_item_instances i ON i.item_instance_id=d.item_instance_id
  JOIN game_item_container_entries e ON e.item_instance_id=d.item_instance_id
  WHERE d.transaction_id=p_transaction AND d.created_xact_id=pg_current_xact_id()
   AND r.created_xact_id=d.created_xact_id AND i.lifecycle=1 AND i.world_id=r.world_id
   AND i.quantity=d.quantity_before AND i.state_revision=d.state_revision_before+1
   AND i.last_transaction_id=r.transaction_id AND e.placed_transaction_id=r.transaction_id
   AND e.character_id=r.character_id AND e.world_id=r.world_id
   AND e.parent_item_instance_id=r.backpack_item_instance_id AND e.placement_ordinal=d.backpack_ordinal
   AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=d.item_instance_id)
   AND EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.character_id=r.character_id AND s.item_instance_id=e.parent_item_instance_id)
   AND (SELECT count(*) FROM game_item_container_entries b WHERE b.parent_item_instance_id=e.parent_item_instance_id)<=20
   AND game_item_loop_tree_size(r.character_id)<=500)
$$;
CREATE FUNCTION game_equipment_hand_displacement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF NOT game_equipment_hand_displacement_complete(NEW.transaction_id) THEN
  RAISE EXCEPTION 'equipment displacement lacks exact successor' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_equipment_hand_displacement_proven AFTER INSERT
 ON game_equipment_hand_displacements DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_equipment_hand_displacement_proven();

DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; at_begin INT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_change_proven','game_equipment_slot_proven','game_item_placement_proven','game_equipment_receipt_proven'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  at_begin:=strpos(body,E'\nBEGIN');
  IF at_begin=0 THEN RAISE EXCEPTION 'equipment guard BEGIN missing: %',name; END IF;
  arm:=CASE name
   WHEN 'game_item_instance_change_proven' THEN $arm$
 IF NEW.quantity=OLD.quantity AND NEW.lifecycle=OLD.lifecycle AND NEW.state_revision=OLD.state_revision+1
  AND EXISTS(SELECT 1 FROM game_equipment_hand_displacements d
   WHERE d.item_instance_id=NEW.item_instance_id AND d.transaction_id=NEW.last_transaction_id
    AND d.state_revision_before=OLD.state_revision AND game_equipment_hand_displacement_complete(d.transaction_id))
 THEN RETURN NULL; END IF;
$arm$
   WHEN 'game_equipment_slot_proven' THEN $arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_equipment_hand_displacements d
  JOIN game_character_equipment_receipts r USING(transaction_id)
  WHERE d.item_instance_id=OLD.item_instance_id AND d.from_slot=OLD.slot
   AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id
   AND game_equipment_hand_displacement_complete(d.transaction_id)) THEN RETURN NULL; END IF;
$arm$
   WHEN 'game_item_placement_proven' THEN $arm$
 IF TG_TABLE_NAME='game_item_container_entries' THEN
  IF EXISTS(SELECT 1 FROM game_equipment_hand_displacements d
   JOIN game_character_equipment_receipts r USING(transaction_id)
   WHERE d.transaction_id=NEW.placed_transaction_id AND d.item_instance_id=NEW.item_instance_id
    AND d.backpack_ordinal=NEW.placement_ordinal AND r.backpack_item_instance_id=NEW.parent_item_instance_id
    AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
    AND game_equipment_hand_displacement_complete(d.transaction_id)) THEN RETURN NULL; END IF;
 END IF;
$arm$
   ELSE $arm$
 IF convert_from(NEW.intent,'UTF8')::jsonb ? 'hand_displacement'
  AND NOT game_equipment_hand_displacement_complete(NEW.transaction_id) THEN
  RAISE EXCEPTION 'equipment receipt lacks hand displacement' USING ERRCODE='23514';
 END IF;
$arm$ END;
  EXECUTE substring(body FROM 1 FOR at_begin+5)||arm||substring(body FROM at_begin+6);
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
 END LOOP;
 FOREACH name IN ARRAY ARRAY['game_equipment_hand_displacement_admit','game_equipment_hand_displacement_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_equipment_hand_displacement_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_equipment_hand_displacement_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_equipment_hand_displacement_complete(UUID) TO oteryn_game_runtime;
