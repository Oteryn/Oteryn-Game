-- UNINSTALLED ITEM-IMBUE-1 continuation. No migration number or runtime grant is assigned.
-- Install only after the checkpoint schema proposal in the existing isolated native harness.
-- This adds the closed retained-zero Expire shape. Apply, Clear, fees, ticking and content
-- admission remain absent. No ItemInstance, custody, CharacterRevision or existing proof arm
-- is changed. One physical transaction expires at most one slot.

CREATE TABLE game_imbuement_expiry_receipts (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    expiry_occurrence UUID NOT NULL CHECK (game_character_is_uuid_v7(expiry_occurrence)),
    character_id UUID NOT NULL REFERENCES game_character_roots,
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation BETWEEN 1 AND 18446744073709551615),
    character_revision NUMERIC(20,0) NOT NULL CHECK (character_revision BETWEEN 1 AND 18446744073709551615),
    equipment_revision NUMERIC(20,0) NOT NULL CHECK (equipment_revision BETWEEN 1 AND 18446744073709551615),
    equipment_slot SMALLINT NOT NULL CHECK (equipment_slot IN (1,2,3,4,5,6,7,8,10)),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances,
    slot SMALLINT NOT NULL CHECK (slot BETWEEN 0 AND 2),
    applying_occurrence UUID NOT NULL CHECK (game_character_is_uuid_v7(applying_occurrence)),
    -- Captured by INSERT ... SELECT from locked native rows, then proven against the
    -- database-owned deletion evidence. JSON here is full stored evidence, not authority.
    slot_before JSONB NOT NULL CHECK (jsonb_typeof(slot_before) = 'object'),
    item_before JSONB NOT NULL CHECK (jsonb_typeof(item_before) = 'object'),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    occurred_at BIGINT NOT NULL CHECK (occurred_at BETWEEN 1 AND 9223372029078775807),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    schema_revision BIGINT NOT NULL,
    retention_profile_id TEXT NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, expiry_occurrence),
    UNIQUE (item_instance_id, slot, applying_occurrence),
    UNIQUE (created_xact_id),
    CHECK (event_id <> transaction_id),
    CHECK ((schema_revision = 1 AND retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1')
        OR (schema_revision = 2 AND retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2'))
);

-- Only the SECURITY DEFINER deletion trigger can insert this witness. A caller cannot
-- manufacture a successful deletion by supplying a receipt for an absent/different row.
CREATE TABLE game_imbuement_expiry_evidence (
    transaction_id UUID PRIMARY KEY REFERENCES game_imbuement_expiry_receipts,
    slot_before JSONB NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TRIGGER game_imbuement_expiry_receipts_stamp_xact BEFORE INSERT
    ON game_imbuement_expiry_receipts FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_imbuement_expiry_evidence_stamp_xact BEFORE INSERT
    ON game_imbuement_expiry_evidence FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_imbuement_expiry_receipts_immutable BEFORE UPDATE OR DELETE
    ON game_imbuement_expiry_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_imbuement_expiry_evidence_immutable BEFORE UPDATE OR DELETE
    ON game_imbuement_expiry_evidence FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_imbuement_expiry_receipts_no_truncate BEFORE TRUNCATE
    ON game_imbuement_expiry_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_imbuement_expiry_evidence_no_truncate BEFORE TRUNCATE
    ON game_imbuement_expiry_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Keep the checkpoint UPDATE predicate unchanged. Only the new narrow zero-row DELETE
-- branch is added; an old receipt, mismatched binding, nonzero row or bare DELETE rejects.
CREATE OR REPLACE FUNCTION game_imbuement_checkpoint_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE receipt game_imbuement_expiry_receipts%ROWTYPE;
BEGIN
    IF TG_OP = 'DELETE' THEN
        SELECT * INTO receipt FROM game_imbuement_expiry_receipts r
         WHERE r.item_instance_id = OLD.item_instance_id AND r.slot = OLD.slot
           AND r.applying_occurrence = OLD.applying_occurrence
           AND r.created_xact_id = pg_current_xact_id();
        IF receipt.transaction_id IS NULL OR OLD.remaining_ms <> 0
           OR receipt.slot_before IS DISTINCT FROM to_jsonb(OLD)
           OR octet_length(OLD.imbuement_key) NOT BETWEEN 1 AND 512
           OR octet_length(OLD.content_revision) NOT BETWEEN 1 AND 512
           OR octet_length(OLD.category) NOT BETWEEN 1 AND 512 THEN
            RAISE EXCEPTION 'expiry requires one exact retained zero row and its current receipt'
                USING ERRCODE = '23514';
        END IF;
        INSERT INTO game_imbuement_expiry_evidence(transaction_id, slot_before)
            VALUES (receipt.transaction_id, to_jsonb(OLD));
        RETURN OLD;
    END IF;
    IF TG_OP <> 'UPDATE' THEN
        RAISE EXCEPTION 'checkpoint cannot delete retained imbuement rows' USING ERRCODE = '23514';
    END IF;
    IF (to_jsonb(NEW)-'remaining_ms')<>(to_jsonb(OLD)-'remaining_ms') OR NEW.remaining_ms>OLD.remaining_ms THEN
        RAISE EXCEPTION 'checkpoint preserves binding and only decreases remaining' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION game_assert_imbuement_expiry(p_transaction UUID) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE r game_imbuement_expiry_receipts%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_imbuement_expiry_receipts WHERE transaction_id = p_transaction;
    IF r.transaction_id IS NULL OR r.created_xact_id <> pg_current_xact_id()
       OR NOT EXISTS (SELECT 1 FROM game_imbuement_expiry_evidence e
                       WHERE e.transaction_id = r.transaction_id
                         AND e.created_xact_id = pg_current_xact_id()
                         AND e.slot_before = r.slot_before)
       OR EXISTS (SELECT 1 FROM game_item_imbuements s
                   WHERE s.item_instance_id = r.item_instance_id AND s.slot = r.slot)
       OR NOT EXISTS (SELECT 1 FROM game_item_instances i
                       WHERE i.item_instance_id = r.item_instance_id AND i.lifecycle = 1
                         AND i.world_id = r.world_id AND to_jsonb(i) = r.item_before)
       OR NOT EXISTS (SELECT 1 FROM game_character_roots c
                       WHERE c.character_id = r.character_id AND c.world_id = r.world_id
                         AND c.character_revision = r.character_revision)
       OR NOT EXISTS (SELECT 1 FROM game_character_equipment_state e
                       WHERE e.character_id = r.character_id AND e.revision = r.equipment_revision)
       OR NOT EXISTS (SELECT 1 FROM game_character_equipment_slots e
                       WHERE e.character_id = r.character_id AND e.slot = r.equipment_slot
                         AND e.item_instance_id = r.item_instance_id AND e.world_id = r.world_id)
       OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a
                       WHERE a.transaction_id = r.transaction_id AND a.event_id = r.event_id
                         AND a.item_instance_id = r.item_instance_id AND a.event_type_id = 2
                         AND a.schema_revision = r.schema_revision
                         AND a.retention_profile_id = r.retention_profile_id
                         AND a.occurred_at = r.occurred_at
                         AND a.envelope_sha256 = r.envelope_sha256
                         AND a.created_xact_id = pg_current_xact_id()
                         AND a.publication_state = 1 AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'expiry must atomically remove its zero slot with evidence and one audit while preserving item, equipment and CharacterRevision'
            USING ERRCODE = '23514';
    END IF;
END $$;
CREATE FUNCTION game_imbuement_expiry_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM game_assert_imbuement_expiry(NEW.transaction_id);
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_imbuement_expiry_consistent
    AFTER INSERT ON game_imbuement_expiry_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_imbuement_expiry_consistency_guard();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_imbuement_checkpoint_guard() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_assert_imbuement_expiry(UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_imbuement_expiry_consistency_guard() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON game_imbuement_expiry_receipts, game_imbuement_expiry_evidence FROM PUBLIC;
REVOKE ALL ON FUNCTION game_imbuement_checkpoint_guard(), game_assert_imbuement_expiry(UUID),
    game_imbuement_expiry_consistency_guard() FROM PUBLIC;
-- Deliberately no grants here. Test-only privileges are assigned by the native harness.
