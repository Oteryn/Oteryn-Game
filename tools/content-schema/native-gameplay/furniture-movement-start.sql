-- A loop receipt's physical movement time is server authority. Its existing
-- DEFAULT is insufficient when the runtime role supplies an explicit timestamp.
CREATE FUNCTION game_srv7_loop_movement_clock_stamp() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 NEW.created_at:=clock_timestamp();
 RETURN NEW;
END $$;
CREATE TRIGGER game_srv7_loop_movement_clock_stamp BEFORE INSERT ON game_item_loop_move_receipts
 FOR EACH ROW EXECUTE FUNCTION game_srv7_loop_movement_clock_stamp();

REVOKE ALL ON FUNCTION game_srv7_loop_movement_clock_stamp() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Crystal game.cpp:2418/2426 starts Item decay after a real movement.
-- Container:addLoot merely creates the default duration; it does not start it.
CREATE FUNCTION game_house_furniture_movement_start(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_loop_move_receipts%ROWTYPE; l game_item_loop_move_lines%ROWTYPE;
 life game_house_furniture_lifetimes%ROWTYPE; started BIGINT;
BEGIN
 SELECT * INTO r FROM game_item_loop_move_receipts WHERE transaction_id=p_tx;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'furniture duration starts only with original movement' USING ERRCODE='23514'; END IF;
 started:=floor(extract(epoch FROM r.created_at)*1000)::bigint;
 FOR l IN SELECT * FROM game_item_loop_move_lines WHERE transaction_id=p_tx
  AND definition_production_key IN('oteryn:item.tibia.i9034','oteryn:item.tibia.i9039',
   'oteryn:item.tibia.i35949','oteryn:item.tibia.i35951','oteryn:item.tibia.i35969','oteryn:item.tibia.i35971')
  AND quantity_before=1 AND quantity_after=1 AND from_kind<>0 AND to_kind<>0 LOOP
  IF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=l.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=l.item_instance_id) THEN CONTINUE; END IF;
  SELECT * INTO life FROM game_house_furniture_lifetimes
   WHERE 'oteryn:item.tibia.i'||item_id::text=l.definition_production_key;
  IF life.duration_ms IS NULL OR life.target_id IS NULL THEN
   RAISE EXCEPTION 'qualified rotating furniture lost duration source' USING ERRCODE='23514'; END IF;
  INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,
   duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,
   target_family,target_production_key,target_revision,content_digest)
  VALUES(l.item_instance_id,r.transaction_id,l.line_ordinal,life.duration_ms,started+life.duration_ms,
   l.definition_family,l.definition_production_key,l.definition_revision_ref,'Item',
   'oteryn:item.tibia.i'||life.target_id::text,l.definition_revision_ref,r.content_digest);
 END LOOP;
END $$;
DO $migration$ DECLARE body TEXT; arm TEXT; begin_at INTEGER; BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item temporal source proof'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_item_loop_move_receipts srv7_furniture_receipt JOIN game_item_loop_move_lines l USING(transaction_id)
  JOIN game_item_instances i USING(item_instance_id) JOIN game_house_furniture_lifetimes life
   ON 'oteryn:item.tibia.i'||life.item_id::text=l.definition_production_key
  WHERE srv7_furniture_receipt.transaction_id=NEW.source_transaction_id AND l.line_ordinal=NEW.source_ordinal
   AND l.item_instance_id=NEW.item_instance_id AND l.created_xact_id=pg_current_xact_id()
   AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
   AND i.lifecycle=1 AND i.quantity=1 AND l.quantity_before=1 AND l.quantity_after=1
   AND l.from_kind<>0 AND l.to_kind<>0
   AND life.item_id IN(9034,9039,35949,35951,35969,35971)
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
    (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
   AND (l.definition_family,l.definition_production_key,l.definition_revision_ref)=
    (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
   AND NEW.duration_millis=life.duration_ms
   AND NEW.expires_at_unix_ms=floor(extract(epoch FROM srv7_furniture_receipt.created_at)*1000)::bigint+life.duration_ms
   AND NEW.target_family='Item' AND NEW.target_production_key='oteryn:item.tibia.i'||life.target_id::text
   AND NEW.target_revision=NEW.definition_revision AND NEW.content_digest=srv7_furniture_receipt.content_digest
   AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id))
 THEN RETURN NULL; END IF;
 $arm$;
 EXECUTE substring(body FROM 1 FOR begin_at+4)||E'\n'||arm||substring(body FROM begin_at+5);
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_movement_start(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_movement_start(uuid) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_movement_start(uuid) TO oteryn_game_runtime;
