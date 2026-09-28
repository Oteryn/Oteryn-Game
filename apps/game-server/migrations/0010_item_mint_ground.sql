-- DUR-03 §§39.1-39.3 / VSL-COMBAT-01 stage C: one-item Ground MINT.
-- One live ItemInstance, its one immediate Ground location, the immutable MINT
-- cause receipt and the event type 2 durable-audit outbox commit in one
-- transaction. The receipt is keyed by the full typed loot output cause
-- (CREATURE-DEATH-OCCURRENCE-IDENTITY-V1 §4.2); there is no hash-only equality,
-- no receipt expiry and no receipt deletion path. TRANSFER stays closed: no
-- CharacterInventory location family exists here.

CREATE TABLE game_item_instances (
    item_instance_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(item_instance_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    definition_family TEXT NOT NULL CHECK (octet_length(definition_family) BETWEEN 1 AND 128),
    definition_production_key TEXT NOT NULL
        CHECK (octet_length(definition_production_key) BETWEEN 1 AND 512),
    definition_revision_ref TEXT NOT NULL
        CHECK (octet_length(definition_revision_ref) BETWEEN 1 AND 512),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 4294967295),
    lifecycle SMALLINT NOT NULL CHECK (lifecycle = 1),
    minted_transaction_id UUID NOT NULL UNIQUE
        CHECK (game_character_is_uuid_v7(minted_transaction_id)),
    UNIQUE (item_instance_id, world_id)
);

-- Exactly one immediate location: Ground is the only admitted family and the
-- item identity is its primary key. Corpse association is provenance only.
CREATE TABLE game_item_ground_locations (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    spatial_position BYTEA NOT NULL CHECK (octet_length(spatial_position) BETWEEN 1 AND 128),
    corpse_ref BYTEA NOT NULL CHECK (octet_length(corpse_ref) BETWEEN 1 AND 128),
    map_revision TEXT NOT NULL CHECK (octet_length(map_revision) BETWEEN 1 AND 512),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 512),
    native_room_placement_context BYTEA NOT NULL
        CHECK (octet_length(native_room_placement_context) BETWEEN 1 AND 128),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

-- One MINT per typed loot output cause, forever. The primary key is the full
-- cause tuple; the intent binding detects a same-cause different-intent reuse.
CREATE TABLE game_item_mint_receipts (
    death_world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(death_world_id)),
    death_channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(death_channel_id)),
    death_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (death_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    death_actor_local_id BIGINT NOT NULL CHECK (death_actor_local_id BETWEEN 0 AND 4294967295),
    death_actor_local_generation NUMERIC(20,0) NOT NULL
        CHECK (death_actor_local_generation BETWEEN 1 AND 18446744073709551615),
    loot_table_family TEXT NOT NULL CHECK (octet_length(loot_table_family) BETWEEN 1 AND 128),
    loot_table_production_key TEXT NOT NULL
        CHECK (octet_length(loot_table_production_key) BETWEEN 1 AND 512),
    loot_table_revision_ref TEXT NOT NULL
        CHECK (octet_length(loot_table_revision_ref) BETWEEN 1 AND 512),
    loot_purpose_key TEXT NOT NULL CHECK (octet_length(loot_purpose_key) BETWEEN 1 AND 512),
    draw_ordinal BIGINT NOT NULL CHECK (draw_ordinal BETWEEN 0 AND 4294967295),
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) = 33),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances (item_instance_id),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    PRIMARY KEY (death_world_id, death_channel_id, death_scope_ownership_generation,
                 death_actor_local_id, death_actor_local_generation, loot_table_family,
                 loot_table_production_key, loot_table_revision_ref, loot_purpose_key,
                 draw_ordinal)
);

-- Event type 2 (DUR03_NATIVE_ONE_ITEM_TRANSACTION): the exact frozen
-- EventEnvelope bytes, one event per logical transaction.
CREATE TABLE game_item_audit_outbox (
    event_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(event_id)),
    transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(transaction_id)),
    transaction_ordinal INTEGER NOT NULL CHECK (transaction_ordinal = 1),
    transaction_count INTEGER NOT NULL CHECK (transaction_count = 1),
    event_type_id BIGINT NOT NULL CHECK (event_type_id = 2),
    schema_revision BIGINT NOT NULL CHECK (schema_revision = 1),
    retention_profile_id TEXT NOT NULL
        CHECK (retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1'),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    -- DUR03-AUDIT-RETENTION-S: P90D ceiling from the envelope timestamp.
    expires_at BIGINT NOT NULL CHECK (expires_at = occurred_at + 7776000000),
    envelope BYTEA NOT NULL CHECK (octet_length(envelope) BETWEEN 1 AND 9216),
    envelope_sha256 BYTEA NOT NULL CHECK (envelope_sha256 = sha256(envelope)),
    publication_state SMALLINT NOT NULL CHECK (publication_state IN (1,2)),
    published_at BIGINT NULL CHECK (published_at IS NULL OR published_at >= occurred_at),
    CHECK ((publication_state = 2) = (published_at IS NOT NULL)),
    UNIQUE (transaction_id, transaction_ordinal)
);

CREATE FUNCTION game_item_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'DUR-03 item record is immutable' USING ERRCODE = '23514';
END;
$$;

CREATE FUNCTION game_item_reject_truncate() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'DUR-03 item relations cannot be truncated' USING ERRCODE = '23514';
END;
$$;

-- Audit rows are immutable except the one-way publication mark. No audit
-- expiry/deletion path exists in this migration.
CREATE FUNCTION game_item_audit_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.publication_state = 1 AND NEW.publication_state = 2
       AND (NEW.event_id, NEW.transaction_id, NEW.transaction_ordinal, NEW.transaction_count,
            NEW.event_type_id, NEW.schema_revision, NEW.retention_profile_id,
            NEW.item_instance_id, NEW.occurred_at, NEW.expires_at, NEW.envelope,
            NEW.envelope_sha256)
         IS NOT DISTINCT FROM
           (OLD.event_id, OLD.transaction_id, OLD.transaction_ordinal, OLD.transaction_count,
            OLD.event_type_id, OLD.schema_revision, OLD.retention_profile_id,
            OLD.item_instance_id, OLD.occurred_at, OLD.expires_at, OLD.envelope,
            OLD.envelope_sha256) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item audit record is immutable' USING ERRCODE = '23514';
END;
$$;

-- Commit-time atomicity: an ItemInstance exists only with its Ground location,
-- its cause receipt and its audit event from the same MINT transaction, in the
-- death's own runtime scope and ownership generation.
CREATE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND g.world_id = NEW.world_id
           AND g.channel_id = r.death_channel_id
           AND g.runtime_scope_ownership_generation = r.death_scope_ownership_generation
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256) THEN
        RAISE EXCEPTION 'item MINT must commit its Ground location, cause receipt and audit event together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_mint_consistent
    AFTER INSERT ON game_item_instances
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_mint_consistency_guard();

CREATE TRIGGER game_item_instance_immutable BEFORE UPDATE OR DELETE
    ON game_item_instances FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_ground_location_immutable BEFORE UPDATE OR DELETE
    ON game_item_ground_locations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_mint_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_item_mint_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_audit_guard BEFORE UPDATE OR DELETE
    ON game_item_audit_outbox FOR EACH ROW EXECUTE FUNCTION game_item_audit_guard();

CREATE TRIGGER game_item_instances_no_truncate BEFORE TRUNCATE ON game_item_instances
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_ground_locations_no_truncate BEFORE TRUNCATE ON game_item_ground_locations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_mint_receipts_no_truncate BEFORE TRUNCATE ON game_item_mint_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_audit_outbox_no_truncate BEFORE TRUNCATE ON game_item_audit_outbox
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_immutable() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_reject_truncate() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_audit_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON
    game_item_instances,
    game_item_ground_locations,
    game_item_mint_receipts,
    game_item_audit_outbox
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_immutable(),
    game_item_reject_truncate(),
    game_item_audit_guard(),
    game_item_mint_consistency_guard()
FROM PUBLIC;
GRANT SELECT, INSERT ON
    game_item_instances,
    game_item_ground_locations,
    game_item_mint_receipts,
    game_item_audit_outbox
TO oteryn_game_runtime;
GRANT SELECT ON
    game_item_instances,
    game_item_ground_locations,
    game_item_mint_receipts,
    game_item_audit_outbox
TO oteryn_game_control;
