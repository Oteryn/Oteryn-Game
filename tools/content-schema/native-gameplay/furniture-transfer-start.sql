-- Actual corpse-entry pickup uses TRANSFER, while ordinary Ground drag uses
-- item-loop movement. Both first-movement callers feed the same absolute owner.
ALTER TABLE game_item_transfer_receipts ADD COLUMN furniture_content_digest BYTEA
 CHECK(furniture_content_digest IS NULL OR octet_length(furniture_content_digest)=32);
-- Physical clock authority is independent of the immutable audit occurrence.
-- Nullable metadata preserves historical receipts without a volatile backfill.
ALTER TABLE game_item_transfer_receipts ADD COLUMN physical_started_at_unix_ms BIGINT
 CHECK(physical_started_at_unix_ms IS NULL OR physical_started_at_unix_ms>0);
CREATE FUNCTION game_srv7_transfer_physical_start_stamp() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF NEW.furniture_content_digest IS NOT NULL OR to_jsonb(NEW)->>'santa_toy_content_digest' IS NOT NULL THEN
  NEW.physical_started_at_unix_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 ELSE NEW.physical_started_at_unix_ms:=NULL; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_srv7_transfer_physical_start_stamp BEFORE INSERT ON game_item_transfer_receipts
 FOR EACH ROW EXECUTE FUNCTION game_srv7_transfer_physical_start_stamp();
REVOKE ALL ON FUNCTION game_srv7_transfer_physical_start_stamp() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_house_furniture_transfer_start(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_transfer_receipts%ROWTYPE; i game_item_instances%ROWTYPE;
 life game_house_furniture_lifetimes%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_transfer_receipts WHERE transaction_id=p_tx;
 IF r.furniture_content_digest IS NULL THEN RETURN; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.created_xact_id<>pg_current_xact_id() OR r.shape<>2 OR r.source_quantity_before<>1
  OR r.source_quantity_after<>1 OR i.lifecycle<>1 OR i.quantity<>1
  OR i.last_transaction_id<>r.transaction_id THEN
  RAISE EXCEPTION 'furniture clock starts only with original exact pickup' USING ERRCODE='23514'; END IF;
 SELECT * INTO life FROM game_house_furniture_lifetimes WHERE item_id IN(9034,9039,35949,35951,35969,35971)
  AND 'oteryn:item.tibia.i'||item_id::text=i.definition_production_key;
 IF life.duration_ms IS NULL OR life.target_id IS NULL THEN
  RAISE EXCEPTION 'furniture pickup duration source changed' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id) THEN RETURN; END IF;
 INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,
  duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,
  target_family,target_production_key,target_revision,content_digest)
 VALUES(i.item_instance_id,r.transaction_id,1,life.duration_ms,r.physical_started_at_unix_ms+life.duration_ms,
  i.definition_family,i.definition_production_key,i.definition_revision_ref,'Item',
  'oteryn:item.tibia.i'||life.target_id::text,i.definition_revision_ref,r.furniture_content_digest);
END $$;
DO $migration$ DECLARE body TEXT; arm TEXT; begin_at INTEGER; BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing absolute Item source proof'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_item_transfer_receipts srv7_furniture_receipt JOIN game_item_instances i
  ON i.item_instance_id=srv7_furniture_receipt.source_item_instance_id JOIN game_house_furniture_lifetimes life
  ON 'oteryn:item.tibia.i'||life.item_id::text=i.definition_production_key
  WHERE srv7_furniture_receipt.transaction_id=NEW.source_transaction_id AND NEW.source_ordinal=1
   AND srv7_furniture_receipt.source_item_instance_id=NEW.item_instance_id AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id()
   AND NEW.created_xact_id=pg_current_xact_id() AND srv7_furniture_receipt.shape=2 AND srv7_furniture_receipt.source_quantity_before=1
   AND srv7_furniture_receipt.source_quantity_after=1 AND i.lifecycle=1 AND i.quantity=1 AND i.last_transaction_id=srv7_furniture_receipt.transaction_id
   AND life.item_id IN(9034,9039,35949,35951,35969,35971)
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
    (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
   AND NEW.duration_millis=life.duration_ms AND NEW.expires_at_unix_ms=srv7_furniture_receipt.physical_started_at_unix_ms+life.duration_ms
   AND NEW.target_family='Item' AND NEW.target_production_key='oteryn:item.tibia.i'||life.target_id::text
   AND NEW.target_revision=NEW.definition_revision AND NEW.content_digest=srv7_furniture_receipt.furniture_content_digest
   AND EXISTS(SELECT 1 FROM game_item_container_entries srv7_furniture_entry WHERE srv7_furniture_entry.item_instance_id=i.item_instance_id
    AND srv7_furniture_entry.character_id=srv7_furniture_receipt.character_id AND srv7_furniture_entry.parent_item_instance_id=srv7_furniture_receipt.destination_parent_item_instance_id
    AND srv7_furniture_entry.placement_ordinal=srv7_furniture_receipt.destination_ordinal AND srv7_furniture_entry.placed_transaction_id=srv7_furniture_receipt.transaction_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id))
 THEN RETURN NULL; END IF;
 $arm$;
 EXECUTE substring(body FROM 1 FOR begin_at+4)||E'\n'||arm||substring(body FROM begin_at+5);
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_transfer_start(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_transfer_start(uuid) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_transfer_start(uuid) TO oteryn_game_runtime;
CREATE FUNCTION game_house_furniture_transfer_clock_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=NEW.source_item_instance_id
  AND i.definition_production_key IN('oteryn:item.tibia.i9034','oteryn:item.tibia.i9039',
   'oteryn:item.tibia.i35949','oteryn:item.tibia.i35951','oteryn:item.tibia.i35969','oteryn:item.tibia.i35971'))
  -- A previously sealed lifetime does not depend on this new movement's
  -- optional content stamp. Creation still requires the source-qualified digest
  -- in game_house_furniture_transfer_start and its same-transaction proof.
  AND NOT (
   EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=NEW.source_item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=NEW.source_item_instance_id))
 THEN RAISE EXCEPTION 'source-qualified furniture pickup requires original lifetime' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_furniture_transfer_clock_proven AFTER INSERT ON game_item_transfer_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_furniture_transfer_clock_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_transfer_clock_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_transfer_clock_proven() FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_transfer_clock_proven() TO oteryn_game_runtime;
