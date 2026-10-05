-- GOLD-FEE-ACT-1: readiness for the event type 2 activation to V2 (decision GOLD-FEE-ACT-PACKET-1,
-- `reviews/OTERYN_GAME_GOLD_FEE_ACT1_TYPE2_ACTIVATION_DECISION_2026-10-04.md` §1.1-§1.5 and §2.1;
-- ARCH-BATCH-ROOT-PACKETS-V1 §1.7; builds on 0010, 0015 and 0072).
--
-- Scope of this migration only:
--   * `game_type2_audit_activation`, created empty. Its one row (`id = 1`) is inserted by
--     GOLD-FEE-ACT-2 and is the activation boundary. It is insert-only: no UPDATE, DELETE or
--     TRUNCATE is granted and a trigger refuses each of them;
--   * the fence `TYPE2_AUDIT_ACTIVATION_FENCE` (5716249083224801603, b"OT2AUDAC" big-endian),
--     one transaction-scoped advisory lock. Every type-2 writer takes it shared as its first
--     statement (`begin_type2_transaction`), and activation takes it exclusive. The value is
--     also defined in `item_mint_audit.rs`, and a test checks that the two agree;
--   * a `BEFORE INSERT` trigger on `game_item_audit_outbox`, the backstop for a binary that does
--     not take the fence. It takes the fence shared, reads the activation in a fresh statement,
--     and refuses `(1, V1)` once the row exists and `(2, V2)` while it does not (SQLSTATE OTA01).
--     After activation a `(1, V1)` event is admitted only when its `event_id` and
--     `envelope_sha256` equal those of a reservation frozen before activation (§1.5);
--   * the two reservation tables that persist an exact envelope (`game_item_mint_reservations`,
--     `game_item_decay_retire_reservations`) gain `type2_schema_revision` (set by the writer, NULL
--     from an older binary) and `type2_pre_activation` (set only by a trigger, under the fence).
--     Existing rows predate activation and are backfilled with the marker true. A reservation
--     inserted after activation must be revision 2, and one before it must not be. Neither
--     column can change once written.
-- With the table empty every writer still emits `(1, V1)`, so this migration changes no
-- behaviour.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. While the
-- activation table is empty it drops the triggers, the functions, the two reservation columns and
-- the table. After the row exists there is no rollback by deleting it: events admitted under V2
-- keep V2, and stopping V2 needs a reviewed successor decision (§1.6).

CREATE TABLE game_type2_audit_activation (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    activated_at TIMESTAMPTZ NOT NULL
);

CREATE TRIGGER game_type2_audit_activation_immutable BEFORE UPDATE OR DELETE
    ON game_type2_audit_activation
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_type2_audit_activation_no_truncate BEFORE TRUNCATE
    ON game_type2_audit_activation
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- §1.3. The fence and the activation read are two statements: under READ COMMITTED the read's
-- snapshot is taken after the shared lock is granted, so it sees every activation that the lock
-- waited for.
CREATE FUNCTION game_item_audit_type2_activation_guard() RETURNS trigger
LANGUAGE plpgsql VOLATILE AS $$
DECLARE
    v_activated BOOLEAN;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(5716249083224801603);
    SELECT EXISTS (SELECT 1 FROM game_type2_audit_activation WHERE id = 1) INTO v_activated;
    IF NOT v_activated THEN
        IF NEW.schema_revision = 1
           AND NEW.retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1' THEN
            RETURN NEW;
        END IF;
    ELSIF NEW.schema_revision = 2
          AND NEW.retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2' THEN
        RETURN NEW;
    -- §1.5: a candidate frozen before activation commits with its persisted V1 bytes.
    ELSIF NEW.schema_revision = 1
          AND NEW.retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1'
          AND (EXISTS (SELECT 1 FROM game_item_mint_reservations r
                       WHERE r.event_id = NEW.event_id
                         AND r.type2_pre_activation
                         AND sha256(r.envelope) = NEW.envelope_sha256)
               OR EXISTS (SELECT 1 FROM game_item_decay_retire_reservations r
                          WHERE r.event_id = NEW.event_id
                            AND r.type2_pre_activation
                            AND sha256(r.envelope) = NEW.envelope_sha256)) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'GOLD-FEE-ACT type-2 audit tuple does not match the activation'
        USING ERRCODE = 'OTA01';
END;
$$;

CREATE TRIGGER game_item_audit_outbox_type2_activation BEFORE INSERT
    ON game_item_audit_outbox
    FOR EACH ROW EXECUTE FUNCTION game_item_audit_type2_activation_guard();

-- §1.5. Whether a reservation was frozen before activation is recorded by the database in the
-- reservation's own inserting transaction, under the fence. A supplied marker is overwritten.
ALTER TABLE game_item_mint_reservations
    ADD COLUMN type2_schema_revision SMALLINT NULL CHECK (type2_schema_revision IN (1, 2)),
    ADD COLUMN type2_pre_activation BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE game_item_mint_reservations ALTER COLUMN type2_pre_activation DROP DEFAULT;
ALTER TABLE game_item_decay_retire_reservations
    ADD COLUMN type2_schema_revision SMALLINT NULL CHECK (type2_schema_revision IN (1, 2)),
    ADD COLUMN type2_pre_activation BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE game_item_decay_retire_reservations ALTER COLUMN type2_pre_activation DROP DEFAULT;

CREATE FUNCTION game_item_type2_reservation_marker() RETURNS trigger
LANGUAGE plpgsql VOLATILE AS $$
DECLARE
    v_activated BOOLEAN;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(5716249083224801603);
    SELECT EXISTS (SELECT 1 FROM game_type2_audit_activation WHERE id = 1) INTO v_activated;
    NEW.type2_pre_activation := NOT v_activated;
    -- An older binary leaves the revision NULL, which is refused after activation.
    IF v_activated IS DISTINCT FROM (NEW.type2_schema_revision IS NOT DISTINCT FROM 2) THEN
        RAISE EXCEPTION 'GOLD-FEE-ACT type-2 reservation revision does not match the activation'
            USING ERRCODE = 'OTA01';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION game_item_type2_reservation_columns_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.type2_schema_revision, NEW.type2_pre_activation)
       IS DISTINCT FROM (OLD.type2_schema_revision, OLD.type2_pre_activation) THEN
        RAISE EXCEPTION 'GOLD-FEE-ACT type-2 reservation marker is immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_item_mint_reservations_type2_marker BEFORE INSERT
    ON game_item_mint_reservations
    FOR EACH ROW EXECUTE FUNCTION game_item_type2_reservation_marker();
CREATE TRIGGER game_item_decay_retire_reservations_type2_marker BEFORE INSERT
    ON game_item_decay_retire_reservations
    FOR EACH ROW EXECUTE FUNCTION game_item_type2_reservation_marker();
CREATE TRIGGER game_item_mint_reservations_type2_columns BEFORE UPDATE
    ON game_item_mint_reservations
    FOR EACH ROW EXECUTE FUNCTION game_item_type2_reservation_columns_guard();
CREATE TRIGGER game_item_decay_retire_reservations_type2_columns BEFORE UPDATE
    ON game_item_decay_retire_reservations
    FOR EACH ROW EXECUTE FUNCTION game_item_type2_reservation_columns_guard();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_audit_type2_activation_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_type2_reservation_marker() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_type2_reservation_columns_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_type2_audit_activation FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_audit_type2_activation_guard(),
    game_item_type2_reservation_marker(),
    game_item_type2_reservation_columns_guard()
FROM PUBLIC;
-- Every type-2 transaction reads the activation; only GOLD-FEE-ACT-2's migration inserts it.
GRANT SELECT ON game_type2_audit_activation TO oteryn_game_runtime;
GRANT SELECT ON game_type2_audit_activation TO oteryn_game_control;
