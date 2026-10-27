-- GEM3 paid actions over the immutable GEM2 / 0313 schema.
-- SQLx applies this transactionally; guards also allow a deliberate reapplication.
-- GEM3: closed Atelier fee causes and ordinary ItemInstance custody.
ALTER TABLE game_character_atelier_receipts DROP CONSTRAINT IF EXISTS game_character_atelier_receipts_kind_check;
ALTER TABLE game_character_atelier_receipts ADD CONSTRAINT game_character_atelier_receipts_kind_check
 CHECK(kind IN ('GEM_INIT','SET_LOCK','REVEAL','SWITCH_DOMAIN','DISMANTLE','GRADE_UP'));
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 IF NOT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='game_character_atelier_receipts'::regclass AND attname='fee_transaction_id' AND NOT attisdropped) THEN
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind IN (9,10,11))';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 -- 0298 retains historical imbuement prices; preserve its NOT VALID status.
 body:=regexp_replace(body,' NOT VALID$','');
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (charm_key IS NULL AND ((cause_kind=9 AND purchase_service_key=''oteryn:atelier.reveal'' AND fee_gold_units IN (125000,1000000,6000000)) OR (cause_kind=10 AND purchase_service_key=''oteryn:atelier.switch_domain'' AND fee_gold_units IN (125000,250000,500000)) OR (cause_kind=11 AND purchase_service_key=''oteryn:atelier.grade_up'' AND fee_gold_units IN (2000000,5000000,30000000,12000000,75000000))))) NOT VALID';
 END IF;
 -- This fee advances the Character through the Atelier chain, like the other Character owners.
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing fee Character proof'; END IF;
 IF position('NEW.cause_kind IN (9,10,11)' IN body)=0 THEN
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
 '(NEW.cause_kind IN (9,10,11) AND EXISTS(SELECT 1 FROM game_character_atelier_receipts r WHERE r.character_id=NEW.character_id AND r.occurrence=NEW.cause_occurrence_id AND r.committed_character_revision=NEW.committed_character_revision AND r.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 END IF;
END $$;

CREATE TABLE IF NOT EXISTS game_atelier_item_records (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 occurrence UUID NOT NULL,
 cause TEXT NOT NULL CHECK(cause IN ('REVEAL','DISMANTLE','GRADE_UP')),
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs)<=20),
 outputs JSONB NOT NULL CHECK(jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs)<=1),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA CHECK(octet_length(envelope_sha256)=32),
 CHECK((cause='DISMANTLE')=(envelope_sha256 IS NOT NULL)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(character_id,occurrence),
 FOREIGN KEY(character_id,occurrence) REFERENCES game_character_atelier_receipts DEFERRABLE INITIALLY DEFERRED,
 CHECK((cause='REVEAL' AND jsonb_array_length(inputs)=1 AND jsonb_array_length(outputs)=0)
    OR (cause='GRADE_UP' AND jsonb_array_length(inputs) BETWEEN 1 AND 20 AND jsonb_array_length(outputs)=0)
    OR (cause='DISMANTLE' AND jsonb_array_length(inputs)=0 AND jsonb_array_length(outputs)=1))
);
CREATE OR REPLACE FUNCTION game_atelier_item_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; line JSONB;
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_container_slots b JOIN game_item_instances bag USING(item_instance_id,world_id)
   WHERE b.character_id=NEW.character_id AND b.item_instance_id=NEW.parent_item_instance_id
     AND b.world_id=NEW.world_id AND bag.lifecycle=1) THEN
  RAISE EXCEPTION 'Atelier requires current main backpack' USING ERRCODE='23514';
 END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.inputs))<>jsonb_array_length(NEW.inputs) THEN
  RAISE EXCEPTION 'duplicate Atelier input' USING ERRCODE='23514';
 END IF;
 FOR line IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM line->'before' OR i.lifecycle<>1
    OR i.world_id<>NEW.world_id OR i.definition_family<>'Item' OR i.quantity NOT BETWEEN 1 AND 100
    OR (line->>'count')::int NOT BETWEEN 1 AND i.quantity
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
       AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.parent_item_instance_id
       AND e.world_id=NEW.world_id AND e.placement_ordinal=(line->>'ordinal')::numeric) THEN
   RAISE EXCEPTION 'Atelier requires unchanged plain carried input' USING ERRCODE='23514';
  END IF;
 END LOOP;
 FOR line IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP
  IF NOT game_character_is_uuid_v7((line->>'item')::uuid)
    OR (NOT line ? 'before' AND EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid))
    OR line->>'key' NOT IN ('oteryn:item.tibia.i46625','oteryn:item.tibia.i46626')
    OR (line->>'count')::int NOT BETWEEN 1 AND 5
    OR octet_length(line->>'revision') NOT BETWEEN 1 AND 512
    OR (NOT line ? 'before' AND (line->>'ordinal')::numeric <= (SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)) THEN
   RAISE EXCEPTION 'invalid Atelier fragment output' USING ERRCODE='23514';
  END IF;
  IF line ? 'before' THEN
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid FOR UPDATE;
   IF to_jsonb(i) IS DISTINCT FROM line->'before' OR i.lifecycle<>1 OR i.world_id<>NEW.world_id
    OR i.definition_family<>'Item' OR i.definition_production_key IS DISTINCT FROM line->>'key'
    OR i.definition_revision_ref IS DISTINCT FROM line->>'revision' OR i.quantity+(line->>'count')::int>100
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
      AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=(line->>'ordinal')::numeric) THEN
    RAISE EXCEPTION 'incompatible Atelier fragment receiver' USING ERRCODE='23514';
   END IF;
  END IF;
 END LOOP;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_atelier_item_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_atelier_item_records%ROWTYPE; i game_item_instances%ROWTYPE; line JSONB;
BEGIN
 SELECT * INTO r FROM game_atelier_item_records WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR NOT EXISTS(SELECT 1 FROM game_character_atelier_receipts a WHERE a.character_id=r.character_id
       AND a.occurrence=r.occurrence AND a.kind=r.cause AND a.created_xact_id=pg_current_xact_id())
    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>(SELECT count(*) FROM jsonb_array_elements(r.outputs) l WHERE NOT l ? 'before')+coalesce((SELECT (change_gold_units/100>0)::int+(change_gold_units%100>0)::int FROM game_item_fee_burns WHERE transaction_id=r.transaction_id),0)
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>jsonb_array_length(r.inputs)+(SELECT count(*) FROM jsonb_array_elements(r.outputs) l WHERE l ? 'before')+coalesce((SELECT line_count FROM game_item_fee_burns WHERE transaction_id=r.transaction_id),0)
    OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id
       AND event_id=r.event_id AND occurred_at=r.occurred_at AND
       ((r.cause='DISMANTLE' AND envelope_sha256=r.envelope_sha256) OR
        (r.cause IN ('REVEAL','GRADE_UP') AND EXISTS(SELECT 1 FROM game_item_fee_burns f
          WHERE f.transaction_id=r.transaction_id AND f.event_id=r.event_id AND f.envelope_sha256=game_item_audit_outbox.envelope_sha256
           AND f.created_xact_id=pg_current_xact_id())))
       AND created_xact_id=pg_current_xact_id() AND publication_state=1 AND published_at IS NULL) THEN
  RAISE EXCEPTION 'incomplete atomic Atelier custody' USING ERRCODE='23514';
 END IF;
 FOR line IN SELECT value FROM jsonb_array_elements(r.inputs) LOOP
  SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid;
  IF i.quantity<>(line->'before'->>'quantity')::bigint-(line->>'count')::bigint
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR i.state_revision<>(line->'before'->>'state_revision')::numeric+1
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
       ((line->'before')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
    OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id
       AND character_id=r.character_id AND world_id=r.world_id AND parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=(line->>'ordinal')::numeric)) THEN
   RAISE EXCEPTION 'Atelier burn mismatch' USING ERRCODE='23514';
  END IF;
 END LOOP;
 FOR line IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id,world_id)
    WHERE n.item_instance_id=(line->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
      AND n.quantity=coalesce((line->'before'->>'quantity')::bigint,0)+(line->>'count')::bigint AND n.definition_family='Item' AND n.definition_production_key=line->>'key'
      AND n.definition_revision_ref=line->>'revision' AND ((NOT line ? 'before' AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL AND n.state_revision=1)
        OR (line ? 'before' AND n.last_transaction_id=r.transaction_id AND n.state_revision=(line->'before'->>'state_revision')::numeric+1 AND (to_jsonb(n)-ARRAY['quantity','last_transaction_id','state_revision'])=((line->'before')-ARRAY['quantity','last_transaction_id','state_revision'])))
      AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
      AND e.placement_ordinal=(line->>'ordinal')::numeric AND (line ? 'before' OR e.placed_transaction_id=r.transaction_id)) THEN
   RAISE EXCEPTION 'Atelier fragment output mismatch' USING ERRCODE='23514';
  END IF;
 END LOOP;
END $$;
CREATE OR REPLACE FUNCTION game_atelier_item_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_atelier_item_complete(NEW.transaction_id); RETURN NULL; END $$;
DROP TRIGGER IF EXISTS game_atelier_item_admit ON game_atelier_item_records;
CREATE TRIGGER game_atelier_item_admit BEFORE INSERT ON game_atelier_item_records FOR EACH ROW EXECUTE FUNCTION game_atelier_item_admit();
DROP TRIGGER IF EXISTS game_atelier_item_consistent ON game_atelier_item_records;
CREATE CONSTRAINT TRIGGER game_atelier_item_consistent AFTER INSERT ON game_atelier_item_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_atelier_item_consistent();
DROP TRIGGER IF EXISTS game_atelier_item_immutable ON game_atelier_item_records;
CREATE TRIGGER game_atelier_item_immutable BEFORE UPDATE OR DELETE ON game_atelier_item_records FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
DROP TRIGGER IF EXISTS game_atelier_item_no_truncate ON game_atelier_item_records;
CREATE TRIGGER game_atelier_item_no_truncate BEFORE TRUNCATE ON game_atelier_item_records EXECUTE FUNCTION game_item_reject_truncate();

-- Extend the existing custody proof arms without altering any older mechanic.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_change_proven','game_item_container_entry_removal_proven','game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('game_atelier_item_records' IN body)>0 THEN CONTINUE; END IF;
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected custody proof body: %',n; END IF;
  IF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_atelier_item_records r, LATERAL jsonb_array_elements(r.inputs||r.outputs) l
      WHERE (l->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_atelier_item_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_atelier_item_records r, LATERAL jsonb_array_elements(r.inputs) l
      WHERE (l->>'item')::uuid=OLD.item_instance_id AND r.character_id=OLD.character_id
       AND r.world_id=OLD.world_id AND r.parent_item_instance_id=OLD.parent_item_instance_id
       AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_atelier_item_complete(r.transaction_id) FROM game_atelier_item_records r WHERE r.character_id=OLD.character_id AND r.created_xact_id=pg_current_xact_id(); RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_placement_proven' THEN
   arm:=$arm$
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_atelier_item_records r, LATERAL jsonb_array_elements(r.outputs) l
       WHERE (l->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.placed_transaction_id
        AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id AND r.parent_item_instance_id=NEW.parent_item_instance_id
        AND (l->>'ordinal')::numeric=NEW.placement_ordinal AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_atelier_item_records r, LATERAL jsonb_array_elements(r.outputs) l
      WHERE (l->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.minted_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_atelier_item_complete(NEW.minted_transaction_id); RETURN NULL;
    END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_atelier_item_admit','game_atelier_item_consistent'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_atelier_item_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_atelier_item_records FROM PUBLIC;
REVOKE ALL ON FUNCTION game_atelier_item_admit(),game_atelier_item_complete(UUID),game_atelier_item_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_atelier_item_records TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_atelier_item_complete(UUID) TO oteryn_game_runtime;

ALTER TABLE game_character_wheel_receipts DROP CONSTRAINT IF EXISTS game_wheel_vessel_vectors;
ALTER TABLE game_character_wheel_receipts
 ADD COLUMN IF NOT EXISTS created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 ADD COLUMN IF NOT EXISTS vessels_before JSONB NOT NULL DEFAULT '[null,null,null,null]',
 ADD COLUMN IF NOT EXISTS vessels_after JSONB NOT NULL DEFAULT '[null,null,null,null]',
 ADD CONSTRAINT game_wheel_vessel_vectors CHECK(jsonb_typeof(vessels_before)='array' AND jsonb_array_length(vessels_before)=4
  AND jsonb_typeof(vessels_after)='array' AND jsonb_array_length(vessels_after)=4);
DO $$ DECLARE body TEXT; widened TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_character_wheel_receipts'::regclass AND conname='game_character_wheel_receipt_kind_shape';
 widened:=replace(body,'(slots_before <> slots_after)','(slots_before <> slots_after OR vessels_before <> vessels_after)');
 IF widened=body AND position('vessels_before <> vessels_after' IN body)=0 THEN
  RAISE EXCEPTION 'missing Wheel allocation change guard';
 END IF;
 ALTER TABLE game_character_wheel_receipts DROP CONSTRAINT game_character_wheel_receipt_kind_shape;
 EXECUTE 'ALTER TABLE game_character_wheel_receipts ADD CONSTRAINT game_character_wheel_receipt_kind_shape '||widened;
END $$;
DROP TRIGGER IF EXISTS game_wheel_receipt_stamp_xact ON game_character_wheel_receipts;
CREATE TRIGGER game_wheel_receipt_stamp_xact BEFORE INSERT ON game_character_wheel_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TABLE IF NOT EXISTS game_character_wheel_vessels (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 vessels JSONB NOT NULL CHECK(jsonb_typeof(vessels)='array' AND jsonb_array_length(vessels)=4),
 last_wheel_occurrence_id UUID NOT NULL,
 FOREIGN KEY(character_id,last_wheel_occurrence_id) REFERENCES game_character_wheel_receipts
);
CREATE OR REPLACE FUNCTION game_wheel_vessels_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE current_vessels JSONB; previous JSONB; vessel JSONB; slot_index INT; gem_key NUMERIC; gem JSONB; byte JSONB;
BEGIN
 SELECT vessels INTO current_vessels FROM game_character_wheel_vessels WHERE character_id=NEW.character_id;
 IF TG_TABLE_NAME='game_character_wheel_receipts' THEN
  SELECT vessels_after INTO previous FROM game_character_wheel_receipts
   WHERE character_id=NEW.character_id AND after_wheel_revision=NEW.before_wheel_revision;
  IF NEW.vessels_before IS DISTINCT FROM coalesce(previous,'[null,null,null,null]'::jsonb)
    OR current_vessels IS DISTINCT FROM NEW.vessels_after THEN
   RAISE EXCEPTION 'Wheel vessel receipt chain mismatch' USING ERRCODE='23514';
  END IF;
  FOR slot_index IN 0..3 LOOP
   vessel:=NEW.vessels_after->slot_index;
   IF vessel<>'null'::jsonb AND vessel IS DISTINCT FROM NEW.vessels_before->slot_index THEN
    IF jsonb_typeof(vessel)<>'array' OR jsonb_array_length(vessel)<>16 THEN
     RAISE EXCEPTION 'invalid vessel identity' USING ERRCODE='23514';
    END IF;
    gem_key:=0;
    FOR byte IN SELECT value FROM jsonb_array_elements(vessel) LOOP
     IF jsonb_typeof(byte)<>'number' OR (byte#>>'{}')::numeric NOT BETWEEN 0 AND 255
       OR trunc((byte#>>'{}')::numeric)<>(byte#>>'{}')::numeric THEN
      RAISE EXCEPTION 'invalid vessel identity byte' USING ERRCODE='23514';
     END IF;
     gem_key:=gem_key*256+(byte#>>'{}')::numeric;
    END LOOP;
    SELECT state_json->'gems'->gem_key::text INTO gem FROM game_character_atelier_state WHERE character_id=NEW.character_id;
    IF gem IS NULL OR gem->>'domain' IS DISTINCT FROM (ARRAY['Green','Red','Purple','Blue'])[slot_index+1]
      OR gem->>'ruleset_revision' IS DISTINCT FROM NEW.wheel_ruleset_revision THEN
     RAISE EXCEPTION 'vessel gem must belong to this Character, domain and ruleset' USING ERRCODE='23514';
    END IF;
   END IF;
  END LOOP;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_character_wheel_receipts r WHERE r.character_id=NEW.character_id
    AND r.wheel_occurrence_id=NEW.last_wheel_occurrence_id AND r.vessels_after=NEW.vessels
    AND r.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'vessels require same-transaction Wheel receipt' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS game_wheel_vessels_consistent ON game_character_wheel_vessels;
CREATE CONSTRAINT TRIGGER game_wheel_vessels_consistent AFTER INSERT OR UPDATE ON game_character_wheel_vessels
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_wheel_vessels_guard();
DROP TRIGGER IF EXISTS game_wheel_vessels_receipt_consistent ON game_character_wheel_receipts;
CREATE CONSTRAINT TRIGGER game_wheel_vessels_receipt_consistent AFTER INSERT ON game_character_wheel_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_wheel_vessels_guard();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_wheel_vessels_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_wheel_vessels_guard() FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_wheel_vessels TO oteryn_game_runtime;

ALTER TABLE game_character_atelier_receipts
 ADD COLUMN IF NOT EXISTS fee_transaction_id UUID UNIQUE REFERENCES game_item_fee_burns(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN IF NOT EXISTS item_transaction_id UUID UNIQUE REFERENCES game_atelier_item_records(transaction_id) DEFERRABLE INITIALLY DEFERRED;
CREATE OR REPLACE FUNCTION game_atelier_bind_assets() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 SELECT transaction_id INTO NEW.fee_transaction_id FROM game_item_fee_burns
  WHERE character_id=NEW.character_id AND cause_occurrence_id=NEW.occurrence AND cause_kind IN (9,10,11) AND created_xact_id=pg_current_xact_id();
 SELECT transaction_id INTO NEW.item_transaction_id FROM game_atelier_item_records
  WHERE character_id=NEW.character_id AND occurrence=NEW.occurrence AND created_xact_id=pg_current_xact_id();
 IF (NEW.kind IN ('REVEAL','SWITCH_DOMAIN','GRADE_UP'))<>(NEW.fee_transaction_id IS NOT NULL)
   OR (NEW.kind IN ('REVEAL','DISMANTLE','GRADE_UP'))<>(NEW.item_transaction_id IS NOT NULL)
   OR (NEW.kind IN ('REVEAL','GRADE_UP') AND NEW.item_transaction_id IS DISTINCT FROM NEW.fee_transaction_id)
   OR NEW.outcome_json->'gold' IS DISTINCT FROM '0'::jsonb
   OR NEW.outcome_json->'fragments' IS DISTINCT FROM '{}'::jsonb
   OR NEW.outcome_json->'unrevealed' IS DISTINCT FROM '{}'::jsonb THEN
  RAISE EXCEPTION 'Atelier assets require real same-transaction custody' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS game_atelier_bind_assets ON game_character_atelier_receipts;
CREATE TRIGGER game_atelier_bind_assets BEFORE INSERT ON game_character_atelier_receipts FOR EACH ROW EXECUTE FUNCTION game_atelier_bind_assets();
CREATE OR REPLACE FUNCTION game_atelier_assets_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE a game_character_atelier_receipts%ROWTYPE; r game_atelier_item_records%ROWTYPE; f game_item_fee_burns%ROWTYPE;
 before_state JSONB; gem JSONB; grade JSONB; quality TEXT; key TEXT; count_needed INT; fee BIGINT;
BEGIN
 IF TG_TABLE_NAME='game_item_fee_burns' THEN
  IF NEW.cause_kind NOT IN (9,10,11) THEN RETURN NULL; END IF;
  SELECT * INTO a FROM game_character_atelier_receipts WHERE character_id=NEW.character_id AND occurrence=NEW.cause_occurrence_id;
  IF a.fee_transaction_id IS DISTINCT FROM NEW.transaction_id THEN
   RAISE EXCEPTION 'Atelier fee has no matching receipt' USING ERRCODE='23514';
  END IF;
 ELSE a:=NEW;
 END IF;
 IF a.kind IN ('GEM_INIT','SET_LOCK') THEN RETURN NULL; END IF;
 SELECT outcome_json INTO STRICT before_state FROM game_character_atelier_receipts
  WHERE character_id=a.character_id AND after_revision=a.before_revision;
 SELECT * INTO r FROM game_atelier_item_records WHERE transaction_id=a.item_transaction_id;
 SELECT * INTO f FROM game_item_fee_burns WHERE transaction_id=a.fee_transaction_id;
 IF a.kind='REVEAL' THEN
  SELECT e.value INTO STRICT gem FROM jsonb_each(a.outcome_json->'gems') e WHERE NOT before_state->'gems' ? e.key;
  quality:=gem->>'quality'; count_needed:=1;
  fee:=CASE quality WHEN 'Lesser' THEN 125000 WHEN 'Regular' THEN 1000000 WHEN 'Greater' THEN 6000000 END;
  SELECT 'oteryn:item.tibia.i'||(CASE WHEN b.vocation IN ('knight','elite_knight') THEN 44602 WHEN b.vocation IN ('paladin','royal_paladin') THEN 44605 WHEN b.vocation IN ('sorcerer','master_sorcerer') THEN 44608 WHEN b.vocation IN ('druid','elder_druid') THEN 44611 WHEN b.vocation IN ('monk','exalted_monk') THEN 49371 END
    +CASE quality WHEN 'Lesser' THEN 0 WHEN 'Regular' THEN 1 ELSE 2 END)::text INTO key FROM game_character_build_state b WHERE b.character_id=a.character_id;
 ELSIF a.kind='SWITCH_DOMAIN' THEN
  SELECT e.value INTO STRICT gem FROM jsonb_each(a.outcome_json->'gems') e WHERE e.value IS DISTINCT FROM before_state->'gems'->e.key;
  fee:=CASE gem->>'quality' WHEN 'Lesser' THEN 125000 WHEN 'Regular' THEN 250000 WHEN 'Greater' THEN 500000 END;
 ELSIF a.kind='GRADE_UP' THEN
  SELECT e.value INTO STRICT grade FROM jsonb_array_elements(a.outcome_json->'grades') e WHERE NOT (before_state->'grades' @> jsonb_build_array(e.value));
  count_needed:=CASE (grade->>1)::int WHEN 1 THEN 5 WHEN 2 THEN 15 WHEN 3 THEN 30 END;
  IF grade->0->>0='Basic' THEN
   key:='oteryn:item.tibia.i46625'; fee:=CASE (grade->>1)::int WHEN 1 THEN 2000000 WHEN 2 THEN 5000000 WHEN 3 THEN 30000000 END;
  ELSE
   key:='oteryn:item.tibia.i46626'; fee:=CASE (grade->>1)::int WHEN 1 THEN 5000000 WHEN 2 THEN 12000000 WHEN 3 THEN 75000000 END;
  END IF;
 ELSE
  SELECT e.value INTO STRICT gem FROM jsonb_each(before_state->'gems') e WHERE NOT a.outcome_json->'gems' ? e.key;
  quality:=gem->>'quality';
  key:=CASE WHEN quality='Greater' THEN 'oteryn:item.tibia.i46626' ELSE 'oteryn:item.tibia.i46625' END;
  IF r.outputs->0->>'key' IS DISTINCT FROM key OR (r.outputs->0->>'count')::int NOT BETWEEN
     (CASE WHEN quality='Regular' THEN 2 ELSE 1 END) AND (CASE WHEN quality='Regular' THEN 5 ELSE 3 END) THEN
   RAISE EXCEPTION 'dismantle fragment yield mismatch' USING ERRCODE='23514';
  END IF;
 END IF;
 IF a.kind<>'DISMANTLE' AND (f.transaction_id IS NULL OR f.created_xact_id<>pg_current_xact_id()
    OR f.character_id<>a.character_id OR f.committed_character_revision<>a.committed_character_revision
    OR f.cause_kind<>CASE a.kind WHEN 'REVEAL' THEN 9 WHEN 'SWITCH_DOMAIN' THEN 10 ELSE 11 END
    OR fee IS NULL OR f.fee_gold_units<>fee) THEN
  RAISE EXCEPTION 'Atelier fee mismatch' USING ERRCODE='23514';
 END IF;
 IF a.kind IN ('REVEAL','GRADE_UP') AND (key IS NULL OR count_needed IS NULL
    OR (SELECT sum((l->>'count')::int) FROM jsonb_array_elements(r.inputs) l) IS DISTINCT FROM count_needed::bigint
    OR EXISTS(SELECT 1 FROM jsonb_array_elements(r.inputs) l WHERE l->'before'->>'definition_production_key' IS DISTINCT FROM key)) THEN
  RAISE EXCEPTION 'Atelier material burn mismatch' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS game_atelier_assets_consistent ON game_character_atelier_receipts;
CREATE CONSTRAINT TRIGGER game_atelier_assets_consistent AFTER INSERT ON game_character_atelier_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_atelier_assets_guard();
DROP TRIGGER IF EXISTS game_atelier_fee_consistent ON game_item_fee_burns;
CREATE CONSTRAINT TRIGGER game_atelier_fee_consistent AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_atelier_assets_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_atelier_bind_assets() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_atelier_assets_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_atelier_bind_assets(),game_atelier_assets_guard() FROM PUBLIC;
