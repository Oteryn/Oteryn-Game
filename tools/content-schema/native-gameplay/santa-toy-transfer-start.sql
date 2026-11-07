-- Separate source proof for individually moved Santa toys in the mature House
-- transfer owner. The existing immutable transfer audit timestamp is unchanged.
ALTER TABLE game_item_transfer_receipts
 ADD COLUMN santa_toy_content_digest BYTEA CHECK(santa_toy_content_digest IS NULL OR octet_length(santa_toy_content_digest)=32);
CREATE FUNCTION game_santa_toy_transfer_current(santa_transfer game_item_transfer_receipts,santa_item game_item_instances)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT santa_transfer.created_xact_id=pg_current_xact_id() AND santa_transfer.shape IN(5,6,7)
  AND santa_transfer.source_item_instance_id=santa_item.item_instance_id
  AND santa_transfer.source_quantity_before=1 AND santa_transfer.source_quantity_after=1
  AND santa_item.lifecycle=1 AND santa_item.quantity=1
  AND CASE WHEN santa_transfer.shape IN(5,7) THEN EXISTS(
   SELECT 1 FROM game_item_house_interior_locations santa_house WHERE santa_house.item_instance_id=santa_item.item_instance_id
    AND santa_house.world_id=santa_item.world_id AND santa_house.placed_transaction_id=santa_transfer.transaction_id)
  ELSE EXISTS(SELECT 1 FROM game_item_container_entries santa_entry
   WHERE santa_entry.item_instance_id=santa_item.item_instance_id AND santa_entry.world_id=santa_item.world_id
    AND santa_entry.character_id=santa_transfer.character_id
    AND santa_entry.parent_item_instance_id=santa_transfer.destination_parent_item_instance_id
    AND santa_entry.placement_ordinal=santa_transfer.destination_ordinal
    AND santa_entry.placed_transaction_id=santa_transfer.transaction_id) END
$$;
CREATE FUNCTION game_santa_toy_transfer_start(p_transaction UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE santa_transfer game_item_transfer_receipts%ROWTYPE; santa_item game_item_instances%ROWTYPE; santa_life game_santa_toy_lifetimes%ROWTYPE;
BEGIN
 SELECT * INTO santa_transfer FROM game_item_transfer_receipts WHERE transaction_id=p_transaction;
 IF santa_transfer.transaction_id IS NULL OR santa_transfer.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'Santa toy requires current individual House movement' USING ERRCODE='23514'; END IF;
 IF santa_transfer.santa_toy_content_digest IS NULL THEN RETURN; END IF;
 SELECT * INTO santa_item FROM game_item_instances WHERE item_instance_id=santa_transfer.source_item_instance_id;
 SELECT * INTO santa_life FROM game_santa_toy_lifetimes WHERE item_key=santa_item.definition_production_key;
 IF santa_life.item_key IS NULL OR santa_transfer.physical_started_at_unix_ms IS NULL
  OR game_santa_toy_transfer_current(santa_transfer,santa_item) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Santa toy clock lacks exact current House movement' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=santa_item.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=santa_item.item_instance_id) THEN RETURN; END IF;
 INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,
 duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,
 target_family,target_production_key,target_revision,content_digest)
 VALUES(santa_item.item_instance_id,santa_transfer.transaction_id,1,santa_life.duration_ms,
 santa_transfer.physical_started_at_unix_ms+santa_life.duration_ms,santa_item.definition_family,
 santa_item.definition_production_key,santa_item.definition_revision_ref,'Item',santa_life.target_key,
 santa_item.definition_revision_ref,santa_transfer.santa_toy_content_digest);
END $$;
DO $migration$ DECLARE body TEXT; arm TEXT; at INTEGER; BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing original absolute timer guard'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_item_transfer_receipts santa_transfer
 JOIN game_item_instances santa_item ON santa_item.item_instance_id=santa_transfer.source_item_instance_id
 JOIN game_santa_toy_lifetimes santa_life ON santa_life.item_key=santa_item.definition_production_key
 WHERE santa_transfer.transaction_id=NEW.source_transaction_id AND NEW.source_ordinal=1
  AND santa_item.item_instance_id=NEW.item_instance_id AND NEW.created_xact_id=pg_current_xact_id()
  AND game_santa_toy_transfer_current(santa_transfer,santa_item) IS TRUE
  AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)=(santa_item.definition_family,santa_item.definition_production_key,santa_item.definition_revision_ref)
  AND (NEW.target_family,NEW.target_production_key,NEW.target_revision)=('Item',santa_life.target_key,santa_item.definition_revision_ref)
  AND NEW.duration_millis=santa_life.duration_ms
  AND NEW.expires_at_unix_ms=santa_transfer.physical_started_at_unix_ms+santa_life.duration_ms
  AND NEW.content_digest=santa_transfer.santa_toy_content_digest
  AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=NEW.item_instance_id)) THEN RETURN NULL; END IF;
 $arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from at for 5);
END $migration$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOREACH f IN ARRAY ARRAY['game_santa_toy_transfer_start(uuid)'::regprocedure,'game_santa_toy_transfer_current(game_item_transfer_receipts,game_item_instances)'::regprocedure] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',f);
 EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;
CREATE FUNCTION game_santa_toy_transfer_clock_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.shape IN(5,6,7) AND EXISTS(SELECT 1 FROM game_item_instances santa_item
  JOIN game_santa_toy_lifetimes santa_life ON santa_life.item_key=santa_item.definition_production_key
  WHERE santa_item.item_instance_id=NEW.source_item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=NEW.source_item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=NEW.source_item_instance_id) THEN
  RAISE EXCEPTION 'individual House toy movement requires its original source clock' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_santa_toy_transfer_clock_complete AFTER INSERT ON game_item_transfer_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_santa_toy_transfer_clock_complete();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_santa_toy_transfer_clock_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_santa_toy_transfer_clock_complete() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
