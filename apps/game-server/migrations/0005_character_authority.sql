-- Producer-owned UUIDv7 allocation (48-bit Unix ms + 74 random bits).
CREATE FUNCTION game_character_uuid_v7() RETURNS UUID
LANGUAGE plpgsql VOLATILE AS $$
DECLARE
    v_bytes BYTEA := uuid_send(gen_random_uuid());
    v_ms BIGINT := floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT;
BEGIN
    v_bytes := overlay(v_bytes PLACING substring(int8send(v_ms) FROM 3 FOR 6) FROM 1 FOR 6);
    v_bytes := set_byte(v_bytes, 6, (get_byte(v_bytes, 6) & 15) | 112);
    v_bytes := set_byte(v_bytes, 8, (get_byte(v_bytes, 8) & 63) | 128);
    RETURN encode(v_bytes, 'hex')::UUID;
END; $$;

CREATE TABLE game_character_recovery_admissions (
    authority_scope_id TEXT NOT NULL CHECK (octet_length(authority_scope_id) BETWEEN 1 AND 128),
    recovery_generation NUMERIC(20,0) PRIMARY KEY CHECK (recovery_generation BETWEEN 1 AND 18446744073709551615),
    recovery_event_id UUID NOT NULL UNIQUE,
    predecessor_generation NUMERIC(20,0) NOT NULL CHECK (predecessor_generation >= 0 AND predecessor_generation < recovery_generation),
    issued_at NUMERIC(20,0) NOT NULL CHECK (issued_at BETWEEN 1 AND 18446744073709551615),
    issuer_identity TEXT NOT NULL CHECK (octet_length(issuer_identity) BETWEEN 1 AND 128),
    reconciled_at BIGINT NOT NULL CHECK (reconciled_at >= 0),
    CHECK (recovery_generation = predecessor_generation + 1),
    CHECK (get_byte(uuid_send(recovery_event_id), 6) >> 4 = 7)
);

CREATE TABLE game_character_account_guards (
    account_id UUID PRIMARY KEY CHECK (account_id <> '00000000-0000-0000-0000-000000000000'::uuid)
);

CREATE TABLE game_character_roots (
    character_id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    lifecycle SMALLINT NOT NULL CHECK (lifecycle = 1),
    character_revision NUMERIC(20,0) NOT NULL CHECK (character_revision BETWEEN 1 AND 18446744073709551615),
    profile_revision TEXT NOT NULL CHECK (octet_length(profile_revision) BETWEEN 1 AND 128),
    ruleset_revision TEXT NOT NULL CHECK (octet_length(ruleset_revision) BETWEEN 1 AND 128),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 128),
    starter_template_revision TEXT NOT NULL CHECK (octet_length(starter_template_revision) BETWEEN 1 AND 128),
    CHECK (get_byte(uuid_send(character_id), 6) >> 4 = 7),
    CHECK (get_byte(uuid_send(account_id), 6) >> 4 = 7),
    CHECK (get_byte(uuid_send(world_id), 6) >> 4 = 7)
);

CREATE TABLE game_character_audit_outbox (
    event_id UUID PRIMARY KEY,
    transaction_id UUID NOT NULL,
    transaction_ordinal INTEGER NOT NULL CHECK (transaction_ordinal = 1),
    transaction_count INTEGER NOT NULL CHECK (transaction_count = 1),
    event_type_id BIGINT NOT NULL CHECK (event_type_id = 1),
    schema_revision BIGINT NOT NULL CHECK (schema_revision = 1),
    retention_profile_id TEXT NOT NULL CHECK (retention_profile_id = 'CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1'),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    occurred_at BIGINT NOT NULL CHECK (occurred_at >= 0),
    payload BYTEA NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 8192),
    payload_sha256 BYTEA NOT NULL CHECK (octet_length(payload_sha256) = 32),
    -- CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1: P90D rolling ceiling from commit.
    expires_at BIGINT NOT NULL CHECK (expires_at = occurred_at + 7776000000),
    publication_state SMALLINT NOT NULL CHECK (publication_state IN (1,2)),
    published_at BIGINT NULL CHECK (published_at IS NULL OR published_at >= occurred_at),
    CHECK ((publication_state = 2) = (published_at IS NOT NULL)),
    CHECK (get_byte(uuid_send(event_id), 6) >> 4 = 7),
    CHECK (get_byte(uuid_send(transaction_id), 6) >> 4 = 7),
    UNIQUE (transaction_id, transaction_ordinal),
    UNIQUE (character_id, event_type_id)
);

CREATE TABLE game_character_operation_receipts (
    operation_id UUID PRIMARY KEY,
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    account_id UUID NOT NULL,
    character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    character_revision NUMERIC(20,0) NOT NULL CHECK (character_revision = 1),
    -- Stable audit identity; the event itself expires under its retention profile.
    event_id UUID NOT NULL UNIQUE,
    transaction_id UUID NOT NULL,
    CHECK (get_byte(uuid_send(operation_id), 6) >> 4 = 7)
);

CREATE FUNCTION game_character_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'Character first-slice authority history is immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_root_immutable BEFORE UPDATE OR DELETE ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_operation_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_recovery_admission_immutable BEFORE UPDATE OR DELETE ON game_character_recovery_admissions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();

-- Explicit legal holds: reason, authorizing actor, start and affected record.
-- Release returns the record to ordinary expiry; a hold never deletes or copies it.
CREATE TABLE game_character_audit_legal_holds (
    hold_id UUID PRIMARY KEY CHECK (get_byte(uuid_send(hold_id), 6) >> 4 = 7),
    event_id UUID NOT NULL,
    reason TEXT NOT NULL CHECK (octet_length(reason) BETWEEN 1 AND 512),
    authorizing_actor TEXT NOT NULL CHECK (octet_length(authorizing_actor) BETWEEN 1 AND 128),
    started_at BIGINT NOT NULL CHECK (started_at >= 0),
    released_at BIGINT NULL CHECK (released_at IS NULL OR released_at >= started_at),
    released_by TEXT NULL CHECK (released_by IS NULL OR octet_length(released_by) BETWEEN 1 AND 128),
    CHECK ((released_at IS NULL) = (released_by IS NULL))
);
CREATE UNIQUE INDEX game_character_audit_one_active_hold
    ON game_character_audit_legal_holds (event_id) WHERE released_at IS NULL;

-- Audit rows are immutable except the one-way publication mark; the only
-- deletion is ordinary expiry of an unheld record past its retention ceiling.
CREATE FUNCTION game_character_audit_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF OLD.publication_state = 1 AND NEW.publication_state = 2
           AND (NEW.event_id, NEW.transaction_id, NEW.transaction_ordinal, NEW.transaction_count,
                NEW.event_type_id, NEW.schema_revision, NEW.retention_profile_id, NEW.character_id,
                NEW.occurred_at, NEW.payload, NEW.payload_sha256, NEW.expires_at)
             IS NOT DISTINCT FROM
               (OLD.event_id, OLD.transaction_id, OLD.transaction_ordinal, OLD.transaction_count,
                OLD.event_type_id, OLD.schema_revision, OLD.retention_profile_id, OLD.character_id,
                OLD.occurred_at, OLD.payload, OLD.payload_sha256, OLD.expires_at) THEN
            RETURN NEW;
        END IF;
    ELSIF OLD.expires_at <= floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT
          AND NOT EXISTS (SELECT 1 FROM game_character_audit_legal_holds
                          WHERE event_id = OLD.event_id AND released_at IS NULL) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'Character audit record is immutable until unheld ordinary expiry' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_audit_guard BEFORE UPDATE OR DELETE ON game_character_audit_outbox
    FOR EACH ROW EXECUTE FUNCTION game_character_audit_guard();

CREATE FUNCTION game_character_audit_hold_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND OLD.released_at IS NULL AND NEW.released_at IS NOT NULL
       AND (NEW.hold_id, NEW.event_id, NEW.reason, NEW.authorizing_actor, NEW.started_at)
           IS NOT DISTINCT FROM (OLD.hold_id, OLD.event_id, OLD.reason, OLD.authorizing_actor, OLD.started_at) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character audit legal hold is append-only until its single release' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_audit_hold_guard BEFORE UPDATE OR DELETE ON game_character_audit_legal_holds
    FOR EACH ROW EXECUTE FUNCTION game_character_audit_hold_guard();

-- Row triggers do not fire for TRUNCATE; refuse it on every Character relation
-- so no statement can erase authority, receipts, unexpired audit or holds.
CREATE FUNCTION game_character_reject_truncate() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'Character authority relations cannot be truncated' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_recovery_admissions_no_truncate BEFORE TRUNCATE ON game_character_recovery_admissions
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_account_guards_no_truncate BEFORE TRUNCATE ON game_character_account_guards
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_roots_no_truncate BEFORE TRUNCATE ON game_character_roots
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_audit_outbox_no_truncate BEFORE TRUNCATE ON game_character_audit_outbox
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_operation_receipts_no_truncate BEFORE TRUNCATE ON game_character_operation_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_audit_legal_holds_no_truncate BEFORE TRUNCATE ON game_character_audit_legal_holds
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON TABLE
    game_character_recovery_admissions,
    game_character_account_guards,
    game_character_roots,
    game_character_audit_outbox,
    game_character_operation_receipts,
    game_character_audit_legal_holds
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_uuid_v7(),
    game_character_immutable(),
    game_character_audit_guard(),
    game_character_audit_hold_guard(),
    game_character_reject_truncate()
FROM PUBLIC;
