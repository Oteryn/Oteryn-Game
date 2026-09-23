-- Canonical UUIDv7: version 7 and RFC 4122/9562 variant (10xx). Every
-- Character identity column is checked with it, so a restored row cannot carry
-- a malformed typed identity.
CREATE FUNCTION game_character_is_uuid_v7(value UUID) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT (get_byte(uuid_send(value), 6) >> 4) = 7
       AND (get_byte(uuid_send(value), 8) & 192) = 128
$$;

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
    predecessor_digest BYTEA NULL CHECK (predecessor_digest IS NULL OR octet_length(predecessor_digest) = 32),
    issued_at NUMERIC(20,0) NOT NULL CHECK (issued_at BETWEEN 1 AND 18446744073709551615),
    issuer_identity TEXT NOT NULL CHECK (octet_length(issuer_identity) BETWEEN 1 AND 128),
    reconciled_at BIGINT NOT NULL CHECK (reconciled_at >= 0),
    CHECK (recovery_generation = predecessor_generation + 1),
    CHECK ((predecessor_generation = 0) = (predecessor_digest IS NULL)),
    CHECK (game_character_is_uuid_v7(recovery_event_id))
);

CREATE TABLE game_character_account_guards (
    account_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(account_id))
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
    CHECK (game_character_is_uuid_v7(character_id)),
    CHECK (game_character_is_uuid_v7(account_id)),
    CHECK (game_character_is_uuid_v7(world_id))
);

CREATE TABLE game_character_audit_outbox (
    event_id UUID PRIMARY KEY,
    transaction_id UUID NOT NULL,
    transaction_ordinal INTEGER NOT NULL CHECK (transaction_ordinal = 1),
    transaction_count INTEGER NOT NULL CHECK (transaction_count = 1),
    event_type_id BIGINT NOT NULL CHECK (event_type_id = 1),
    schema_revision BIGINT NOT NULL CHECK (schema_revision = 1),
    retention_profile_id TEXT NOT NULL CHECK (retention_profile_id = 'CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1'),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) CHECK (game_character_is_uuid_v7(character_id)),
    occurred_at BIGINT NOT NULL CHECK (occurred_at >= 0),
    payload BYTEA NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 8192),
    payload_sha256 BYTEA NOT NULL CHECK (octet_length(payload_sha256) = 32),
    -- CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1: P90D rolling ceiling from commit.
    expires_at BIGINT NOT NULL CHECK (expires_at = occurred_at + 7776000000),
    -- Originating bounded server build, captured atomically for the EventEnvelope.
    server_build_id TEXT NOT NULL CHECK (octet_length(server_build_id) BETWEEN 1 AND 128 AND server_build_id !~ '[^A-Za-z0-9._:/+-]'),
    publication_state SMALLINT NOT NULL CHECK (publication_state IN (1,2)),
    published_at BIGINT NULL CHECK (published_at IS NULL OR published_at >= occurred_at),
    CHECK ((publication_state = 2) = (published_at IS NOT NULL)),
    CHECK (game_character_is_uuid_v7(event_id)),
    CHECK (game_character_is_uuid_v7(transaction_id)),
    UNIQUE (transaction_id, transaction_ordinal),
    UNIQUE (character_id, event_type_id)
);

CREATE TABLE game_character_operation_receipts (
    operation_id UUID PRIMARY KEY,
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots(character_id) CHECK (game_character_is_uuid_v7(character_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    character_revision NUMERIC(20,0) NOT NULL CHECK (character_revision = 1),
    -- Stable audit identity; the event itself expires under its retention profile.
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    -- Durable originating occurrence time; the retained envelope must match it.
    occurred_at BIGINT NOT NULL CHECK (occurred_at >= 0),
    -- Durable originating build; the retained audit envelope must match it.
    server_build_id TEXT NOT NULL CHECK (octet_length(server_build_id) BETWEEN 1 AND 128 AND server_build_id !~ '[^A-Za-z0-9._:/+-]'),
    -- Durable TransactionId identity; unique beyond audit expiry.
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    -- CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1 decision bound by command_binding.
    issuer_decision_id UUID NOT NULL UNIQUE,
    intent_source_revision BIGINT NOT NULL UNIQUE CHECK (intent_source_revision > 0),
    issued_at_source BIGINT NOT NULL CHECK (issued_at_source >= 0),
    expires_at_source BIGINT NOT NULL,
    CHECK (expires_at_source > issued_at_source AND expires_at_source - issued_at_source <= 300),
    CHECK (game_character_is_uuid_v7(operation_id))
);

-- Game-owned Character interpretation (operator configuration). Append-only;
-- the current interpretation is the highest revision. Bootstrap intent
-- revisions are requested context and must equal the current one.
CREATE TABLE game_character_interpretations (
    interpretation_revision BIGINT PRIMARY KEY CHECK (interpretation_revision > 0),
    -- Same grammar as CharacterInterpretationV1, enforced for every writer.
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    starter_template_revision TEXT NOT NULL CHECK (starter_template_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    configured_at BIGINT NOT NULL CHECK (configured_at >= 0)
);

-- Retained source high-water of the single enabled intent issuer/variant scope
-- (OTERYN_PLATFORM_CHARACTER_AUTHORITY / OPERATOR_CONTROL_PLANE_BOOTSTRAP).
-- It only advances, together with the receipt of the decision it names.
CREATE TABLE game_character_bootstrap_intent_floors (
    issuer_scope SMALLINT PRIMARY KEY CHECK (issuer_scope = 1),
    source_revision BIGINT NOT NULL CHECK (source_revision > 0),
    issuer_decision_id UUID NOT NULL,
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) BETWEEN 1 AND 1024)
);

CREATE FUNCTION game_character_intent_floor_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.issuer_scope = OLD.issuer_scope
       AND NEW.source_revision > OLD.source_revision THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character bootstrap-intent source high-water only advances' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_intent_floor_guard BEFORE UPDATE OR DELETE ON game_character_bootstrap_intent_floors
    FOR EACH ROW EXECUTE FUNCTION game_character_intent_floor_guard();

CREATE FUNCTION game_character_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'Character first-slice authority history is immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_root_immutable BEFORE UPDATE OR DELETE ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_operation_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_recovery_admission_immutable BEFORE UPDATE OR DELETE ON game_character_recovery_admissions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_interpretation_immutable BEFORE UPDATE OR DELETE ON game_character_interpretations FOR EACH ROW EXECUTE FUNCTION game_character_immutable();

-- Explicit legal holds: reason, authorizing actor, start and affected record.
-- Release returns the record to ordinary expiry; a hold never deletes or copies it.
CREATE TABLE game_character_audit_legal_holds (
    hold_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(hold_id)),
    event_id UUID NOT NULL CHECK (game_character_is_uuid_v7(event_id)),
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
                NEW.occurred_at, NEW.payload, NEW.payload_sha256, NEW.expires_at, NEW.server_build_id)
             IS NOT DISTINCT FROM
               (OLD.event_id, OLD.transaction_id, OLD.transaction_ordinal, OLD.transaction_count,
                OLD.event_type_id, OLD.schema_revision, OLD.retention_profile_id, OLD.character_id,
                OLD.occurred_at, OLD.payload, OLD.payload_sha256, OLD.expires_at, OLD.server_build_id) THEN
            RETURN NEW;
        END IF;
    ELSE
        -- Serialize with hold placement at the database boundary, then check
        -- holds in a later statement that sees every committed hold.
        PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-audit-retention', 0));
        IF OLD.expires_at <= floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT
           AND NOT EXISTS (SELECT 1 FROM game_character_audit_legal_holds
                           WHERE event_id = OLD.event_id AND released_at IS NULL) THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION 'Character audit record is immutable until unheld ordinary expiry' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_audit_guard BEFORE UPDATE OR DELETE ON game_character_audit_outbox
    FOR EACH ROW EXECUTE FUNCTION game_character_audit_guard();

CREATE FUNCTION game_character_audit_hold_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    -- Every hold change takes the same retention lock as audit deletion.
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-audit-retention', 0));
    IF TG_OP = 'INSERT' THEN
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.released_at IS NULL AND NEW.released_at IS NOT NULL
       AND (NEW.hold_id, NEW.event_id, NEW.reason, NEW.authorizing_actor, NEW.started_at)
           IS NOT DISTINCT FROM (OLD.hold_id, OLD.event_id, OLD.reason, OLD.authorizing_actor, OLD.started_at) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character audit legal hold is append-only until its single release' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_audit_hold_guard BEFORE INSERT OR UPDATE OR DELETE ON game_character_audit_legal_holds
    FOR EACH ROW EXECUTE FUNCTION game_character_audit_hold_guard();

-- Operator-only procedures. They are not reachable from the Game server
-- process: EXECUTE is revoked from PUBLIC below and granted only to the
-- privileged operator role of a deployment, which needs no table privilege.
-- They run as SECURITY DEFINER with a fixed search_path (this schema, then
-- pg_temp), so the procedure is the authorization boundary. The row guards
-- above still enforce the retention and history invariants for every writer.

-- Configure the Game-owned current Character interpretation. History is
-- append-only; configuring the current value again returns its revision.
CREATE FUNCTION game_character_configure_interpretation(
    p_profile TEXT, p_ruleset TEXT, p_content TEXT, p_starter TEXT
) RETURNS BIGINT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_current game_character_interpretations%ROWTYPE;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-interpretation', 0));
    SELECT * INTO v_current FROM game_character_interpretations
        ORDER BY interpretation_revision DESC LIMIT 1;
    IF FOUND AND (v_current.profile_revision, v_current.ruleset_revision,
                  v_current.content_revision, v_current.starter_template_revision)
                 = (p_profile, p_ruleset, p_content, p_starter) THEN
        RETURN v_current.interpretation_revision;
    END IF;
    INSERT INTO game_character_interpretations(interpretation_revision, profile_revision,
        ruleset_revision, content_revision, starter_template_revision, configured_at)
    VALUES (COALESCE(v_current.interpretation_revision, 0) + 1, p_profile, p_ruleset,
        p_content, p_starter, floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT);
    RETURN COALESCE(v_current.interpretation_revision, 0) + 1;
END; $$;

-- Place an explicit legal hold on a retained audit event. An exact replay
-- (same event, reason and actor) returns the committed hold.
CREATE FUNCTION game_character_place_legal_hold(
    p_event_id UUID, p_reason TEXT, p_actor TEXT
) RETURNS UUID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_hold game_character_audit_legal_holds%ROWTYPE;
    v_hold_id UUID;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-audit-retention', 0));
    PERFORM 1 FROM game_character_audit_outbox WHERE event_id = p_event_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Character audit event is not retained' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_hold FROM game_character_audit_legal_holds
        WHERE event_id = p_event_id AND released_at IS NULL;
    IF FOUND THEN
        IF v_hold.reason = p_reason AND v_hold.authorizing_actor = p_actor THEN
            RETURN v_hold.hold_id;
        END IF;
        RAISE EXCEPTION 'Character audit event already has an active legal hold' USING ERRCODE = '23505';
    END IF;
    INSERT INTO game_character_audit_legal_holds(hold_id, event_id, reason, authorizing_actor, started_at)
    VALUES (game_character_uuid_v7(), p_event_id, p_reason, p_actor,
        floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT)
    RETURNING hold_id INTO v_hold_id;
    RETURN v_hold_id;
END; $$;

-- Release an active legal hold once; the event returns to ordinary expiry.
CREATE FUNCTION game_character_release_legal_hold(p_hold_id UUID, p_actor TEXT)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
    UPDATE game_character_audit_legal_holds
       SET released_at = greatest(started_at, floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT),
           released_by = p_actor
     WHERE hold_id = p_hold_id AND released_at IS NULL;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Character audit legal hold is not active' USING ERRCODE = '23514';
    END IF;
END; $$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_character_configure_interpretation(text, text, text, text) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_place_legal_hold(uuid, text, text) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_release_legal_hold(uuid, text) SET search_path = %I, pg_temp', current_schema());
END $$;

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
CREATE TRIGGER game_character_bootstrap_intent_floors_no_truncate BEFORE TRUNCATE ON game_character_bootstrap_intent_floors
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_interpretations_no_truncate BEFORE TRUNCATE ON game_character_interpretations
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON TABLE
    game_character_recovery_admissions,
    game_character_account_guards,
    game_character_roots,
    game_character_audit_outbox,
    game_character_operation_receipts,
    game_character_audit_legal_holds,
    game_character_bootstrap_intent_floors,
    game_character_interpretations
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_is_uuid_v7(uuid),
    game_character_uuid_v7(),
    game_character_immutable(),
    game_character_audit_guard(),
    game_character_audit_hold_guard(),
    game_character_reject_truncate(),
    game_character_intent_floor_guard(),
    game_character_configure_interpretation(text, text, text, text),
    game_character_place_legal_hold(uuid, text, text),
    game_character_release_legal_hold(uuid, text)
FROM PUBLIC;
