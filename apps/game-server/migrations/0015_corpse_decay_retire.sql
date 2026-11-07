-- DUR-03 §39.1/§39.4 `DECAY_RETIRE` (child D3-6;
-- `reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`
-- §4.6/§4.7, decisions D135/D136).
--
-- Scope of this migration only (every other §39 obligation is unchanged):
--   * a corpse decays at the durable deadline `materialized_at + 60 s` of its
--     own CORPSE_MATERIALIZATION receipt; decay retires every live entry of
--     its container and then the corpse itself (RETIRED, quantity 0, no
--     location), never dropping anything to Ground;
--   * as N+1 separate one-item logical transactions, never one multi-item
--     transaction: one per live entry, then one for the corpse, admitted only
--     once zero live entries remain under it. Each touches one ItemInstance
--     and one location line (DUR03-RL-01/-RL-06 defaults; no new row);
--   * each step is reserved and committed under the retired ItemInstanceId
--     (one retirement per item forever; the corpse's step is thereby keyed by
--     its unique CORPSE_MATERIALIZATION receipt) with its own receipt and its
--     own admitted `CorpseDecay` audit event (operation tag 5);
--   * the deadline is judged by the DATABASE clock at commit
--     (`clock_timestamp()`), so a retirement is never early;
--   * the one Ground removal D3-4 left closed for a corpse
--     (`game_item_ground_removal_proven`) is extended, as D3-4 recorded, to
--     admit exactly the corpse's own DECAY_RETIRE step; the corpse-entry
--     removal proof and the item-change proof likewise admit a DECAY_RETIRE
--     receipt of the same physical transaction.
-- Nothing here weakens an existing guard: each replaced function keeps its
-- previous body verbatim (including the D133/D134 clauses of 0014) and only
-- adds one admitting clause for a DECAY_RETIRE receipt of this transaction.

-- One logical DECAY_RETIRE step per (item, owner generation): committed in its
-- own transaction before the first commit pass, it binds the frozen
-- TransactionId, EventId, trusted timestamp, deadline, before-state and exact
-- event bytes, the fence (the retiring owner's scope ownership generation and
-- holder node incarnation) and the DUR03-RL-08 work units already charged. A
-- later generation (restart or handoff) reserves afresh; the former
-- generation can no longer commit, and the receipt below is unique per item.
-- The only mutation is a +1 work-unit charge up to 3.
CREATE TABLE game_item_decay_retire_reservations (
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    fence_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (fence_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    corpse_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 1 AND 4294967295),
    -- NULL exactly for the corpse's own step; the entry's ordinal otherwise.
    placement_ordinal NUMERIC(20,0) NULL CHECK (placement_ordinal BETWEEN 1 AND 16),
    deadline BIGINT NOT NULL CHECK (deadline >= 60000),
    occurred_at BIGINT NOT NULL CHECK (occurred_at >= deadline),
    envelope BYTEA NOT NULL CHECK (octet_length(envelope) BETWEEN 1 AND 9216),
    fence_holder_node_id UUID NOT NULL,
    fence_holder_registration_revision NUMERIC(20,0) NOT NULL
        CHECK (fence_holder_registration_revision BETWEEN 0 AND 18446744073709551615),
    -- DUR03-RL-08: work units per logical transaction.
    work_units_used SMALLINT NOT NULL CHECK (work_units_used BETWEEN 0 AND 3),
    reserved_at BIGINT NOT NULL CHECK (reserved_at >= 0),
    CHECK ((item_instance_id = corpse_item_instance_id) = (placement_ordinal IS NULL)),
    PRIMARY KEY (item_instance_id, fence_scope_ownership_generation)
);

-- Terminal result: one DECAY_RETIRE per ItemInstance, forever. No expiry and
-- no deletion path.
CREATE TABLE game_item_decay_retire_receipts (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    corpse_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    fence_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (fence_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    transaction_id UUID NOT NULL UNIQUE,
    event_id UUID NOT NULL UNIQUE,
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 1 AND 4294967295),
    placement_ordinal NUMERIC(20,0) NULL CHECK (placement_ordinal BETWEEN 1 AND 16),
    deadline BIGINT NOT NULL CHECK (deadline >= 60000),
    occurred_at BIGINT NOT NULL CHECK (occurred_at >= deadline),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- The physical transaction that inserted this receipt (stamped below,
    -- never caller supplied): every removal and item change it proves must be
    -- of that same physical transaction.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK ((item_instance_id = corpse_item_instance_id) = (placement_ordinal IS NULL)),
    FOREIGN KEY (item_instance_id, fence_scope_ownership_generation)
        REFERENCES game_item_decay_retire_reservations (item_instance_id,
                                                         fence_scope_ownership_generation)
);

CREATE FUNCTION game_item_decay_retire_reservation_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.work_units_used = OLD.work_units_used + 1
       AND (to_jsonb(NEW) - 'work_units_used') = (to_jsonb(OLD) - 'work_units_used') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 DECAY_RETIRE reservation is immutable except its work-unit charge'
        USING ERRCODE = '23514';
END;
$$;

-- Commit-time atomicity of one DECAY_RETIRE step: its reservation, its audit
-- event, the item retired to exactly quantity 0 with no location left, the
-- removal of exactly its one corpse location proven from the guarded
-- pre-DELETE evidence of this physical transaction, the deadline reached by
-- the database clock, and for the corpse's own step no live entry left.
-- SECURITY DEFINER: the corpse receipt row lock (serializing against a
-- concurrent loot MINT into the same corpse, which takes the same lock in
-- game_item_corpse_container_entry_proven) needs UPDATE privilege on
-- game_item_mint_receipts, which the runtime role is not granted.
CREATE FUNCTION game_item_decay_retire_consistency_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    item game_item_instances%ROWTYPE;
    corpse_materialized BIGINT;
    places INTEGER;
BEGIN
    SELECT r.materialized_at INTO corpse_materialized
      FROM game_item_mint_receipts r
     WHERE r.item_instance_id = NEW.corpse_item_instance_id
       AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
       AND r.draw_ordinal = 0
       FOR UPDATE;
    IF NOT FOUND OR corpse_materialized IS NULL
       OR NEW.deadline <> corpse_materialized + 60000
       OR floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint < NEW.deadline THEN
        RAISE EXCEPTION 'DECAY_RETIRE requires a materialized corpse past its materialized_at + 60 s deadline'
            USING ERRCODE = '23514';
    END IF;
    SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_corpse_container_entries ce
             WHERE ce.item_instance_id = NEW.item_instance_id)
      INTO places;
    IF item.item_instance_id IS NULL
       OR item.lifecycle <> 2 OR item.quantity <> 0
       OR item.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR item.world_id <> NEW.world_id
       OR places <> 0
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.quantity_before)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_decay_retire_reservations v
            WHERE (v.item_instance_id, v.fence_scope_ownership_generation,
                   v.corpse_item_instance_id, v.world_id, v.channel_id, v.transaction_id,
                   v.event_id, v.quantity_before, v.deadline, v.occurred_at)
                = (NEW.item_instance_id, NEW.fence_scope_ownership_generation,
                   NEW.corpse_item_instance_id, NEW.world_id, NEW.channel_id,
                   NEW.transaction_id, NEW.event_id, NEW.quantity_before, NEW.deadline,
                   NEW.occurred_at)
              AND v.placement_ordinal IS NOT DISTINCT FROM NEW.placement_ordinal
              AND sha256(v.envelope) = NEW.envelope_sha256)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The corpse's own step: its own Ground removed in this transaction
       -- (the evidence is unique per item and only a proven removal writes
       -- it), and no live entry left under it.
       OR (NEW.item_instance_id = NEW.corpse_item_instance_id AND (
               NOT EXISTS (
                   SELECT 1 FROM game_item_ground_removal_evidence gre
                    WHERE gre.item_instance_id = NEW.item_instance_id
                      AND gre.world_id = NEW.world_id
                      AND gre.channel_id = NEW.channel_id)
               OR EXISTS (
                   SELECT 1 FROM game_item_corpse_container_entries ce
                    WHERE ce.parent_item_instance_id = NEW.corpse_item_instance_id)))
       -- An entry's step: that entry of this corpse removed in this physical
       -- transaction, while the corpse was live on Ground in this scope.
       OR (NEW.item_instance_id <> NEW.corpse_item_instance_id AND NOT EXISTS (
               SELECT 1 FROM game_item_corpse_entry_removal_evidence cre
                WHERE cre.item_instance_id = NEW.item_instance_id
                  AND cre.parent_item_instance_id = NEW.corpse_item_instance_id
                  AND cre.placement_ordinal = NEW.placement_ordinal
                  AND cre.world_id = NEW.world_id
                  AND cre.channel_id = NEW.channel_id
                  AND cre.created_xact_id = pg_current_xact_id())) THEN
        RAISE EXCEPTION 'DECAY_RETIRE must commit its reservation, audit event, removal and retirement together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_decay_retire_consistent
    AFTER INSERT ON game_item_decay_retire_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_decay_retire_consistency_guard();

-- 0011: a live item may change only with a TRANSFER receipt of the same
-- physical transaction naming it. Same body, plus: or with its own
-- DECAY_RETIRE receipt of the same physical transaction.
CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0014 (D134): a Ground location ends only as the source of a TRANSFER
-- committed in the same transaction, and never for a corpse. As 0014
-- recorded, D3-6 extends this for its own transaction: the corpse's own
-- DECAY_RETIRE step, with its receipt of this same physical transaction, is
-- the one admitted removal of a corpse's Ground row. Everything after the new
-- first clause is the 0014 body verbatim, so a TRANSFER of a corpse stays
-- refused exactly as before.
CREATE OR REPLACE FUNCTION game_item_ground_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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

-- 0014: a corpse entry's custody ends only as the source of a TRANSFER
-- committed in the same physical transaction. Same body, plus: or as an
-- entry DECAY_RETIRE step of its own corpse with its receipt of this same
-- physical transaction.
CREATE OR REPLACE FUNCTION game_item_corpse_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.parent_item_instance_id
           AND d.item_instance_id <> d.corpse_item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'corpse container entry removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_item_decay_retire_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_item_decay_retire_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_decay_retire_reservation_guard BEFORE UPDATE OR DELETE
    ON game_item_decay_retire_reservations FOR EACH ROW
    EXECUTE FUNCTION game_item_decay_retire_reservation_guard();
CREATE TRIGGER game_item_decay_retire_receipts_no_truncate BEFORE TRUNCATE
    ON game_item_decay_retire_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_decay_retire_reservations_no_truncate BEFORE TRUNCATE
    ON game_item_decay_retire_reservations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
-- `created_xact_id` is never caller supplied (0011's stamp function).
CREATE TRIGGER game_item_decay_retire_receipts_stamp_xact BEFORE INSERT
    ON game_item_decay_retire_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_decay_retire_reservation_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_decay_retire_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_removal_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_corpse_entry_removal_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON
    game_item_decay_retire_reservations,
    game_item_decay_retire_receipts
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_decay_retire_reservation_guard(),
    game_item_decay_retire_consistency_guard()
FROM PUBLIC;
-- Least privilege: the runtime role reserves and records DECAY_RETIRE steps;
-- the item UPDATE and the Ground/corpse-entry DELETE grants it already holds
-- (0011, 0014) are admitted for a retirement only by the proofs above.
GRANT SELECT, INSERT ON
    game_item_decay_retire_reservations,
    game_item_decay_retire_receipts
TO oteryn_game_runtime;
GRANT UPDATE (work_units_used) ON game_item_decay_retire_reservations TO oteryn_game_runtime;
GRANT SELECT ON
    game_item_decay_retire_reservations,
    game_item_decay_retire_receipts
TO oteryn_game_control;
