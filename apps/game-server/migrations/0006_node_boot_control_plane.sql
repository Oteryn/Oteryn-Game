-- OPS-NODE-BOOT-01 D2/D3: least-privilege GameNode runtime and control-plane
-- group roles, exact-scope control grants, recorded Platform descriptor
-- issuances, launch-authorization revocation and the narrow audit-expiry
-- boundary. Deployment creates distinct LOGIN roles as members of these
-- groups; this migration creates no login role or credential.

DO $$
DECLARE
    v_role TEXT;
BEGIN
    FOREACH v_role IN ARRAY ARRAY['oteryn_game_runtime', 'oteryn_game_control'] LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = v_role) THEN
            -- Roles are cluster-global: a concurrent migration of another
            -- database may create the same group first.
            BEGIN
                EXECUTE format('CREATE ROLE %I NOLOGIN', v_role);
            EXCEPTION WHEN duplicate_object OR unique_violation THEN
                NULL;
            END;
        END IF;
        IF EXISTS (
            SELECT 1 FROM pg_roles WHERE rolname = v_role
              AND (rolcanlogin OR rolsuper OR rolbypassrls OR rolcreaterole OR rolcreatedb OR rolreplication)
        ) THEN
            RAISE EXCEPTION 'pre-existing Game group role % is not a plain NOLOGIN group', v_role
                USING ERRCODE = '42501';
        END IF;
    END LOOP;
END $$;

-- Exact-scope control authorization (D2). One row per control login role,
-- Channel scope and permitted operation (1 assign, 2 replace, 3 revoke).
-- Only the database owner writes it; the control role can only read it.
CREATE TABLE game_control_scope_grants (
    control_role TEXT NOT NULL CHECK (
        octet_length(control_role) BETWEEN 1 AND 63
        AND control_role !~ '[^A-Za-z0-9._:-]'
    ),
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(channel_id)),
    operation SMALLINT NOT NULL CHECK (operation IN (1, 2, 3)),
    PRIMARY KEY (control_role, world_id, channel_id, operation)
);

-- Every committed assignment decision is authorized for the authenticated
-- session role, which must also be the actor recorded in the canonical
-- command (NASG-COMMAND-BYTES: version, kind, 32-byte key, actor length, actor).
CREATE FUNCTION game_control_scope_grant_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_kind INTEGER := get_byte(NEW.command, 1);
    v_actor_len INTEGER := get_byte(NEW.command, 34);
BEGIN
    IF convert_from(substring(NEW.command FROM 36 FOR v_actor_len), 'UTF8') <> session_user
       OR NOT EXISTS (
            SELECT 1 FROM game_control_scope_grants g
            WHERE g.control_role = session_user
              AND '\x01'::BYTEA || uuid_send(g.world_id) || uuid_send(g.channel_id) = NEW.scope_key
              AND g.operation = v_kind
       ) THEN
        RAISE EXCEPTION 'runtime-scope assignment is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_control_scope_grant_guard BEFORE INSERT
    ON game_runtime_scope_assignment_receipts FOR EACH ROW EXECUTE FUNCTION game_control_scope_grant_guard();

-- Control-plane issuances of the Platform producer descriptor, keyed by
-- descriptor revision (D2). The fresh-store provenance is present exactly on
-- the issuance that authorizes S2 initialization. Rows are immutable.
CREATE TABLE game_native_source_descriptor_issuances (
    descriptor_revision NUMERIC(20, 0) PRIMARY KEY
        CHECK (descriptor_revision BETWEEN 1 AND 18446744073709551615),
    descriptor_facts BYTEA NOT NULL CHECK (octet_length(descriptor_facts) BETWEEN 1 AND 4096),
    installed_at BIGINT NOT NULL CHECK (installed_at >= 0),
    source_authority TEXT NOT NULL CHECK (
        octet_length(source_authority) BETWEEN 1 AND 128
        AND source_authority !~ '[^A-Za-z0-9._:/-]'
    ),
    bootstrap_namespace TEXT NULL CHECK (octet_length(bootstrap_namespace) BETWEEN 1 AND 256),
    bootstrap_provenance TEXT NULL CHECK (octet_length(bootstrap_provenance) BETWEEN 1 AND 256),
    initialized_at BIGINT NULL CHECK (initialized_at >= 0),
    issued_by TEXT NOT NULL,
    issued_at BIGINT NOT NULL CHECK (issued_at >= 0),
    CHECK ((bootstrap_namespace IS NULL) = (bootstrap_provenance IS NULL)
       AND (bootstrap_namespace IS NULL) = (initialized_at IS NULL))
);
CREATE TRIGGER game_native_descriptor_issuance_immutable BEFORE UPDATE OR DELETE
    ON game_native_source_descriptor_issuances FOR EACH ROW
    EXECUTE FUNCTION game_durability_native_source_immutable_history();

-- Record one issuance. An exact replay succeeds; different content for a
-- recorded revision, a revision not above the latest one, or a source
-- authority different from the store's fixed authority rejects (OTN03).
CREATE FUNCTION game_native_source_record_issuance(
    p_revision NUMERIC, p_facts BYTEA, p_installed_at BIGINT, p_source_authority TEXT,
    p_namespace TEXT, p_provenance TEXT, p_initialized_at BIGINT
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_existing game_native_source_descriptor_issuances%ROWTYPE;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:native-source-issuance', 0));
    SELECT * INTO v_existing FROM game_native_source_descriptor_issuances
        WHERE descriptor_revision = p_revision;
    IF FOUND THEN
        IF (v_existing.descriptor_facts, v_existing.installed_at, v_existing.source_authority)
               = (p_facts, p_installed_at, p_source_authority)
           AND (v_existing.bootstrap_namespace, v_existing.bootstrap_provenance, v_existing.initialized_at)
               IS NOT DISTINCT FROM (p_namespace, p_provenance, p_initialized_at) THEN
            RETURN;
        END IF;
        RAISE EXCEPTION 'descriptor issuance conflicts with the recorded revision' USING ERRCODE = 'OTN03';
    END IF;
    IF EXISTS (SELECT 1 FROM game_native_source_descriptor_issuances
               WHERE descriptor_revision > p_revision OR source_authority <> p_source_authority)
       OR EXISTS (SELECT 1 FROM game_durability_native_source_registration
                  WHERE source_authority <> p_source_authority) THEN
        RAISE EXCEPTION 'descriptor issuance is stale or changes the source authority' USING ERRCODE = 'OTN03';
    END IF;
    -- Exactly one fresh-store authorization exists for the lifetime of the
    -- store: a second one, or one after initialization, would let a runtime
    -- choose between competing initial source and trust descriptors.
    IF p_namespace IS NOT NULL
       AND (EXISTS (SELECT 1 FROM game_native_source_descriptor_issuances
                    WHERE bootstrap_namespace IS NOT NULL)
            OR EXISTS (SELECT 1 FROM game_durability_native_source_registration)) THEN
        RAISE EXCEPTION 'a fresh-store authorization is already recorded' USING ERRCODE = 'OTN03';
    END IF;
    INSERT INTO game_native_source_descriptor_issuances (descriptor_revision, descriptor_facts,
        installed_at, source_authority, bootstrap_namespace, bootstrap_provenance, initialized_at,
        issued_by, issued_at)
    VALUES (p_revision, p_facts, p_installed_at, p_source_authority, p_namespace, p_provenance,
        p_initialized_at, session_user, floor(extract(epoch FROM statement_timestamp()))::BIGINT);
END; $$;

-- Revocation of an abandoned, unconsumed launch authorization (D3 step 3).
CREATE TABLE game_node_bootstrap_authorization_revocations (
    authorization_digest BYTEA PRIMARY KEY
        REFERENCES game_node_bootstrap_authorizations (authorization_digest),
    revoked_by TEXT NOT NULL,
    revoked_at BIGINT NOT NULL CHECK (revoked_at >= 0)
);
CREATE TRIGGER game_node_authorization_revocation_immutable BEFORE UPDATE OR DELETE
    ON game_node_bootstrap_authorization_revocations FOR EACH ROW
    EXECUTE FUNCTION game_durability_native_source_immutable_history();

-- Idempotent; a consumed or unknown authorization rejects (OTN01), because a
-- registered incarnation is ended through registration revocation instead.
CREATE FUNCTION game_node_revoke_bootstrap_authorization(p_authorization BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_authorization game_node_bootstrap_authorizations%ROWTYPE;
BEGIN
    IF p_authorization IS NULL OR octet_length(p_authorization) <> 32 THEN
        RAISE EXCEPTION 'launch authorization revocation rejected' USING ERRCODE = 'OTN01';
    END IF;
    SELECT * INTO v_authorization FROM game_node_bootstrap_authorizations
        WHERE authorization_digest = sha256(p_authorization) FOR UPDATE;
    IF NOT FOUND OR v_authorization.consumed_node_id IS NOT NULL THEN
        RAISE EXCEPTION 'launch authorization revocation rejected' USING ERRCODE = 'OTN01';
    END IF;
    INSERT INTO game_node_bootstrap_authorization_revocations (authorization_digest, revoked_by, revoked_at)
        VALUES (v_authorization.authorization_digest, session_user,
                floor(extract(epoch FROM statement_timestamp()))::BIGINT)
        ON CONFLICT DO NOTHING;
END; $$;

-- Registration additionally rejects a revoked authorization. The body is
-- otherwise unchanged from 0003.
CREATE OR REPLACE FUNCTION game_node_register(p_authorization BYTEA, p_launch_binding TEXT, p_node_id UUID)
RETURNS NUMERIC
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_digest BYTEA;
    v_authorization game_node_bootstrap_authorizations%ROWTYPE;
    v_existing game_node_registrations%ROWTYPE;
    v_revision NUMERIC(20, 0);
    v_now BIGINT := floor(extract(epoch FROM statement_timestamp()))::BIGINT;
BEGIN
    IF p_authorization IS NULL OR octet_length(p_authorization) <> 32
       OR p_launch_binding IS NULL OR p_node_id IS NULL OR NOT game_node_is_uuid_v7(p_node_id) THEN
        RAISE EXCEPTION 'GameNode registration rejected' USING ERRCODE = 'OTN01';
    END IF;
    v_digest := sha256(p_authorization);
    SELECT * INTO v_authorization FROM game_node_bootstrap_authorizations
        WHERE authorization_digest = v_digest FOR UPDATE;
    IF NOT FOUND OR v_authorization.launch_binding <> p_launch_binding
       OR EXISTS (SELECT 1 FROM game_node_bootstrap_authorization_revocations
                  WHERE authorization_digest = v_digest) THEN
        RAISE EXCEPTION 'GameNode registration rejected' USING ERRCODE = 'OTN01';
    END IF;
    IF v_authorization.consumed_node_id IS NOT NULL THEN
        -- Lost-response reconciliation returns the original result only.
        IF v_authorization.consumed_node_id <> p_node_id THEN
            RAISE EXCEPTION 'GameNode registration rejected' USING ERRCODE = 'OTN01';
        END IF;
        SELECT registration_revision INTO v_revision FROM game_node_registrations
            WHERE node_id = p_node_id AND authorization_digest = v_digest;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'GameNode registration state is contradictory' USING ERRCODE = 'XX000';
        END IF;
        RETURN v_revision;
    END IF;
    PERFORM 1 FROM game_node_registrations WHERE node_id = p_node_id;
    IF FOUND THEN
        RAISE EXCEPTION 'GameNode registration rejected' USING ERRCODE = 'OTN01';
    END IF;
    SELECT registration_revision_high_water INTO v_revision
        FROM game_node_registration_writer WHERE writer_id = 1 FOR UPDATE;
    IF NOT FOUND OR v_revision >= 18446744073709551615 THEN
        RAISE EXCEPTION 'GameNode registration revision unavailable' USING ERRCODE = 'OTN01';
    END IF;
    IF NOT game_node_registration_history_valid() THEN
        RAISE EXCEPTION 'GameNode registration history contradicts its high-water' USING ERRCODE = 'XX000';
    END IF;
    v_revision := v_revision + 1;
    UPDATE game_node_registration_writer SET registration_revision_high_water = v_revision WHERE writer_id = 1;
    INSERT INTO game_node_registrations
        (node_id, registration_revision, authorization_digest, launch_binding, state, registered_at)
        VALUES (p_node_id, v_revision, v_digest, p_launch_binding, 1, v_now);
    UPDATE game_node_bootstrap_authorizations SET consumed_node_id = p_node_id, consumed_at = v_now
        WHERE authorization_digest = v_digest;
    IF v_authorization.supersedes_node_id IS NOT NULL THEN
        SELECT * INTO v_existing FROM game_node_registrations
            WHERE node_id = v_authorization.supersedes_node_id FOR UPDATE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'GameNode registration state is contradictory' USING ERRCODE = 'XX000';
        END IF;
        IF v_existing.state = 1 AND NOT game_node_end_registration(
            v_existing.node_id, v_existing.registration_revision, 3::SMALLINT, p_node_id) THEN
            RAISE EXCEPTION 'GameNode registration state is contradictory' USING ERRCODE = 'XX000';
        END IF;
    END IF;
    RETURN v_revision;
END; $$;

-- Ordinary Character audit expiry as the one runtime deletion boundary: the
-- runtime role holds no DELETE on the outbox. The audit row guard still
-- refuses any record that is unexpired or under an unreleased legal hold.
CREATE FUNCTION game_character_expire_audit(p_batch INTEGER) RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_deleted BIGINT;
BEGIN
    IF p_batch IS NULL OR p_batch < 1 OR p_batch > 64 THEN
        RAISE EXCEPTION 'Character audit expiry batch is out of bounds' USING ERRCODE = '22023';
    END IF;
    -- Holds and expiry serialize on one retention lock; the later statement
    -- then sees every committed hold (READ COMMITTED).
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-audit-retention', 0));
    WITH deleted AS (
        DELETE FROM game_character_audit_outbox WHERE event_id IN (
            SELECT a.event_id FROM game_character_audit_outbox a
            WHERE a.expires_at <= floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT
              AND NOT EXISTS (SELECT 1 FROM game_character_audit_legal_holds h
                              WHERE h.event_id = a.event_id AND h.released_at IS NULL)
            ORDER BY a.expires_at, a.event_id LIMIT p_batch FOR UPDATE)
        RETURNING 1)
    SELECT count(*) INTO v_deleted FROM deleted;
    RETURN v_deleted;
END; $$;

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_control_scope_grant_guard()',
        'game_native_source_record_issuance(numeric, bytea, bigint, text, text, text, bigint)',
        'game_node_revoke_bootstrap_authorization(bytea)',
        'game_character_expire_audit(integer)'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON TABLE
    game_control_scope_grants,
    game_native_source_descriptor_issuances,
    game_node_bootstrap_authorization_revocations
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_control_scope_grant_guard(),
    game_native_source_record_issuance(NUMERIC, BYTEA, BIGINT, TEXT, TEXT, TEXT, BIGINT),
    game_node_revoke_bootstrap_authorization(BYTEA),
    game_node_register(BYTEA, TEXT, UUID),
    game_character_expire_audit(INTEGER)
FROM PUBLIC;

-- Both roles inspect the migration ledger before any semantic pass.
GRANT SELECT ON _sqlx_migrations TO oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_node_is_uuid_v7(UUID), game_node_lock_current_registration(UUID, NUMERIC)
    TO oteryn_game_runtime, oteryn_game_control;

-- GameNode runtime: fenced admission, S2 custody and observations, readiness,
-- Character reads and bootstrap, and audit expiry. It reaches registration and
-- currentness only through the definer functions and never mutates control
-- state. A column UPDATE grant below exists only because row locks (FOR SHARE)
-- require it; the immutability and generation guards refuse any such update.
GRANT SELECT, INSERT, UPDATE, DELETE ON
    game_durability_admission_account_guards,
    game_durability_admission_character_guards,
    game_durability_admission_guard_history,
    game_durability_admission_lifecycle_receipts,
    game_durability_admission_runtime_guards,
    game_durability_admission_signing_trust_guards,
    game_durability_control_loss_continuity,
    game_durability_executor_custody,
    game_durability_fresh_admission_receipts,
    game_durability_reconnect_attempts,
    game_durability_reconnect_pending_commands,
    game_durability_reconnect_sessions,
    game_durability_recovery_grant_consumptions,
    game_durability_session_replacements,
    game_durability_session_use_ledgers,
    game_durability_session_use_memberships,
    game_durability_transport_ref_reservations
TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON
    game_durability_native_source_registration,
    game_durability_native_source_descriptor_history,
    game_durability_native_source_floors,
    game_durability_native_source_observation_history,
    game_durability_native_source_publication_slots
TO oteryn_game_runtime;
GRANT SELECT ON
    game_native_source_descriptor_issuances,
    game_node_registrations,
    game_runtime_scope_assignment_writer,
    game_runtime_scope_assignment_receipts,
    game_character_recovery_admissions,
    game_character_interpretations
TO oteryn_game_runtime;
GRANT SELECT, UPDATE (decided_at) ON game_runtime_scope_assignments TO oteryn_game_runtime;
GRANT UPDATE (writer_id) ON game_runtime_scope_assignment_writer TO oteryn_game_runtime;
GRANT UPDATE (reconciled_at) ON game_character_recovery_admissions TO oteryn_game_runtime;
GRANT SELECT, DELETE ON game_runtime_readiness_attestations TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON
    game_character_account_guards,
    game_character_roots,
    game_character_operation_receipts,
    game_character_bootstrap_intent_floors
TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_audit_outbox TO oteryn_game_runtime;
-- Character authority integrity checks see only whether an unreleased hold
-- names an event; the hold reason and actor stay unreadable.
GRANT SELECT (event_id, released_at) ON game_character_audit_legal_holds TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION
    game_node_register(BYTEA, TEXT, UUID),
    game_node_prove_current_incarnation(UUID, NUMERIC, BYTEA),
    game_node_require_current(UUID, NUMERIC, BYTEA),
    game_runtime_attest_readiness(BYTEA, UUID, NUMERIC, BYTEA),
    game_runtime_scope_assignment_history_valid(),
    game_character_is_uuid_v7(UUID),
    game_character_is_rfc_uuid(UUID),
    game_character_uuid_v7(),
    game_character_expire_audit(INTEGER)
TO oteryn_game_runtime;

-- Control plane: launch authorizations and registration revocation, exact-scope
-- assignment decisions, the Character interpretation, the fresh Character
-- recovery admission and descriptor issuances.
GRANT SELECT, INSERT ON game_node_bootstrap_authorizations TO oteryn_game_control;
GRANT SELECT ON game_node_bootstrap_authorization_revocations TO oteryn_game_control;
GRANT SELECT, UPDATE ON game_node_registration_writer, game_node_registrations TO oteryn_game_control;
GRANT SELECT, INSERT ON game_node_registration_endings TO oteryn_game_control;
GRANT SELECT ON game_control_scope_grants, game_native_source_descriptor_issuances TO oteryn_game_control;
GRANT MAINTAIN ON
    game_durability_admission_account_guards,
    game_durability_admission_character_guards,
    game_durability_admission_guard_history,
    game_durability_admission_lifecycle_receipts,
    game_durability_admission_runtime_guards,
    game_durability_admission_signing_trust_guards,
    game_durability_control_loss_continuity,
    game_durability_executor_custody,
    game_durability_fresh_admission_receipts,
    game_durability_reconnect_attempts,
    game_durability_reconnect_pending_commands,
    game_durability_reconnect_sessions,
    game_durability_recovery_grant_consumptions,
    game_durability_session_replacements,
    game_durability_session_use_ledgers,
    game_durability_session_use_memberships,
    game_durability_transport_ref_reservations
TO oteryn_game_control;
GRANT SELECT, INSERT, UPDATE ON
    game_runtime_scope_assignment_slots,
    game_runtime_scope_assignments,
    game_durability_admission_runtime_guards
TO oteryn_game_control;
GRANT SELECT, UPDATE ON game_runtime_scope_assignment_writer TO oteryn_game_control;
GRANT SELECT, INSERT ON
    game_runtime_scope_assignment_receipts,
    game_durability_admission_guard_history
TO oteryn_game_control;
GRANT SELECT ON
    game_character_account_guards,
    game_character_roots,
    game_character_operation_receipts,
    game_character_audit_outbox,
    game_character_audit_legal_holds,
    game_character_bootstrap_intent_floors,
    game_character_interpretations
TO oteryn_game_control;
GRANT SELECT, INSERT ON game_character_recovery_admissions TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION
    game_node_end_registration(UUID, NUMERIC, SMALLINT, UUID),
    game_node_registration_history_valid(),
    game_runtime_scope_assignment_history_valid(),
    game_node_revoke_bootstrap_authorization(BYTEA),
    game_native_source_record_issuance(NUMERIC, BYTEA, BIGINT, TEXT, TEXT, TEXT, BIGINT),
    game_character_configure_interpretation(TEXT, TEXT, TEXT, TEXT),
    game_character_is_uuid_v7(UUID),
    game_character_is_rfc_uuid(UUID)
TO oteryn_game_control;
