-- PARITY-IMBUE-FORGE-WIRE-1: the imbuing shrine's apply and clear writers (IMBUE-FORGE0 §4.1).
-- One immutable CommandRef record per accepted command proves, in the same physical
-- transaction: the consumed backpack materials, the item-only fee (cause 6) and the one
-- game_item_imbuements row written or removed. The runtime still holds no direct
-- INSERT/UPDATE/DELETE grant on game_item_imbuements: the record's own SECURITY DEFINER
-- trigger writes the row, and the row guard admits a delete only for a same-transaction clear.
CREATE TABLE game_item_imbuement_records (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 -- 1 apply, 2 clear.
 action SMALLINT NOT NULL CHECK(action IN (1,2)),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 item_ordinal NUMERIC(20,0) NOT NULL CHECK(item_ordinal>0),
 slot SMALLINT NOT NULL CHECK(slot BETWEEN 0 AND 2),
 category SMALLINT NOT NULL CHECK(category BETWEEN 0 AND 19),
 imbuement_key TEXT NOT NULL CHECK(octet_length(imbuement_key) BETWEEN 1 AND 512),
 imbuement_revision TEXT NOT NULL CHECK(octet_length(imbuement_revision) BETWEEN 1 AND 512),
 tier SMALLINT NOT NULL CHECK(tier BETWEEN 1 AND 3),
 fee_gold_units BIGINT NOT NULL,
 -- Apply: 1..3 distinct direct backpack stacks {item, ordinal, count, before}; clear: none.
 materials JSONB NOT NULL CHECK(jsonb_typeof(materials)='array'),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 CHECK((action=1 AND jsonb_array_length(materials) BETWEEN 1 AND 3
        AND fee_gold_units=(CASE tier WHEN 1 THEN 5000 WHEN 2 THEN 30000 ELSE 200000 END))
    OR (action=2 AND jsonb_array_length(materials)=0 AND fee_gold_units=15000))
);

-- The command's inputs, unchanged and plain, at commit of the record row.
CREATE FUNCTION game_item_imbuement_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; m JSONB; b game_item_imbuements%ROWTYPE;
BEGIN
 IF EXISTS(SELECT 1 FROM game_item_imbuement_records WHERE created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'one imbuement command per transaction' USING ERRCODE='23514'; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
                    ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id
                     AND e.placement_ordinal=NEW.item_ordinal)
 THEN RAISE EXCEPTION 'imbuement target must be a carried main backpack entry' USING ERRCODE='23514'; END IF;
 SELECT * INTO b FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id AND slot=NEW.slot;
 IF NEW.action=1 AND (b.item_instance_id IS NOT NULL OR EXISTS(SELECT 1 FROM game_item_imbuements
      WHERE item_instance_id=NEW.item_instance_id AND category=NEW.category))
 THEN RAISE EXCEPTION 'imbuement slot or category is occupied' USING ERRCODE='23514'; END IF;
 IF NEW.action=2 AND (b.item_instance_id IS NULL OR (b.category,b.imbuement_key,b.imbuement_revision,b.tier)
      IS DISTINCT FROM (NEW.category,NEW.imbuement_key,NEW.imbuement_revision,NEW.tier))
 THEN RAISE EXCEPTION 'imbuement clear must name the stored row' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.materials))<>jsonb_array_length(NEW.materials)
 THEN RAISE EXCEPTION 'duplicate imbuement material' USING ERRCODE='23514'; END IF;
 FOR m IN SELECT value FROM jsonb_array_elements(NEW.materials) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(m->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR i.item_instance_id=NEW.item_instance_id OR to_jsonb(i)<>m->'before'
     OR i.lifecycle<>1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
     OR (m->>'count')::bigint NOT BETWEEN 1 AND i.quantity
     OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
                     AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.parent_item_instance_id
                     AND e.placement_ordinal=(m->>'ordinal')::numeric)
  THEN RAISE EXCEPTION 'imbuement requires unchanged plain carried materials' USING ERRCODE='23514'; END IF;
 END LOOP;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_item_imbuement_records_admit BEFORE INSERT ON game_item_imbuement_records
 FOR EACH ROW EXECUTE FUNCTION game_item_imbuement_admit();

-- The one row change, made by the record itself: the runtime never writes the table directly.
CREATE FUNCTION game_item_imbuement_write() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.action=1 THEN
  INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,
    remaining_ms,applying_occurrence_id,applying_transaction_id)
  VALUES(NEW.item_instance_id,NEW.slot,NEW.category,NEW.imbuement_key,NEW.imbuement_revision,NEW.tier,
    72000000,NEW.transaction_id,NEW.transaction_id);
 ELSE
  DELETE FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id AND slot=NEW.slot;
  IF NOT FOUND THEN RAISE EXCEPTION 'imbuement clear found no row' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_item_imbuement_records_write AFTER INSERT ON game_item_imbuement_records
 FOR EACH ROW EXECUTE FUNCTION game_item_imbuement_write();

CREATE FUNCTION game_item_imbuement_complete(p_transaction UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_imbuement_records%ROWTYPE; i game_item_instances%ROWTYPE; m JSONB; lines INTEGER;
BEGIN
 SELECT * INTO r FROM game_item_imbuement_records WHERE transaction_id=p_transaction;
 SELECT f.line_count INTO lines FROM game_item_fee_burns f
  WHERE f.transaction_id=r.transaction_id AND f.cause_kind=6 AND f.cause_occurrence_id=r.transaction_id
    AND f.event_id=r.event_id AND f.character_id=r.character_id AND f.world_id=r.world_id
    AND f.fee_gold_units=r.fee_gold_units AND f.created_xact_id=pg_current_xact_id();
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR lines IS NULL
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)
       <>jsonb_array_length(r.materials)+lines
    OR (r.action=1 AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=r.item_instance_id
         AND b.slot=r.slot AND b.category=r.category AND b.imbuement_key=r.imbuement_key
         AND b.imbuement_revision=r.imbuement_revision AND b.tier=r.tier AND b.remaining_ms=72000000
         AND b.applying_transaction_id=r.transaction_id AND b.applying_occurrence_id=r.transaction_id))
    OR (r.action=2 AND EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=r.item_instance_id AND b.slot=r.slot))
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
                    ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=r.item_instance_id AND e.character_id=r.character_id
                     AND e.parent_item_instance_id=r.parent_item_instance_id)
 THEN RAISE EXCEPTION 'incomplete atomic imbuement' USING ERRCODE='23514'; END IF;
 FOR m IN SELECT value FROM jsonb_array_elements(r.materials) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(m->>'item')::uuid;
  IF i.quantity<>(m->'before'->>'quantity')::bigint-(m->>'count')::bigint
     OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
     OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
     OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
        ((m->'before')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
     OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
     OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id
          AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id
          AND placement_ordinal=(m->>'ordinal')::numeric))
  THEN RAISE EXCEPTION 'imbuement material consumption mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_item_imbuement_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_item_imbuement_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_item_imbuement_records_consistent AFTER INSERT ON game_item_imbuement_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_imbuement_consistent();
CREATE FUNCTION game_item_imbuement_fee_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind=6 THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_imbuement_records WHERE transaction_id=NEW.transaction_id) THEN
   RAISE EXCEPTION 'imbuement fee requires its imbuement record' USING ERRCODE='23514'; END IF;
  PERFORM game_item_imbuement_complete(NEW.transaction_id);
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_imbuement_fee_proven AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_imbuement_fee_proven();
CREATE TRIGGER game_item_imbuement_records_immutable BEFORE UPDATE OR DELETE ON game_item_imbuement_records
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_imbuement_records_no_truncate BEFORE TRUNCATE ON game_item_imbuement_records
 EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_item_imbuement_records FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_imbuement_records TO oteryn_game_runtime;
GRANT SELECT ON game_item_imbuement_records TO oteryn_game_control;

-- Item-only fee: 5000/30000/200000 apply by tier, 15000 clear (Canary imbuements.xml).
ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check,
 ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK(cause_kind IN (1,2,3,4,5,6));
DO $$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF body IS NULL THEN RAISE EXCEPTION 'missing purchase fee constraint'; END IF;
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
  || substr(body,7) || ') OR (cause_kind=6 AND charm_key IS NULL AND purchase_service_key=''oteryn:service.imbuement'' AND fee_gold_units IN (5000,30000,200000,15000) AND line_count<=15))';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_committed_character_revision_check;
 ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_committed_character_revision_check
  CHECK(committed_character_revision BETWEEN 1 AND 18446744073709551615 AND (cause_kind IN (5,6) OR committed_character_revision>=2));
END;
$$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 -- The item-only fee's Character proof is its same-transaction imbuement record.
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing item-only fee root proof'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
   '(NEW.cause_kind=6 AND EXISTS(SELECT 1 FROM game_item_imbuement_records r WHERE r.transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 -- Material decrements pass the generic quantity arm of game_item_instance_guard; only their
 -- change and whole-consumption entry removal need this record's proof. Older arms unchanged.
 FOREACH n IN ARRAY ARRAY['game_item_instance_change_proven','game_item_container_entry_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_imbuement_records r, LATERAL jsonb_array_elements(r.materials) m
      WHERE (m->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
        AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_imbuement_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_imbuement_records r
       CROSS JOIN LATERAL jsonb_array_elements(r.materials) m JOIN game_item_instances i
       ON i.item_instance_id=(m->>'item')::uuid WHERE i.item_instance_id=OLD.item_instance_id
       AND r.character_id=OLD.character_id AND r.parent_item_instance_id=OLD.parent_item_instance_id
       AND (m->>'ordinal')::numeric=OLD.placement_ordinal AND r.world_id=OLD.world_id
       AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
       AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 -- A clear removes its row only under its same-transaction record; every other rule is kept.
 body:=pg_get_functiondef('game_item_imbuement_row_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'unexpected imbuement row guard body'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_imbuement_records r WHERE r.action=2
       AND r.item_instance_id=OLD.item_instance_id AND r.slot=OLD.slot AND r.category=OLD.category
       AND r.imbuement_key=OLD.imbuement_key AND r.tier=OLD.tier
       AND r.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
$arm$ from begin_at for 5);
END $migration$;
DO $$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_imbuement_admit()','game_item_imbuement_write()',
   'game_item_imbuement_complete(uuid)','game_item_imbuement_consistent()','game_item_imbuement_fee_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',name);
 END LOOP;
END;
$$;
