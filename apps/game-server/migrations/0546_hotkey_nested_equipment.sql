-- PKG5R2B I17a: one equipment receipt removes a nested source and equips it.
-- Existing direct-source guards stay in force. The new source evidence is captured
-- before deletion and binds the actual ancestor owner in this physical transaction.
ALTER TABLE game_character_equipment_receipts
 ADD COLUMN nested_source_parent_item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
 ADD CONSTRAINT game_equipment_nested_source_shape CHECK (
  nested_source_parent_item_instance_id IS NULL OR
  (operation=1 AND to_slot IS NOT NULL AND from_slot IS NULL
   AND nested_source_parent_item_instance_id<>backpack_item_instance_id));
CREATE INDEX game_equipment_nested_source_parent_idx
 ON game_character_equipment_receipts(nested_source_parent_item_instance_id)
 WHERE nested_source_parent_item_instance_id IS NOT NULL;

CREATE TABLE game_equipment_nested_removals (
 transaction_id UUID PRIMARY KEY REFERENCES game_character_equipment_receipts(transaction_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL,
 placement_ordinal NUMERIC(20,0) NOT NULL CHECK(placement_ordinal BETWEEN 1 AND 18446744073709551615),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_equipment_nested_removals_item_idx ON game_equipment_nested_removals(item_instance_id);
CREATE INDEX game_equipment_nested_removals_parent_idx ON game_equipment_nested_removals(parent_item_instance_id);
CREATE INDEX game_equipment_nested_removals_character_idx ON game_equipment_nested_removals(character_id);

CREATE FUNCTION game_equipment_nested_removal_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN
  RAISE EXCEPTION 'equipment nested removal evidence is immutable' USING ERRCODE='23514';
 END IF;
 IF pg_trigger_depth()<2 OR NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
  SELECT 1 FROM game_character_equipment_receipts r
   JOIN game_item_nested_entries e ON e.item_instance_id=r.item_instance_id
   CROSS JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) o
  WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
   AND r.operation=1 AND r.to_slot IS NOT NULL
   AND r.item_instance_id=NEW.item_instance_id AND r.character_id=NEW.character_id
   AND r.world_id=NEW.world_id AND e.world_id=NEW.world_id
   AND e.parent_item_instance_id=NEW.parent_item_instance_id
   AND e.placement_ordinal=NEW.placement_ordinal
   AND r.nested_source_parent_item_instance_id=e.parent_item_instance_id
   AND r.backpack_ordinal=e.placement_ordinal
   AND o.owner_character=r.character_id AND o.depth BETWEEN 1 AND 7
   AND EXISTS(SELECT 1 FROM game_item_container_slots s
    WHERE s.character_id=r.character_id AND s.item_instance_id=r.backpack_item_instance_id)
 ) THEN
  RAISE EXCEPTION 'equipment nested evidence requires the actual source owner' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_equipment_nested_removal_guard BEFORE INSERT OR UPDATE OR DELETE
 ON game_equipment_nested_removals FOR EACH ROW EXECUTE FUNCTION game_equipment_nested_removal_guard();

CREATE FUNCTION game_equipment_nested_source_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
  WHERE r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id()
   AND r.nested_source_parent_item_instance_id IS NOT NULL) THEN
  INSERT INTO game_equipment_nested_removals(transaction_id,item_instance_id,parent_item_instance_id,
    character_id,world_id,placement_ordinal)
   SELECT r.transaction_id,OLD.item_instance_id,OLD.parent_item_instance_id,r.character_id,
    OLD.world_id,OLD.placement_ordinal FROM game_character_equipment_receipts r
   WHERE r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id()
    AND r.nested_source_parent_item_instance_id=OLD.parent_item_instance_id
    AND r.backpack_ordinal=OLD.placement_ordinal;
  IF NOT FOUND THEN
   RAISE EXCEPTION 'equipment nested source mismatch' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN OLD;
END $$;
CREATE TRIGGER game_equipment_nested_source_capture BEFORE DELETE ON game_item_nested_entries
 FOR EACH ROW EXECUTE FUNCTION game_equipment_nested_source_capture();

-- Extend the currently installed guard chains, retaining every other owner's arms.
DO $migration$
DECLARE body TEXT; amended TEXT; at_begin INT;
BEGIN
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 at_begin:=strpos(body,E'\nBEGIN');
 IF at_begin=0 THEN RAISE EXCEPTION 'nested entry guard BEGIN missing'; END IF;
 amended:=substring(body FROM 1 FOR at_begin+5)||$arm$
 IF TG_OP='DELETE' AND EXISTS (
  SELECT 1 FROM game_equipment_nested_removals e
   JOIN game_character_equipment_receipts r USING(transaction_id)
   JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
   JOIN game_character_equipment_slots s ON s.item_instance_id=e.item_instance_id
  WHERE e.item_instance_id=OLD.item_instance_id AND e.parent_item_instance_id=OLD.parent_item_instance_id
   AND e.world_id=OLD.world_id AND e.placement_ordinal=OLD.placement_ordinal
   AND e.created_xact_id=pg_current_xact_id() AND r.created_xact_id=e.created_xact_id
   AND r.item_instance_id=e.item_instance_id AND r.nested_source_parent_item_instance_id=e.parent_item_instance_id
   AND r.character_id=e.character_id AND r.world_id=e.world_id AND r.backpack_ordinal=e.placement_ordinal
   AND r.operation=1 AND r.to_slot=s.slot AND s.character_id=e.character_id
   AND s.placed_transaction_id=r.transaction_id AND i.lifecycle=1 AND i.world_id=e.world_id
   AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
 ) THEN RETURN NULL; END IF;
$arm$||substring(body FROM at_begin+6);
 EXECUTE amended;

 body:=pg_get_functiondef('game_equipment_receipt_proven()'::regprocedure);
 at_begin:=strpos(body,E'\nBEGIN');
 IF at_begin=0 THEN RAISE EXCEPTION 'equipment receipt guard BEGIN missing'; END IF;
 amended:=substring(body FROM 1 FOR at_begin+5)||$arm$
 IF NEW.nested_source_parent_item_instance_id IS NOT NULL AND (
  NOT EXISTS(SELECT 1 FROM game_equipment_nested_removals e
   WHERE e.transaction_id=NEW.transaction_id AND e.item_instance_id=NEW.item_instance_id
    AND e.parent_item_instance_id=NEW.nested_source_parent_item_instance_id
    AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
    AND e.placement_ordinal=NEW.backpack_ordinal AND e.created_xact_id=pg_current_xact_id())
  OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=NEW.item_instance_id)
  OR convert_from(NEW.intent,'UTF8')::jsonb -> 'nested_source' IS DISTINCT FROM jsonb_build_object(
   'parent',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.nested_source_parent_item_instance_id),n) FROM generate_series(0,15) n)),
   'ordinal',NEW.backpack_ordinal)
 ) THEN RAISE EXCEPTION 'equipment receipt lacks exact nested source removal' USING ERRCODE='23514'; END IF;
$arm$||substring(body FROM at_begin+6);
 -- The existing swap proof still compares all four exact locations. Only its
 -- incoming source location varies; the displaced item still enters the main bag.
 body:=replace(amended,
  '''operation'',''REMOVE'',''kind'',''MAIN_BACKPACK_ENTRY''',
  '''operation'',''REMOVE'',''kind'',CASE WHEN NEW.nested_source_parent_item_instance_id IS NULL THEN ''MAIN_BACKPACK_ENTRY'' ELSE ''NESTED_CONTAINER_ENTRY'' END');
 IF body=amended THEN RAISE EXCEPTION 'equipment swap source kind anchor missing'; END IF;
 amended:=replace(body,
  'uuid_send(NEW.backpack_item_instance_id),n) FROM generate_series(0,15) n)),''ordinal'',NEW.backpack_ordinal)',
  'uuid_send(COALESCE(NEW.nested_source_parent_item_instance_id,NEW.backpack_item_instance_id)),n) FROM generate_series(0,15) n)),''ordinal'',NEW.backpack_ordinal)');
 IF body=amended THEN RAISE EXCEPTION 'equipment swap source parent anchor missing'; END IF;
 EXECUTE amended;
END $migration$;

REVOKE ALL ON game_equipment_nested_removals FROM PUBLIC;
GRANT SELECT,INSERT ON game_equipment_nested_removals TO oteryn_game_runtime;
GRANT SELECT ON game_equipment_nested_removals TO oteryn_game_control;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_equipment_nested_removal_guard','game_equipment_nested_source_capture',
  'game_item_nested_entry_proven','game_equipment_receipt_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
END $$;
