-- LOCAL CANDIDATE: explicit absolute Item lifetime on the existing Item owner.
-- Current runtime eligibility is independently fenced by the Rust writer;
-- schedules and receipts below are immutable original-source history.
CREATE TABLE game_spell_field_temporal_schedules (
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 source_stage INTEGER NOT NULL CHECK(source_stage BETWEEN 1 AND 32),
 blocks_movement BOOLEAN NOT NULL, blocks_projectile BOOLEAN NOT NULL, immovable_block_solid BOOLEAN NOT NULL,
 PRIMARY KEY(item_instance_id,source_stage),
 source_transaction_id UUID NOT NULL,
 source_ordinal INTEGER NOT NULL,
 duration_millis BIGINT NOT NULL CHECK(duration_millis BETWEEN 1 AND 4294967295),
 expires_at_unix_ms BIGINT NOT NULL CHECK(expires_at_unix_ms>0),
 definition_family TEXT NOT NULL CHECK(definition_family='Item'),
 definition_production_key TEXT NOT NULL CHECK(octet_length(definition_production_key) BETWEEN 1 AND 512),
 definition_revision TEXT NOT NULL CHECK(octet_length(definition_revision) BETWEEN 1 AND 512),
 target_family TEXT CHECK(target_family='Item'),
 target_production_key TEXT CHECK(octet_length(target_production_key) BETWEEN 1 AND 512),
 target_revision TEXT CHECK(octet_length(target_revision) BETWEEN 1 AND 512),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(num_nonnulls(target_family,target_production_key,target_revision) IN(0,3)),
 FOREIGN KEY(source_transaction_id,source_ordinal) REFERENCES game_spell_item_lines(transaction_id,ordinal)
);
CREATE TABLE game_spell_field_temporal_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 source_stage INTEGER NOT NULL CHECK(source_stage BETWEEN 1 AND 32),
 UNIQUE(item_instance_id,source_stage),
 FOREIGN KEY(item_instance_id,source_stage) REFERENCES game_spell_field_temporal_schedules(item_instance_id,source_stage),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 1 AND 18446744073709551614),
 quantity_before BIGINT NOT NULL CHECK(quantity_before=1),
 occurred_at_unix_ms BIGINT NOT NULL CHECK(occurred_at_unix_ms>0),
 custody_kind SMALLINT NOT NULL CHECK(custody_kind IN(1,2,3)),
 character_id UUID REFERENCES game_character_roots(character_id),
 game_session_id UUID,
 character_lease_generation NUMERIC(20,0),
 connection_generation NUMERIC(20,0),
 source_parent UUID,
 source_ordinal NUMERIC(20,0),
 source_slot SMALLINT,
 equipment_revision_before NUMERIC(20,0),
 source_stack_ordinal BIGINT,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(((custody_kind=1 AND character_id IS NULL AND num_nonnulls(game_session_id,character_lease_generation,connection_generation,source_parent,source_ordinal,source_slot,equipment_revision_before)=0 AND source_stack_ordinal>0)
   OR (custody_kind=2 AND character_id IS NOT NULL AND game_session_id IS NOT NULL AND character_lease_generation>0 AND connection_generation>0 AND source_parent IS NOT NULL AND source_ordinal>0 AND num_nonnulls(source_slot,equipment_revision_before,source_stack_ordinal)=0)
   OR (custody_kind=3 AND character_id IS NOT NULL AND game_session_id IS NOT NULL AND character_lease_generation>0 AND connection_generation>0 AND source_slot BETWEEN 1 AND 10 AND source_slot<>9 AND equipment_revision_before BETWEEN 1 AND 18446744073709551614 AND num_nonnulls(source_parent,source_ordinal,source_stack_ordinal)=0)) IS TRUE)
);
CREATE TABLE game_spell_field_temporal_audit (
 event_id UUID PRIMARY KEY REFERENCES game_spell_field_temporal_receipts(event_id),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_spell_field_temporal_receipts(transaction_id),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 16384),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_spell_field_temporal_before (
 transaction_id UUID PRIMARY KEY REFERENCES game_spell_field_temporal_receipts(transaction_id),
 item_instance_id UUID NOT NULL,
 state_revision_before NUMERIC(20,0) NOT NULL,
 quantity_before BIGINT NOT NULL,
 definition_family TEXT NOT NULL,
 definition_production_key TEXT NOT NULL,
 definition_revision TEXT NOT NULL,
 custody_kind SMALLINT NOT NULL,
 character_id UUID,
 source_parent UUID, source_ordinal NUMERIC(20,0), source_slot SMALLINT, source_stack_ordinal BIGINT,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_spell_field_temporal_schedule_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE intent JSONB; operation JSONB; stage JSONB; elapsed BIGINT;
BEGIN
 SELECT convert_from(r.intent,'UTF8')::jsonb INTO intent FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id)
 WHERE r.transaction_id=NEW.source_transaction_id AND l.ordinal=NEW.source_ordinal AND l.item_instance_id=NEW.item_instance_id
   AND l.operation_kind=1 AND l.quantity_after=1 AND l.expires_at_unix_ms IS NULL
   AND l.content_generation_digest=NEW.content_digest AND NEW.created_xact_id=pg_current_xact_id()
   AND r.created_xact_id=NEW.created_xact_id AND l.created_xact_id=NEW.created_xact_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'field chain requires original same-XID mint' USING ERRCODE='23514'; END IF;
 operation:=intent->'operations'->(NEW.source_ordinal-1);
 IF operation->>'kind'<>'mint_ground' OR jsonb_typeof(operation->'decay_chain')<>'array'
  OR jsonb_array_length(operation->'decay_chain') NOT BETWEEN 1 AND 32 THEN
  RAISE EXCEPTION 'field chain requires bounded complete source policy' USING ERRCODE='23514'; END IF;
 stage:=operation->'decay_chain'->(NEW.source_stage-1);
 IF stage IS DISTINCT FROM jsonb_build_object('definition',jsonb_build_object('family',NEW.definition_family,'key',NEW.definition_production_key,'revision',NEW.definition_revision),
   'duration_millis',NEW.duration_millis,'target',CASE WHEN NEW.target_family IS NULL THEN NULL ELSE jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,'revision',NEW.target_revision) END,
   'blocks_movement',NEW.blocks_movement,'blocks_projectile',NEW.blocks_projectile,'immovable_block_solid',NEW.immovable_block_solid) THEN
  RAISE EXCEPTION 'field stage differs from original source' USING ERRCODE='23514'; END IF;
 SELECT sum((value->>'duration_millis')::bigint) INTO elapsed FROM jsonb_array_elements(operation->'decay_chain') WITH ORDINALITY t(value,ordinal) WHERE ordinal<=NEW.source_stage;
 IF NEW.expires_at_unix_ms<>(SELECT occurred_at_unix_ms FROM game_spell_item_receipts WHERE transaction_id=NEW.source_transaction_id)+elapsed
  OR (NEW.source_stage=1 AND stage->'definition' IS DISTINCT FROM operation->'definition')
  OR (NEW.source_stage>1 AND stage->'definition' IS DISTINCT FROM operation->'decay_chain'->(NEW.source_stage-2)->'target')
  OR (NEW.target_family IS NOT NULL AND NEW.source_stage<jsonb_array_length(operation->'decay_chain') AND stage->'target' IS DISTINCT FROM operation->'decay_chain'->NEW.source_stage->'definition')
  OR (NEW.target_family IS NULL AND NEW.source_stage<>jsonb_array_length(operation->'decay_chain')) THEN
  RAISE EXCEPTION 'field chain discontinuity/deadline mismatch' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_field_temporal_schedule_proven AFTER INSERT ON game_spell_field_temporal_schedules
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_field_temporal_schedule_proven();

CREATE FUNCTION game_spell_field_temporal_capture() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_spell_field_temporal_receipts%ROWTYPE; p game_spell_field_temporal_schedules%ROWTYPE; matches BOOLEAN;
BEGIN
 SELECT * INTO r FROM game_spell_field_temporal_receipts WHERE transaction_id=NEW.last_transaction_id AND item_instance_id=NEW.item_instance_id AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN NEW; END IF;
 SELECT * INTO STRICT p FROM game_spell_field_temporal_schedules WHERE item_instance_id=OLD.item_instance_id AND source_stage=r.source_stage;
 IF r.source_stage>1 AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts previous WHERE previous.item_instance_id=OLD.item_instance_id AND previous.source_stage=r.source_stage-1 AND previous.transaction_id=OLD.last_transaction_id AND previous.state_revision_before+1=OLD.state_revision) THEN RAISE EXCEPTION 'field stage requires exact preceding receipt' USING ERRCODE='23514'; END IF;
 IF OLD.lifecycle<>1 OR OLD.world_id<>r.world_id OR OLD.quantity<>r.quantity_before OR OLD.state_revision<>r.state_revision_before
  OR (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)<>(p.definition_family,p.definition_production_key,p.definition_revision)
  OR r.occurred_at_unix_ms<p.expires_at_unix_ms THEN RAISE EXCEPTION 'Item temporal receipt lacks exact due predecessor' USING ERRCODE='23514'; END IF;
 IF r.custody_kind=1 THEN
  SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=OLD.item_instance_id AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND g.runtime_scope_ownership_generation=r.ownership_generation AND g.stack_ordinal=r.source_stack_ordinal) INTO matches;
 ELSIF r.custody_kind=2 THEN
  SELECT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.source_parent AND e.placement_ordinal=r.source_ordinal) INTO matches;
 ELSE
  SELECT EXISTS(SELECT 1 FROM game_character_equipment_slots e JOIN game_character_equipment_state s USING(character_id) WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.slot=r.source_slot AND s.revision=r.equipment_revision_before) INTO matches;
 END IF;
 IF NOT matches THEN RAISE EXCEPTION 'Item temporal predecessor custody differs' USING ERRCODE='23514'; END IF;
 INSERT INTO game_spell_field_temporal_before VALUES(r.transaction_id,OLD.item_instance_id,OLD.state_revision,OLD.quantity,OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref,r.custody_kind,r.character_id,r.source_parent,r.source_ordinal,r.source_slot,r.source_stack_ordinal,pg_current_xact_id());
 RETURN NEW;
END $$;
CREATE TRIGGER game_spell_field_temporal_capture BEFORE UPDATE ON game_item_instances FOR EACH ROW EXECUTE FUNCTION game_spell_field_temporal_capture();

CREATE FUNCTION game_spell_field_temporal_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_spell_field_temporal_schedules%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO STRICT p FROM game_spell_field_temporal_schedules WHERE item_instance_id=NEW.item_instance_id AND source_stage=NEW.source_stage;
 SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
 IF NEW.occurred_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint THEN
  RAISE EXCEPTION 'Item temporal occurrence cannot use a future clock' USING ERRCODE='23514'; END IF;
 IF NEW.custody_kind<>1 AND NOT EXISTS(
  SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_durability_admission_character_guards g USING(character_id)
   JOIN game_character_roots c USING(character_id)
   WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id AND s.session_state=2
     AND s.current_generation=NEW.connection_generation AND s.character_lease_generation=NEW.character_lease_generation
     AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=NEW.world_id AND s.runtime_scope_channel_id=NEW.channel_id
     AND s.scope_ownership_generation=NEW.ownership_generation AND g.eligible AND g.holder_game_session_id=s.game_session_id
     AND g.lease_generation=s.character_lease_generation AND c.world_id=NEW.world_id AND c.lifecycle=1) THEN
  RAISE EXCEPTION 'inventory Item timer requires current Character/session/scope' USING ERRCODE='23514'; END IF;
 IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.occurred_at_unix_ms<p.expires_at_unix_ms
  OR i.last_transaction_id<>NEW.transaction_id OR i.state_revision<>NEW.state_revision_before+1
  OR NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_before b WHERE b.transaction_id=NEW.transaction_id AND b.item_instance_id=NEW.item_instance_id AND b.state_revision_before=NEW.state_revision_before AND b.quantity_before=NEW.quantity_before AND b.created_xact_id=NEW.created_xact_id)
  OR NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_audit a WHERE a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id AND a.created_xact_id=NEW.created_xact_id
    AND convert_from(a.envelope,'UTF8')::jsonb=jsonb_build_object('receipt',to_jsonb(NEW),'schedule',to_jsonb(p)))
  OR (p.target_family IS NULL AND (i.lifecycle<>2 OR i.quantity<>0))
  OR (p.target_family IS NOT NULL AND (i.lifecycle<>1 OR i.quantity<>NEW.quantity_before OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)<>(p.target_family,p.target_production_key,p.target_revision))) THEN
  RAISE EXCEPTION 'Item temporal outcome requires exact source/custody/audit/successor' USING ERRCODE='23514';
 END IF;
 IF p.target_family IS NOT NULL AND NOT (
  (NEW.custody_kind=1 AND EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id
    AND g.world_id=NEW.world_id AND g.channel_id=NEW.channel_id AND g.runtime_scope_ownership_generation=NEW.ownership_generation AND g.stack_ordinal=NEW.source_stack_ordinal))
  OR (NEW.custody_kind=2 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
    AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal))
  OR (NEW.custody_kind=3 AND EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id
    AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id AND e.slot=NEW.source_slot))) THEN
  RAISE EXCEPTION 'transformed temporal Item must retain exact custody' USING ERRCODE='23514'; END IF;
 IF NEW.custody_kind=3 AND NOT EXISTS(SELECT 1 FROM game_character_equipment_state e WHERE e.character_id=NEW.character_id AND e.revision=NEW.equipment_revision_before+1 AND e.last_transaction_id=NEW.transaction_id) THEN
  RAISE EXCEPTION 'temporal equipped Item must advance actual equipment epoch' USING ERRCODE='23514';
 END IF;
 IF p.target_family IS NULL AND (EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id)) THEN
  RAISE EXCEPTION 'retired temporal Item retains custody' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_field_temporal_receipt_proven AFTER INSERT ON game_spell_field_temporal_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_field_temporal_receipt_proven();

DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_spell_field_temporal_schedules','game_spell_field_temporal_receipts','game_spell_field_temporal_audit','game_spell_field_temporal_before'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',t||'_immutable',t);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',t||'_no_truncate',t);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',t);
  EXECUTE format('GRANT SELECT ON %I TO oteryn_game_runtime,oteryn_game_control',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_spell_field_temporal_schedule_proven','game_spell_field_temporal_capture','game_spell_field_temporal_receipt_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',t,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',t);
 END LOOP;
END $$;
GRANT INSERT ON game_spell_field_temporal_schedules,game_spell_field_temporal_receipts,game_spell_field_temporal_audit TO oteryn_game_runtime;

-- Preserve latest original branches; add exact chain receipt cause only.
CREATE OR REPLACE FUNCTION game_item_instance_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
      AND (to_jsonb(NEW)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
        =(to_jsonb(OLD)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
      AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
        WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
          AND r.created_xact_id=pg_current_xact_id() AND r.state_revision_before=OLD.state_revision AND r.quantity_before=OLD.quantity
          AND r.occurred_at_unix_ms>=p.expires_at_unix_ms
          AND (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
          AND ((p.target_family IS NULL AND NEW.lifecycle=2 AND NEW.quantity=0
            AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref))
           OR (p.target_family IS NOT NULL AND NEW.lifecycle=1 AND NEW.quantity=OLD.quantity
             AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(p.target_family,p.target_production_key,p.target_revision)))) THEN
        NEW.state_revision:=OLD.state_revision+1;
        INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before) VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
        RETURN NEW;
    END IF;

    IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
      AND (to_jsonb(NEW)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
        =(to_jsonb(OLD)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
      AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
        WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
          AND r.created_xact_id=pg_current_xact_id() AND r.state_revision_before=OLD.state_revision AND r.quantity_before=OLD.quantity
          AND r.occurred_at_unix_ms>=p.expires_at_unix_ms
          AND (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
          AND ((p.target_family IS NULL AND NEW.lifecycle=2 AND NEW.quantity=0
            AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref))
           OR (p.target_family IS NOT NULL AND NEW.lifecycle=1 AND NEW.quantity=OLD.quantity
             AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(p.target_family,p.target_production_key,p.target_revision)))) THEN
        NEW.state_revision:=OLD.state_revision+1;
        INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before) VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
        RETURN NEW;
    END IF;

    NEW.state_revision := OLD.state_revision + 1;
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'quantity' - 'lifecycle' - 'last_transaction_id' - 'state_revision')
         = (to_jsonb(OLD) - 'quantity' - 'lifecycle' - 'last_transaction_id' - 'state_revision') THEN
        INSERT INTO game_item_transfer_quantity_evidence
            (item_instance_id, transaction_id, quantity_before)
        VALUES (OLD.item_instance_id, NEW.last_transaction_id, OLD.quantity);
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item record changes only through a TRANSFER'
        USING ERRCODE = '23514';
END;
$$;
CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       WHERE r.operation=1 AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.last_transaction_id AND r.state_revision_before=OLD.state_revision
         AND NEW.state_revision=OLD.state_revision+1 AND NEW.quantity=OLD.quantity
         AND NEW.lifecycle=OLD.lifecycle AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=NEW.item_instance_id AND e.transaction_id=NEW.last_transaction_id
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind<>1
         AND l.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
         WHERE r.transaction_id = NEW.last_transaction_id
           AND (r.source_item_instance_id = NEW.item_instance_id
                OR r.receiver_item_instance_id = NEW.item_instance_id)
           -- Repair generation 2 (finding 2's root cause, applied here too):
           -- the matched receipt must be this SAME physical transaction's
           -- own receipt, never a historical one whose logical TransactionId
           -- is replayed from a different, already-committed transaction.
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
         WHERE d.transaction_id = NEW.last_transaction_id
           AND d.item_instance_id = NEW.item_instance_id
           AND d.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          -- The line's fee record is this same physical transaction's own record, never a
          -- committed one a later line is appended to.
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
                                    AND f.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE OR REPLACE FUNCTION game_item_ground_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=1
        AND r.source_stack_ordinal=OLD.stack_ordinal AND r.world_id=OLD.world_id AND r.channel_id=OLD.channel_id AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=1
        AND r.source_stack_ordinal=OLD.stack_ordinal AND r.world_id=OLD.world_id AND r.channel_id=OLD.channel_id AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=OLD.item_instance_id AND e.source_stack_ordinal=OLD.stack_ordinal AND (e.world_id,e.channel_id)=(OLD.world_id,OLD.channel_id)
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
       WHERE l.item_instance_id=OLD.item_instance_id AND l.operation_kind IN(2,3)
         AND (l.world_id,l.channel_id,l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
           =(OLD.world_id,OLD.channel_id,OLD.spatial_position,OLD.map_revision,OLD.content_revision,OLD.native_room_placement_context,OLD.stack_ordinal)
         AND i.last_transaction_id=l.transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
         WHERE m.item_instance_id = OLD.item_instance_id
           AND m.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'a corpse Ground location is never removed by a TRANSFER'
            USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Ground removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND (l.operation_kind=6 OR r.character_id=OLD.character_id)
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.to_slot IS NOT NULL AND r.item_instance_id=OLD.item_instance_id
         AND r.character_id=OLD.character_id AND r.backpack_item_instance_id=OLD.parent_item_instance_id
         AND r.backpack_ordinal=OLD.placement_ordinal AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND f.character_id = OLD.character_id
           AND f.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_ground_removal_proven','game_item_container_entry_removal_proven','game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  IF to_regprocedure(f||'()') IS NOT NULL THEN EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',f,current_schema()); END IF;
 END LOOP;
END $$;
