-- GAP-SV2 I04. Closed WriteOnceCause: no free text in the transform proof or audit.
ALTER TABLE game_item_text_write_receipts ADD COLUMN created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id();
CREATE TRIGGER game_item_text_write_receipts_stamp BEFORE INSERT ON game_item_text_write_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TABLE game_item_write_once_transforms (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances,
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 before_item JSONB NOT NULL,
 target_key TEXT NOT NULL CHECK(octet_length(target_key) BETWEEN 1 AND 512),
 target_revision TEXT NOT NULL CHECK(octet_length(target_revision) BETWEEN 1 AND 512),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 FOREIGN KEY(game_session_id,command_id) REFERENCES game_item_text_write_receipts
);
CREATE INDEX game_item_write_once_character ON game_item_write_once_transforms(character_id);
REVOKE ALL ON game_item_write_once_transforms FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_item_write_once_transforms TO oteryn_game_runtime;
GRANT SELECT ON game_item_write_once_transforms TO oteryn_game_control;
CREATE TRIGGER game_item_write_once_stamp BEFORE INSERT ON game_item_write_once_transforms
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_item_write_once_immutable BEFORE UPDATE OR DELETE ON game_item_write_once_transforms
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_write_once_no_truncate BEFORE TRUNCATE ON game_item_write_once_transforms
 EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_item_write_once_complete(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_write_once_transforms%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_item_write_once_transforms WHERE transaction_id=p_tx;
 SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
 IF r.created_xact_id<>pg_current_xact_id()
  OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
  OR i.state_revision IS DISTINCT FROM (r.before_item->>'state_revision')::numeric+1
  OR i.definition_production_key IS DISTINCT FROM r.target_key
  OR i.definition_revision_ref IS DISTINCT FROM r.target_revision
  OR i.world_id IS DISTINCT FROM r.world_id OR i.lifecycle<>1 OR i.quantity<>1
  OR (to_jsonb(i)-ARRAY['definition_production_key','definition_revision_ref','last_transaction_id','state_revision'])
   IS DISTINCT FROM (r.before_item-ARRAY['definition_production_key','definition_revision_ref','last_transaction_id','state_revision'])
  OR NOT EXISTS(SELECT 1 FROM game_item_text_write_receipts t WHERE t.game_session_id=r.game_session_id
   AND t.command_id=r.command_id AND t.character_id=r.character_id AND t.created_xact_id=r.created_xact_id)
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=r.event_id AND a.transaction_id=r.transaction_id
   AND a.item_instance_id=r.item_instance_id AND a.created_xact_id=r.created_xact_id
   AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.event_type_id=2)
 THEN RAISE EXCEPTION 'write-once requires atomic text, transform and audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_item_write_once_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_item_write_once_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_item_write_once_proven AFTER INSERT ON game_item_write_once_transforms
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_write_once_proven();

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected write-once guard body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_write_once_transforms r
     WHERE r.item_instance_id=OLD.item_instance_id AND r.transaction_id=NEW.last_transaction_id
      AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()
      AND NEW.definition_family='Item' AND NEW.definition_production_key=r.target_key
      AND NEW.definition_revision_ref=r.target_revision AND NEW.state_revision=OLD.state_revision
      AND (to_jsonb(NEW)-ARRAY['definition_production_key','definition_revision_ref','last_transaction_id','state_revision'])
       IS NOT DISTINCT FROM (to_jsonb(OLD)-ARRAY['definition_production_key','definition_revision_ref','last_transaction_id','state_revision']))
    THEN NEW.state_revision:=OLD.state_revision+1; RETURN NEW; END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_write_once_transforms r WHERE r.item_instance_id=NEW.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
      PERFORM game_item_write_once_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_item_write_once_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_write_once_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_write_once_complete(UUID),game_item_write_once_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_write_once_complete(UUID),game_item_write_once_proven() TO oteryn_game_runtime;

-- I05 reuses the existing stage-indexed schedules/receipts. A real custody move
-- advances the Item revision without consuming a stage; retain that exact lineage
-- in O(1) rather than walking an unbounded history of movement receipts.
CREATE TABLE game_item_decay_handoffs (
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 result_revision NUMERIC(20,0) NOT NULL CHECK(result_revision BETWEEN 2 AND 18446744073709551615),
 custody_transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(custody_transaction_id)),
 source_stage INTEGER NOT NULL,
 stage_transaction_id UUID NOT NULL REFERENCES game_spell_field_temporal_receipts(transaction_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(item_instance_id,result_revision),
 FOREIGN KEY(item_instance_id,source_stage) REFERENCES game_spell_field_temporal_receipts(item_instance_id,source_stage)
);
CREATE INDEX game_item_decay_handoff_move ON game_item_decay_handoffs(custody_transaction_id);
CREATE INDEX game_item_decay_handoff_stage ON game_item_decay_handoffs(stage_transaction_id);
REVOKE ALL ON game_item_decay_handoffs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_item_decay_handoffs TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_item_decay_handoffs_immutable BEFORE UPDATE OR DELETE ON game_item_decay_handoffs
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_decay_handoffs_no_truncate BEFORE TRUNCATE ON game_item_decay_handoffs
 EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_item_general_decay_chain(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p
  JOIN game_spell_item_receipts r ON r.transaction_id=p.source_transaction_id
  WHERE p.item_instance_id=p_item AND p.source_stage=1
   AND convert_from(r.intent,'UTF8')::jsonb->'operations'->(p.source_ordinal-1)->'general_decay'='true'::jsonb)
$$;

CREATE FUNCTION game_item_decay_handoff_capture() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE previous RECORD;
BEGIN
 -- This AFTER trigger grants no update authority. The original owner's BEFORE
 -- guard and deferred mutation/custody proofs must still succeed. Consequently
 -- loop, depot, Inbox, House and trade owners can preserve the same stage proof;
 -- a refused or unaudited owner transaction rolls back this handoff as well.
 IF NEW.last_transaction_id IS NULL OR NEW.last_transaction_id IS NOT DISTINCT FROM OLD.last_transaction_id
  OR NEW.state_revision<>OLD.state_revision+1 OR NEW.quantity<>1 OR NEW.lifecycle<>1
  OR (to_jsonb(NEW)-ARRAY['last_transaction_id','state_revision']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['last_transaction_id','state_revision']) THEN RETURN NEW; END IF;
 SELECT r.source_stage,r.transaction_id INTO previous FROM game_spell_field_temporal_receipts r
  JOIN game_spell_field_temporal_schedules p USING(item_instance_id,source_stage)
  WHERE r.item_instance_id=OLD.item_instance_id
   AND (p.target_family,p.target_production_key,p.target_revision)=
       (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)
   AND ((r.transaction_id=OLD.last_transaction_id AND r.state_revision_before+1=OLD.state_revision)
    OR EXISTS(SELECT 1 FROM game_item_decay_handoffs h WHERE h.item_instance_id=OLD.item_instance_id
     AND h.result_revision=OLD.state_revision AND h.custody_transaction_id=OLD.last_transaction_id
     AND h.source_stage=r.source_stage AND h.stage_transaction_id=r.transaction_id))
  ORDER BY r.source_stage DESC LIMIT 1;
 IF FOUND THEN
  INSERT INTO game_item_decay_handoffs(item_instance_id,result_revision,custody_transaction_id,source_stage,stage_transaction_id)
   VALUES(NEW.item_instance_id,NEW.state_revision,NEW.last_transaction_id,previous.source_stage,previous.transaction_id);
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_item_decay_handoff_capture AFTER UPDATE ON game_item_instances
 FOR EACH ROW EXECUTE FUNCTION game_item_decay_handoff_capture();

DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT; n TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
 original:='operation->>''kind''<>''mint_ground''';
 replacement:='(operation->>''kind'' NOT IN(''mint_ground'',''mint_inventory'') OR (operation->>''kind''=''mint_inventory'' AND operation->''general_decay'' IS DISTINCT FROM ''true''::jsonb))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing stage mint source proof'; END IF;
 EXECUTE replace(body,original,replacement);

 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='p.event_source_transaction_id IS NOT NULL OR p.map_source_transaction_id IS NOT NULL';
 replacement:=original||' OR (NEW.quantity_before=1 AND NEW.quantity_after=1 AND NEW.from_kind IN(1,2,3,4,5,6,7) AND NEW.to_kind IN(1,2,3,4) AND game_item_general_decay_chain(NEW.item_instance_id))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing stage custody admission arm'; END IF;
 EXECUTE replace(body,original,replacement);

 body:=pg_get_functiondef('game_spell_field_temporal_capture()'::regprocedure);
 original:='previous.transaction_id=OLD.last_transaction_id AND previous.state_revision_before+1=OLD.state_revision';
 replacement:='(('||original||') OR EXISTS(SELECT 1 FROM game_item_decay_handoffs h WHERE h.item_instance_id=OLD.item_instance_id AND h.result_revision=OLD.state_revision AND h.custody_transaction_id=OLD.last_transaction_id AND h.source_stage=previous.source_stage AND h.stage_transaction_id=previous.transaction_id))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing exact stage predecessor proof'; END IF;
 EXECUTE replace(body,original,replacement);

 -- Ordinary source-absolute chains survive takeover; only volatile spell fields
 -- enter restart retirement. Keep the SQL proof closed even for direct producers.
 body:=pg_get_functiondef('game_field_restart_retirement_proven()'::regprocedure);
 original:='AND l.operation_kind=1';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing volatile field source proof'; END IF;
 EXECUTE replace(body,original,original||' AND NOT game_item_general_decay_chain(i.item_instance_id)');
 body:=pg_get_functiondef('game_event_item_timer_ground_current(uuid,uuid,uuid,numeric,bigint)'::regprocedure);
 original:='p.event_source_transaction_id IS NOT NULL OR p.map_source_transaction_id IS NOT NULL';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing current Ground timer source proof'; END IF;
 EXECUTE replace(body,original,original||' OR game_item_general_decay_chain(p_item)');

 -- Durable session encoding: 1 Active, 2 Reconnectable, 3 Terminal. Retained
 -- in-world actors keep their timer; a terminal/offline owner cannot execute it.
 FOREACH n IN ARRAY ARRAY['game_spell_field_temporal_receipt_proven','game_spell_item_temporal_receipt_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('s.session_state=2' IN body)=0 THEN RAISE EXCEPTION 'missing Character timer session gate: %',n; END IF;
  EXECUTE replace(body,'s.session_state=2','s.session_state IN(1,2)');
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_item_general_decay_chain(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_decay_handoff_capture() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_general_decay_chain(UUID),game_item_decay_handoff_capture() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_general_decay_chain(UUID) TO oteryn_game_runtime;
