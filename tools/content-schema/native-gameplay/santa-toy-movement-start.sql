-- Closed Santa toy aliases: XML default duration starts only when that Item
-- actually moves (Crystal game.cpp2418/2426), never when its containing giftbag moves.
CREATE TABLE game_santa_toy_lifetimes(item_key TEXT PRIMARY KEY,target_key TEXT NOT NULL,duration_ms BIGINT NOT NULL CHECK(duration_ms=3000));
INSERT INTO game_santa_toy_lifetimes VALUES('oteryn:item.tibia.i6566','oteryn:item.tibia.i5791',3000),('oteryn:item.tibia.i6567','oteryn:item.tibia.i6511',3000),('oteryn:item.tibia.i6568','oteryn:item.tibia.i5080',3000);
REVOKE ALL ON game_santa_toy_lifetimes FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_santa_toy_lifetimes TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_santa_toy_movement_start(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_loop_move_receipts%ROWTYPE; l game_item_loop_move_lines%ROWTYPE;
 life game_santa_toy_lifetimes%ROWTYPE; started BIGINT;
BEGIN
 SELECT * INTO r FROM game_item_loop_move_receipts WHERE transaction_id=p_tx;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'Santa toy duration starts only with original movement' USING ERRCODE='23514'; END IF;
 started:=floor(extract(epoch FROM r.created_at)*1000)::bigint;
 FOR l IN SELECT * FROM game_item_loop_move_lines WHERE transaction_id=p_tx
  AND definition_production_key IN('oteryn:item.tibia.i6566','oteryn:item.tibia.i6567','oteryn:item.tibia.i6568')
  AND quantity_before=1 AND quantity_after=1 AND from_kind IN(1,2,3) AND to_kind IN(1,2,3) LOOP
  IF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=l.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=l.item_instance_id) THEN CONTINUE; END IF;
  SELECT * INTO life FROM game_santa_toy_lifetimes
   WHERE item_key=l.definition_production_key;
  IF life.duration_ms IS NULL OR life.target_key IS NULL THEN
   RAISE EXCEPTION 'qualified Santa toy lost duration source' USING ERRCODE='23514'; END IF;
  INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,
   duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,
   target_family,target_production_key,target_revision,content_digest)
  VALUES(l.item_instance_id,r.transaction_id,l.line_ordinal,life.duration_ms,started+life.duration_ms,
   l.definition_family,l.definition_production_key,l.definition_revision_ref,'Item',
   life.target_key,l.definition_revision_ref,r.content_digest);
 END LOOP;
END $$;
DO $migration$ DECLARE body TEXT; arm TEXT; begin_at INTEGER; BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item temporal source proof'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id)
  JOIN game_item_instances i USING(item_instance_id) JOIN game_santa_toy_lifetimes life
   ON life.item_key=l.definition_production_key
  WHERE r.transaction_id=NEW.source_transaction_id AND l.line_ordinal=NEW.source_ordinal
   AND l.item_instance_id=NEW.item_instance_id AND l.created_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
   AND i.lifecycle=1 AND i.quantity=1 AND l.quantity_before=1 AND l.quantity_after=1
   AND l.from_kind IN(1,2,3) AND l.to_kind IN(1,2,3)
   AND life.item_key IN('oteryn:item.tibia.i6566','oteryn:item.tibia.i6567','oteryn:item.tibia.i6568')
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
    (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
   AND (l.definition_family,l.definition_production_key,l.definition_revision_ref)=
    (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
   AND NEW.duration_millis=life.duration_ms
   AND NEW.expires_at_unix_ms=floor(extract(epoch FROM r.created_at)*1000)::bigint+life.duration_ms
   AND NEW.target_family='Item' AND NEW.target_production_key=life.target_key
   AND NEW.target_revision=NEW.definition_revision AND NEW.content_digest=r.content_digest
   AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id))
 THEN RETURN NULL; END IF;
 $arm$;
 EXECUTE substring(body FROM 1 FOR begin_at+4)||E'\n'||arm||substring(body FROM begin_at+5);
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_santa_toy_movement_start(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_santa_toy_movement_start(uuid) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_santa_toy_movement_start(uuid) TO oteryn_game_runtime;
