-- GameNode process-incarnation registration (GAME-NODE-REGISTRATION-BOOTSTRAP-AUTH-V1)
-- and the Channel-only runtime-scope assignment authority (OPS-SCOPE-ASSIGNMENT-FENCING-V1).
-- No roles are created here. Every new relation and function is closed to PUBLIC;
-- deployment provisioning grants the dedicated assignment-writer, control and
-- GameNode runtime roles separately. Ordinary runtime roles receive no table
-- mutation privilege and reach registration only through the definer functions.

CREATE FUNCTION game_node_is_uuid_v7(value UUID) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT value IS NOT NULL
        AND (get_byte(uuid_send(value), 6) >> 4) = 7
        AND (get_byte(uuid_send(value), 8) & 192) = 128
$$;

-- Writer-owned monotonic registration revisions. Never reset or deleted.
CREATE TABLE game_node_registration_writer (
    writer_id SMALLINT PRIMARY KEY CHECK (writer_id = 1),
    registration_revision_high_water NUMERIC(20, 0) NOT NULL
        CHECK (registration_revision_high_water BETWEEN 0 AND 18446744073709551615)
);
INSERT INTO game_node_registration_writer (writer_id, registration_revision_high_water) VALUES (1, 0);

-- One-launch bootstrap authorizations. Only the SHA-256 digest of the launch
-- secret is retained; a consumed authorization is never reusable.
CREATE TABLE game_node_bootstrap_authorizations (
    authorization_digest BYTEA PRIMARY KEY CHECK (octet_length(authorization_digest) = 32),
    launch_binding TEXT NOT NULL UNIQUE CHECK (
        octet_length(launch_binding) BETWEEN 1 AND 128
        AND launch_binding !~ '[^A-Za-z0-9._:-]'
    ),
    supersedes_node_id UUID NULL CHECK (supersedes_node_id IS NULL OR game_node_is_uuid_v7(supersedes_node_id)),
    issued_at BIGINT NOT NULL CHECK (issued_at >= 0),
    consumed_node_id UUID NULL UNIQUE,
    consumed_at BIGINT NULL CHECK (consumed_at IS NULL OR consumed_at >= 0),
    CHECK ((consumed_node_id IS NULL) = (consumed_at IS NULL)),
    CHECK (consumed_node_id IS NULL OR supersedes_node_id IS NULL OR consumed_node_id <> supersedes_node_id)
);

-- state: 1 CURRENT, 2 REVOKED, 3 SUPERSEDED. A NodeId names one process incarnation.
CREATE TABLE game_node_registrations (
    node_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(node_id)),
    registration_revision NUMERIC(20, 0) NOT NULL UNIQUE
        CHECK (registration_revision BETWEEN 1 AND 18446744073709551615),
    authorization_digest BYTEA NOT NULL UNIQUE
        REFERENCES game_node_bootstrap_authorizations (authorization_digest),
    launch_binding TEXT NOT NULL,
    state SMALLINT NOT NULL CHECK (state IN (1, 2, 3)),
    registered_at BIGINT NOT NULL CHECK (registered_at >= 0),
    ended_at BIGINT NULL CHECK (ended_at IS NULL OR ended_at >= registered_at),
    superseded_by UUID NULL REFERENCES game_node_registrations (node_id),
    CHECK ((state = 1) = (ended_at IS NULL)),
    CHECK ((state = 3) = (superseded_by IS NOT NULL))
);
ALTER TABLE game_node_bootstrap_authorizations
    ADD CONSTRAINT game_node_authorization_supersedes
        FOREIGN KEY (supersedes_node_id) REFERENCES game_node_registrations (node_id),
    ADD CONSTRAINT game_node_authorization_consumed
        FOREIGN KEY (consumed_node_id) REFERENCES game_node_registrations (node_id);

CREATE FUNCTION game_node_registration_writer_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.writer_id <> OLD.writer_id
       OR NEW.registration_revision_high_water <= OLD.registration_revision_high_water THEN
        RAISE EXCEPTION 'GameNode registration high-water cannot roll back' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_node_registration_writer_guard BEFORE UPDATE OR DELETE
    ON game_node_registration_writer FOR EACH ROW EXECUTE FUNCTION game_node_registration_writer_guard();

CREATE FUNCTION game_node_bootstrap_authorization_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.authorization_digest <> OLD.authorization_digest
       OR NEW.launch_binding <> OLD.launch_binding
       OR NEW.supersedes_node_id IS DISTINCT FROM OLD.supersedes_node_id
       OR NEW.issued_at <> OLD.issued_at
       OR OLD.consumed_node_id IS NOT NULL
       OR NEW.consumed_node_id IS NULL THEN
        RAISE EXCEPTION 'GameNode bootstrap authorization is single-use and immutable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_node_bootstrap_authorization_guard BEFORE UPDATE OR DELETE
    ON game_node_bootstrap_authorizations FOR EACH ROW EXECUTE FUNCTION game_node_bootstrap_authorization_guard();

CREATE FUNCTION game_node_registration_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.node_id <> OLD.node_id
       OR NEW.registration_revision <> OLD.registration_revision
       OR NEW.authorization_digest <> OLD.authorization_digest
       OR NEW.launch_binding <> OLD.launch_binding
       OR NEW.registered_at <> OLD.registered_at
       OR OLD.state <> 1
       OR NEW.state = 1 THEN
        RAISE EXCEPTION 'GameNode registration can only leave CURRENT once' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_node_registration_guard BEFORE UPDATE OR DELETE
    ON game_node_registrations FOR EACH ROW EXECUTE FUNCTION game_node_registration_guard();

-- Immutable registration endings (2 REVOKED, 3 SUPERSEDED). Each ending takes
-- its own writer revision, so registration and ending revisions together must
-- cover 1..high-water exactly; a restore that rolls a registration row back to
-- CURRENT, or drops an ending, is detected rather than trusted.
CREATE TABLE game_node_registration_endings (
    node_id UUID PRIMARY KEY REFERENCES game_node_registrations (node_id),
    ending_revision NUMERIC(20, 0) NOT NULL UNIQUE
        CHECK (ending_revision BETWEEN 1 AND 18446744073709551615),
    state SMALLINT NOT NULL CHECK (state IN (2, 3)),
    superseded_by UUID NULL REFERENCES game_node_registrations (node_id),
    ended_at BIGINT NOT NULL CHECK (ended_at >= 0),
    CHECK ((state = 3) = (superseded_by IS NOT NULL))
);
CREATE FUNCTION game_node_registration_ending_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'GameNode registration endings are immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_node_registration_ending_guard BEFORE UPDATE OR DELETE
    ON game_node_registration_endings FOR EACH ROW EXECUTE FUNCTION game_node_registration_ending_guard();

-- Authoritative registration history: registration and ending revisions are
-- disjoint and contiguous through the writer high-water, and every
-- registration row agrees with its immutable ending (CURRENT has none).
CREATE FUNCTION game_node_registration_history_valid() RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
BEGIN
    SELECT registration_revision_high_water INTO v_high_water
        FROM game_node_registration_writer WHERE writer_id = 1 FOR SHARE;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    IF (SELECT count(*)::numeric FROM game_node_registrations)
           + (SELECT count(*)::numeric FROM game_node_registration_endings) <> v_high_water
       OR (SELECT count(*)::numeric FROM (
               SELECT registration_revision FROM game_node_registrations
               UNION SELECT ending_revision FROM game_node_registration_endings) revisions) <> v_high_water
       OR (SELECT coalesce(min(revision), 1) FROM (
               SELECT registration_revision AS revision FROM game_node_registrations
               UNION ALL SELECT ending_revision FROM game_node_registration_endings) revisions) <> 1
       OR (SELECT coalesce(max(revision), 0) FROM (
               SELECT registration_revision AS revision FROM game_node_registrations
               UNION ALL SELECT ending_revision FROM game_node_registration_endings) revisions) <> v_high_water THEN
        RETURN FALSE;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1 FROM game_node_registrations g
        LEFT JOIN game_node_registration_endings e USING (node_id)
        WHERE (e.node_id IS NULL) <> (g.state = 1)
           OR (e.node_id IS NOT NULL AND (
                  e.state <> g.state
                  OR e.superseded_by IS DISTINCT FROM g.superseded_by
                  OR e.ended_at IS DISTINCT FROM g.ended_at))
    );
END; $$;

-- The only way a registration leaves CURRENT: allocate the next writer
-- revision, end the row and record the immutable ending in one transaction.
-- Idempotent for an identical ending; any other ended state is rejected.
CREATE FUNCTION game_node_end_registration(
    p_node_id UUID, p_registration_revision NUMERIC, p_state SMALLINT, p_superseded_by UUID)
RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
    v_row game_node_registrations%ROWTYPE;
    v_now BIGINT := floor(extract(epoch FROM statement_timestamp()))::BIGINT;
BEGIN
    IF p_state NOT IN (2, 3) OR ((p_state = 3) <> (p_superseded_by IS NOT NULL)) THEN
        RETURN FALSE;
    END IF;
    SELECT registration_revision_high_water INTO v_high_water
        FROM game_node_registration_writer WHERE writer_id = 1 FOR UPDATE;
    IF NOT FOUND OR NOT game_node_registration_history_valid() THEN
        RAISE EXCEPTION 'GameNode registration history contradicts its high-water' USING ERRCODE = 'XX000';
    END IF;
    SELECT * INTO v_row FROM game_node_registrations
        WHERE node_id = p_node_id AND registration_revision = p_registration_revision FOR UPDATE;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    IF v_row.state <> 1 THEN
        RETURN v_row.state = p_state AND v_row.superseded_by IS NOT DISTINCT FROM p_superseded_by;
    END IF;
    IF v_high_water >= 18446744073709551615 THEN
        RAISE EXCEPTION 'GameNode registration revision unavailable' USING ERRCODE = 'OTN01';
    END IF;
    v_high_water := v_high_water + 1;
    v_now := greatest(v_now, v_row.registered_at);
    UPDATE game_node_registration_writer SET registration_revision_high_water = v_high_water
        WHERE writer_id = 1;
    UPDATE game_node_registrations SET state = p_state, ended_at = v_now, superseded_by = p_superseded_by
        WHERE node_id = p_node_id;
    INSERT INTO game_node_registration_endings (node_id, ending_revision, state, superseded_by, ended_at)
        VALUES (p_node_id, v_high_water, p_state, p_superseded_by, v_now);
    RETURN TRUE;
END; $$;

-- GameNode registration boundary. The caller proves one-launch authorization by
-- presenting its secret; the NodeId is never a credential.
CREATE FUNCTION game_node_register(p_authorization BYTEA, p_launch_binding TEXT, p_node_id UUID)
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
    IF NOT FOUND OR v_authorization.launch_binding <> p_launch_binding THEN
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
    -- Retained registration and ending history must match the high-water
    -- exactly: neither ahead of it nor behind it, with no row/ending mismatch.
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

-- Control-plane currentness check of a target incarnation (no possession
-- proof). Holds a share lock until the caller's transaction ends, so a
-- concurrent revoke/supersede serializes with the caller's decision.
CREATE FUNCTION game_node_lock_current_registration(p_node_id UUID, p_registration_revision NUMERIC)
RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NOT game_node_registration_history_valid() THEN
        RETURN FALSE;
    END IF;
    PERFORM 1 FROM game_node_registrations g
        WHERE g.node_id = p_node_id AND g.registration_revision = p_registration_revision AND g.state = 1
          AND NOT EXISTS (SELECT 1 FROM game_node_registration_endings e WHERE e.node_id = g.node_id)
        FOR SHARE OF g;
    RETURN FOUND;
END; $$;

-- Current-incarnation primitive for fenced writers (S2 custody, readiness).
-- NodeId and revision are public; the caller must also prove possession of
-- the incarnation's own launch secret, checked against the retained digest.
-- Holds a share lock until the caller's transaction ends.
CREATE FUNCTION game_node_prove_current_incarnation(
    p_node_id UUID, p_registration_revision NUMERIC, p_secret BYTEA)
RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF p_secret IS NULL OR octet_length(p_secret) <> 32
       OR NOT game_node_registration_history_valid() THEN
        RETURN FALSE;
    END IF;
    PERFORM 1 FROM game_node_registrations g
        WHERE g.node_id = p_node_id AND g.registration_revision = p_registration_revision AND g.state = 1
          AND g.authorization_digest = sha256(p_secret)
          AND NOT EXISTS (SELECT 1 FROM game_node_registration_endings e WHERE e.node_id = g.node_id)
        FOR SHARE OF g;
    RETURN FOUND;
END; $$;

CREATE FUNCTION game_node_require_current(
    p_node_id UUID, p_registration_revision NUMERIC, p_secret BYTEA)
RETURNS VOID
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NOT game_node_prove_current_incarnation(p_node_id, p_registration_revision, p_secret) THEN
        RAISE EXCEPTION 'GameNode process incarnation is not current' USING ERRCODE = 'OTN02';
    END IF;
END; $$;

-- Assignment writer namespace: writer-allocated source revisions only.
CREATE TABLE game_runtime_scope_assignment_writer (
    writer_id SMALLINT PRIMARY KEY CHECK (writer_id = 1),
    source_revision_high_water NUMERIC(20, 0) NOT NULL
        CHECK (source_revision_high_water BETWEEN 0 AND 18446744073709551615)
);
INSERT INTO game_runtime_scope_assignment_writer (writer_id, source_revision_high_water) VALUES (1, 0);

-- One current record per supported Channel scope. state: 1 ASSIGNED, 2 REVOKED.
CREATE TABLE game_runtime_scope_assignments (
    scope_key BYTEA PRIMARY KEY,
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(channel_id)),
    ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (ownership_generation BETWEEN 1 AND 18446744073709551615),
    state SMALLINT NOT NULL CHECK (state IN (1, 2)),
    holder_node_id UUID NULL REFERENCES game_node_registrations (node_id),
    holder_registration_revision NUMERIC(20, 0) NULL,
    source_revision NUMERIC(20, 0) NOT NULL UNIQUE
        CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL UNIQUE CHECK (octet_length(decision_identity) BETWEEN 1 AND 64),
    operation_key BYTEA NOT NULL CHECK (octet_length(operation_key) = 32),
    decided_at BIGINT NOT NULL CHECK (decided_at >= 0),
    CHECK (scope_key = '\x01'::BYTEA || uuid_send(world_id) || uuid_send(channel_id)),
    CHECK ((state = 1) = (holder_node_id IS NOT NULL)),
    CHECK ((holder_node_id IS NULL) = (holder_registration_revision IS NULL))
);

-- Immutable receipt per accepted operation identity.
CREATE TABLE game_runtime_scope_assignment_receipts (
    operation_key BYTEA PRIMARY KEY CHECK (octet_length(operation_key) = 32),
    command BYTEA NOT NULL CHECK (octet_length(command) BETWEEN 1 AND 1024),
    scope_key BYTEA NOT NULL REFERENCES game_runtime_scope_assignments (scope_key),
    ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (ownership_generation BETWEEN 1 AND 18446744073709551615),
    state SMALLINT NOT NULL CHECK (state IN (1, 2)),
    holder_node_id UUID NULL,
    holder_registration_revision NUMERIC(20, 0) NULL,
    source_revision NUMERIC(20, 0) NOT NULL UNIQUE
        CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL UNIQUE CHECK (octet_length(decision_identity) BETWEEN 1 AND 64),
    decided_at BIGINT NOT NULL CHECK (decided_at >= 0),
    fenced_publication_revision NUMERIC(20, 0) NULL
        CHECK (fenced_publication_revision IS NULL OR fenced_publication_revision BETWEEN 1 AND 18446744073709551615),
    CHECK ((state = 1) = (holder_node_id IS NOT NULL)),
    CHECK ((holder_node_id IS NULL) = (holder_registration_revision IS NULL))
);

-- NASG-INFLIGHT durable custody: the exact binding is checkpointed before the
-- authoritative transaction and is cleared only by that transaction or by
-- reconciliation under the same row lock.
CREATE TABLE game_runtime_scope_assignment_slots (
    writer_registration TEXT PRIMARY KEY CHECK (
        octet_length(writer_registration) BETWEEN 1 AND 128
        AND writer_registration !~ '[^A-Za-z0-9._:-]'
    ),
    operation_key BYTEA NULL CHECK (operation_key IS NULL OR octet_length(operation_key) = 32),
    command BYTEA NULL CHECK (command IS NULL OR octet_length(command) BETWEEN 1 AND 1024),
    checkpointed_at BIGINT NULL CHECK (checkpointed_at IS NULL OR checkpointed_at >= 0),
    CHECK ((operation_key IS NULL) = (command IS NULL)),
    CHECK ((command IS NULL) = (checkpointed_at IS NULL))
);

CREATE FUNCTION game_runtime_scope_assignment_writer_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.writer_id <> OLD.writer_id
       OR NEW.source_revision_high_water <= OLD.source_revision_high_water THEN
        RAISE EXCEPTION 'runtime-scope assignment writer high-water cannot roll back' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_runtime_scope_assignment_writer_guard BEFORE UPDATE OR DELETE
    ON game_runtime_scope_assignment_writer FOR EACH ROW EXECUTE FUNCTION game_runtime_scope_assignment_writer_guard();

CREATE FUNCTION game_runtime_scope_assignment_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.scope_key <> OLD.scope_key
       OR NEW.world_id <> OLD.world_id
       OR NEW.channel_id <> OLD.channel_id
       OR NEW.ownership_generation <= OLD.ownership_generation
       OR NEW.source_revision <= OLD.source_revision THEN
        RAISE EXCEPTION 'runtime-scope assignment cannot roll back, reuse a generation or be deleted' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_runtime_scope_assignment_guard BEFORE UPDATE OR DELETE
    ON game_runtime_scope_assignments FOR EACH ROW EXECUTE FUNCTION game_runtime_scope_assignment_guard();

CREATE TRIGGER game_runtime_scope_assignment_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_runtime_scope_assignment_receipts FOR EACH ROW EXECUTE FUNCTION game_durability_reject_history_mutation();

-- The receipts are the authoritative retained assignment history. Locking the
-- singleton writer row makes this check stable against a concurrent successor.
-- There must be exactly one receipt for every allocated revision, and every
-- materialized current row must exactly reproduce its scope's latest receipt.
CREATE FUNCTION game_runtime_scope_assignment_history_valid() RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
BEGIN
    SELECT source_revision_high_water INTO v_high_water
        FROM game_runtime_scope_assignment_writer WHERE writer_id = 1 FOR SHARE;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    IF (SELECT count(*)::numeric FROM game_runtime_scope_assignment_receipts) <> v_high_water
       OR (SELECT coalesce(min(source_revision), 1) FROM game_runtime_scope_assignment_receipts) <> 1
       OR (SELECT coalesce(max(source_revision), 0) FROM game_runtime_scope_assignment_receipts) <> v_high_water
       OR EXISTS (
            SELECT 1 FROM game_runtime_scope_assignment_receipts r
            LEFT JOIN game_runtime_scope_assignments a USING (scope_key)
            WHERE a.scope_key IS NULL
       ) THEN
        RETURN FALSE;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1
        FROM game_runtime_scope_assignments a
        LEFT JOIN LATERAL (
            SELECT r.* FROM game_runtime_scope_assignment_receipts r
            WHERE r.scope_key = a.scope_key
            ORDER BY r.source_revision DESC LIMIT 1
        ) r ON TRUE
        WHERE r.source_revision IS NULL
           OR a.ownership_generation IS DISTINCT FROM r.ownership_generation
           OR a.state IS DISTINCT FROM r.state
           OR a.holder_node_id IS DISTINCT FROM r.holder_node_id
           OR a.holder_registration_revision IS DISTINCT FROM r.holder_registration_revision
           OR a.source_revision IS DISTINCT FROM r.source_revision
           OR a.decision_identity IS DISTINCT FROM r.decision_identity
           OR a.operation_key IS DISTINCT FROM r.operation_key
           OR a.decided_at IS DISTINCT FROM r.decided_at
    );
END; $$;

CREATE FUNCTION game_runtime_scope_assignment_slot_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.writer_registration <> OLD.writer_registration
       OR (OLD.operation_key IS NOT NULL AND NEW.operation_key IS NOT NULL
           AND (NEW.operation_key <> OLD.operation_key OR NEW.command <> OLD.command)) THEN
        RAISE EXCEPTION 'runtime-scope assignment slot binding is immutable until cleared' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_runtime_scope_assignment_slot_guard BEFORE UPDATE OR DELETE
    ON game_runtime_scope_assignment_slots FOR EACH ROW EXECUTE FUNCTION game_runtime_scope_assignment_slot_guard();

-- Transaction-bound readiness attestations: written only by the definer
-- function below after the exact current holder proved possession of its
-- incarnation secret, checked by the readiness trigger in the same
-- transaction and removed by the attesting publisher before commit. A
-- transaction id is never reused, so a retained row can never authorize
-- another transaction.
CREATE TABLE game_runtime_readiness_attestations (
    attested_xact XID8 NOT NULL,
    scope_key BYTEA NOT NULL REFERENCES game_runtime_scope_assignments (scope_key),
    holder_node_id UUID NOT NULL,
    holder_registration_revision NUMERIC(20, 0) NOT NULL,
    ownership_generation NUMERIC(20, 0) NOT NULL,
    PRIMARY KEY (attested_xact, scope_key)
);

CREATE FUNCTION game_runtime_attest_readiness(
    p_scope_key BYTEA, p_node_id UUID, p_registration_revision NUMERIC, p_secret BYTEA)
RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_assignment game_runtime_scope_assignments%ROWTYPE;
BEGIN
    IF NOT game_runtime_scope_assignment_history_valid() THEN
        RETURN FALSE;
    END IF;
    SELECT * INTO v_assignment FROM game_runtime_scope_assignments
        WHERE scope_key = p_scope_key FOR SHARE;
    IF NOT FOUND OR v_assignment.state <> 1
       OR v_assignment.holder_node_id <> p_node_id
       OR v_assignment.holder_registration_revision <> p_registration_revision
       OR NOT game_node_prove_current_incarnation(p_node_id, p_registration_revision, p_secret) THEN
        RETURN FALSE;
    END IF;
    INSERT INTO game_runtime_readiness_attestations
        (attested_xact, scope_key, holder_node_id, holder_registration_revision, ownership_generation)
        VALUES (pg_current_xact_id(), p_scope_key, p_node_id, p_registration_revision,
                v_assignment.ownership_generation)
        ON CONFLICT (attested_xact, scope_key) DO UPDATE
            SET holder_node_id = EXCLUDED.holder_node_id,
                holder_registration_revision = EXCLUDED.holder_registration_revision,
                ownership_generation = EXCLUDED.ownership_generation;
    RETURN TRUE;
END; $$;

-- Readiness fence: once a Channel scope has an assignment record, every
-- Runtime guard publication for that scope must carry the exact current
-- assignment generation, so a replaced/revoked process cannot keep advancing
-- the guard chain for an older generation (ready or not). A guard may become
-- ready only for the current ASSIGNED generation, only while its holder is the
-- current registered incarnation, and only in a transaction where that holder
-- attested possession of its incarnation secret. The assignment writer's own
-- fence runs after the assignment row advances, so it satisfies the same rule.
CREATE FUNCTION game_runtime_guard_requires_current_assignment() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_assignment game_runtime_scope_assignments%ROWTYPE;
BEGIN
    SELECT * INTO v_assignment FROM game_runtime_scope_assignments
        WHERE scope_key = NEW.scope_key FOR SHARE;
    IF NOT FOUND THEN
        RETURN NEW;
    END IF;
    -- A partially restored assignment row is never trusted for any Runtime
    -- publication: authoritative retained history must validate first.
    IF NOT game_runtime_scope_assignment_history_valid() THEN
        RAISE EXCEPTION 'runtime publication requires valid assignment history' USING ERRCODE = '23514';
    END IF;
    IF v_assignment.ownership_generation <> NEW.ownership_generation THEN
        RAISE EXCEPTION 'runtime publication requires the current assignment generation' USING ERRCODE = '23514';
    END IF;
    IF NEW.ready THEN
        IF NOT (
            v_assignment.state = 1
            AND game_node_lock_current_registration(
                v_assignment.holder_node_id, v_assignment.holder_registration_revision)
        ) THEN
            RAISE EXCEPTION 'runtime readiness requires the current scope assignment' USING ERRCODE = '23514';
        END IF;
        -- Checked, not consumed: INSERT .. ON CONFLICT DO UPDATE fires both
        -- the insert and update row triggers. The attesting publisher
        -- removes its own attestation before commit.
        PERFORM 1 FROM game_runtime_readiness_attestations
            WHERE attested_xact = pg_current_xact_id() AND scope_key = NEW.scope_key
              AND holder_node_id = v_assignment.holder_node_id
              AND holder_registration_revision = v_assignment.holder_registration_revision
              AND ownership_generation = v_assignment.ownership_generation;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'runtime readiness requires an attested current holder' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_runtime_guard_requires_current_assignment BEFORE INSERT OR UPDATE
    ON game_durability_admission_runtime_guards FOR EACH ROW
    EXECUTE FUNCTION game_runtime_guard_requires_current_assignment();

REVOKE ALL ON TABLE
    game_node_registration_writer,
    game_node_bootstrap_authorizations,
    game_node_registrations,
    game_node_registration_endings,
    game_runtime_scope_assignment_writer,
    game_runtime_scope_assignments,
    game_runtime_scope_assignment_receipts,
    game_runtime_scope_assignment_slots,
    game_runtime_readiness_attestations
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_node_is_uuid_v7(UUID),
    game_node_registration_writer_guard(),
    game_node_bootstrap_authorization_guard(),
    game_node_registration_guard(),
    game_node_registration_ending_guard(),
    game_node_registration_history_valid(),
    game_node_end_registration(UUID, NUMERIC, SMALLINT, UUID),
    game_node_register(BYTEA, TEXT, UUID),
    game_node_lock_current_registration(UUID, NUMERIC),
    game_node_prove_current_incarnation(UUID, NUMERIC, BYTEA),
    game_node_require_current(UUID, NUMERIC, BYTEA),
    game_runtime_scope_assignment_writer_guard(),
    game_runtime_scope_assignment_guard(),
    game_runtime_scope_assignment_history_valid(),
    game_runtime_scope_assignment_slot_guard(),
    game_runtime_attest_readiness(BYTEA, UUID, NUMERIC, BYTEA),
    game_runtime_guard_requires_current_assignment()
FROM PUBLIC;
