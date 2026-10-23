-- ITEM-TIMED-1: command replay, equipment custody and the atomic Aldo item-only fee.
CREATE TABLE game_item_timed_commands (
    transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(session_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
    result BYTEA NOT NULL CHECK(octet_length(result) BETWEEN 1 AND 48),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TRIGGER game_item_timed_commands_stamp BEFORE INSERT ON game_item_timed_commands
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_item_timed_commands_immutable BEFORE UPDATE OR DELETE ON game_item_timed_commands
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_timed_commands_no_truncate BEFORE TRUNCATE ON game_item_timed_commands
 EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_item_timed_commands FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_timed_commands TO oteryn_game_runtime;
GRANT SELECT ON game_item_timed_commands TO oteryn_game_control;

ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check,
 ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK(cause_kind IN (1,2,3,4));
-- Preserve the previous closed causes byte-for-byte, with one new fixed service arm.
DO $$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF body IS NULL THEN RAISE EXCEPTION 'missing purchase fee constraint'; END IF;
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
  || substr(body,7) || ') OR (cause_kind=4 AND charm_key IS NULL AND purchase_service_key=''oteryn:service.repair.aldo'' AND fee_gold_units=10000 AND line_count<=19))';
 -- Item-only fees may commit at CharacterRevision 1. Every older cause retains >=2.
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_committed_character_revision_check;
 ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_committed_character_revision_check
  CHECK(committed_character_revision BETWEEN 1 AND 18446744073709551615 AND (cause_kind=4 OR committed_character_revision>=2));
END;
$$;

-- A complete, symmetric physical-transaction proof: the fixed repair, its row, the
-- combined fee audit and command receipt. Older guard bodies are retained below.
CREATE FUNCTION game_timed_repair_complete(w game_item_timed_state_writes) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; t game_item_timed_states%ROWTYPE;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=w.item_instance_id;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=w.item_instance_id;
 IF w.cause<>8 OR w.created_xact_id<>pg_current_xact_id()
  OR w.definition_before_family<>'Item' OR w.definition_after_family<>'Item'
  OR w.definition_before_production_key<>'oteryn:item.tibia.i6530'
  OR w.definition_after_production_key<>'oteryn:item.tibia.i6529'
  OR w.charges_before IS NOT NULL OR w.remaining_ms_before IS NOT NULL
  OR w.deadline_before IS NOT NULL OR w.deadline_after IS NOT NULL OR w.lit_continuous
  OR w.full_charges IS NOT NULL OR w.charges_after IS NOT NULL
  OR w.full_remaining_ms IS DISTINCT FROM 14400000 OR w.remaining_ms_after IS DISTINCT FROM 14400000
  OR t.item_instance_id IS NULL OR t.state_revision<>w.expected_revision+1
  OR t.charges IS NOT NULL OR t.remaining_ms IS DISTINCT FROM 14400000 OR t.deadline_at IS NOT NULL
  OR i.lifecycle<>1 OR i.quantity<>1 OR i.last_transaction_id IS DISTINCT FROM w.transaction_id
  OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)
    IS DISTINCT FROM (w.definition_after_family,w.definition_after_production_key,w.definition_after_revision_ref)
  OR EXISTS(SELECT 1 FROM game_item_timed_state_writes other WHERE other.created_xact_id=pg_current_xact_id()
    AND (other.item_instance_id,other.expected_revision)<>(w.item_instance_id,w.expected_revision))
  OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
     ON b.item_instance_id=e.parent_item_instance_id AND b.character_id=e.character_id
     WHERE e.item_instance_id=w.item_instance_id AND e.character_id=w.holder_character_id)
  OR NOT EXISTS(SELECT 1 FROM game_item_fee_burns f JOIN game_item_timed_commands c USING(transaction_id)
     WHERE f.transaction_id=w.transaction_id AND f.cause_kind=4 AND f.fee_gold_units=10000
       AND f.cause_occurrence_id=w.transaction_id AND f.character_id=w.holder_character_id
       AND c.character_id=w.holder_character_id AND c.item_instance_id=w.item_instance_id
       AND f.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'repair requires its fixed transform, full row, fee, audit and command' USING ERRCODE='23514';
 END IF;
 INSERT INTO game_item_timed_definitions(definition_family,definition_production_key,definition_revision_ref,full_charges,full_remaining_ms,lit_continuous)
 VALUES(w.definition_after_family,w.definition_after_production_key,w.definition_after_revision_ref,NULL,14400000,false)
 ON CONFLICT DO NOTHING;
 IF NOT EXISTS(SELECT 1 FROM game_item_timed_definitions d WHERE
  (d.definition_family,d.definition_production_key,d.definition_revision_ref)=(w.definition_after_family,w.definition_after_production_key,w.definition_after_revision_ref)
  AND d.full_charges IS NULL AND d.full_remaining_ms=14400000 AND NOT d.lit_continuous) THEN
  RAISE EXCEPTION 'repair target disagrees with pinned definition' USING ERRCODE='23514';
 END IF;
END;
$$;

CREATE FUNCTION game_timed_repair_fee_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE w game_item_timed_state_writes%ROWTYPE;
BEGIN
 IF NEW.cause_kind=4 THEN
  SELECT * INTO w FROM game_item_timed_state_writes WHERE transaction_id=NEW.transaction_id;
  IF w.transaction_id IS NULL THEN RAISE EXCEPTION 'repair fee requires timed write' USING ERRCODE='23514'; END IF;
  PERFORM game_timed_repair_complete(w);
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_timed_repair_fee_proven AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_timed_repair_fee_proven();

DO $$
DECLARE body TEXT; name TEXT;
BEGIN
 -- Preserve the existing fee conservation, bank, inputs, outputs and audit proofs.
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO body;
 IF position('AND cr.xmin = pg_current_xact_id()::xid' IN body)=0 THEN RAISE EXCEPTION 'missing fee root proof'; END IF;
 body:=replace(body,'AND cr.xmin = pg_current_xact_id()::xid',
   'AND (cr.xmin = pg_current_xact_id()::xid OR (NEW.cause_kind=4 AND EXISTS(SELECT 1 FROM game_item_timed_state_writes w WHERE w.transaction_id=NEW.transaction_id AND w.cause=8 AND w.holder_character_id=NEW.character_id AND w.created_xact_id=pg_current_xact_id())))');
 EXECUTE body;
 -- The exact existing transform arm also admits the fixed repair, whose complete
 -- shape is proven by game_timed_repair_complete in this same physical transaction.
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard()','game_item_instance_change_proven()'] LOOP
  SELECT pg_get_functiondef(name::regprocedure) INTO body;
  IF position('w.cause IN (2, 3)' IN body)=0 THEN RAISE EXCEPTION 'missing timed transform arm in %',name; END IF;
  EXECUTE replace(body,'w.cause IN (2, 3)','w.cause IN (2, 3, 8)');
 END LOOP;
 SELECT pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure) INTO body;
 body:=replace(body,'BEGIN', E'BEGIN\n IF NEW.cause=8 THEN PERFORM game_timed_repair_complete(NEW); RETURN NULL; END IF;');
 -- A held active item can now occupy a genuine equipment slot as well as a bag entry.
 body:=replace(body,'OR EXISTS (SELECT 1 FROM game_item_container_entries e',
   'OR EXISTS (SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=NEW.item_instance_id AND s.character_id=NEW.holder_character_id) OR EXISTS (SELECT 1 FROM game_item_container_entries e');
 EXECUTE body;
 -- An expiry burn removes an equipment slot under its own immutable timed record.
 SELECT pg_get_functiondef('game_equipment_slot_proven()'::regprocedure) INTO body;
 body:=replace(body,'BEGIN', E'BEGIN\n IF TG_OP=''DELETE'' AND EXISTS(SELECT 1 FROM game_item_timed_state_writes w JOIN game_item_instances i USING(item_instance_id) WHERE w.item_instance_id=OLD.item_instance_id AND w.holder_character_id=OLD.character_id AND w.cause IN(2,3) AND w.created_xact_id=pg_current_xact_id() AND i.last_transaction_id=w.transaction_id AND i.lifecycle=2) THEN RETURN NULL; END IF;');
 EXECUTE body;
 SELECT pg_get_functiondef('game_equipment_state_proven()'::regprocedure) INTO body;
 body:=replace(body,'BEGIN', E'BEGIN\n IF EXISTS(SELECT 1 FROM game_item_timed_state_writes w WHERE w.transaction_id=NEW.last_transaction_id AND w.holder_character_id=NEW.character_id AND w.cause IN(2,3) AND w.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;');
 EXECUTE body;
END;
$$;
DO $$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_timed_repair_complete(game_item_timed_state_writes)','game_timed_repair_fee_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',name);
 END LOOP;
END;
$$;
