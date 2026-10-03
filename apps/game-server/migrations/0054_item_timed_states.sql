-- TIMED-RT-1a (control-plane lease D353; decision
-- `reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md` §4-§6,
-- §12; TIMED-ITEM-0 §4).
--
-- Scope of this migration only:
--   * `game_item_timed_states`: TIMED-ITEM-0 §4's lazy row of a timed item (charges and
--     remaining time) plus TIMED-ITEM-0B's `deadline_at` (D360), with its partial index;
--   * `game_item_timed_state_writes`: one immutable write record per committed write of a
--     row, keyed by (item, expected revision), so at most one commit per key is a database
--     fact (§4 "Write records");
--   * `game_item_timed_definitions`: the full values and the lit flag a writer asserted for a
--     definition revision, pinned by its first write so no later write can claim others;
--   * the guard: rows are created at revision 1 and only grow by 1, never deleted; every row
--     write is proven by its record of the same physical transaction, and every record by its
--     row; values never exceed the pinned full values; a deadline only on a lit item on a
--     Ground or house tile; a lit item never in a container.
-- Admitted cause in this migration: `Checkpoint` only (§6.3 plain checkpoint, the item in its
-- holder's custody, values never above the previous ones). The other `TimedItemCause`
-- variants (Expire, SetDeadline, ClearDeadline, PutOut) and `npc_repair` are typed here and
-- refused until the migration that admits their TRANSFORM, BURN or move lines replaces the
-- record guard (TIMED-RT-1b, TIMED-REPAIR-1). No MINT, move or existing item shape writes a
-- row, so no existing item changes.

-- Lazy row of a timed item (TIMED-ITEM-0 §4). An item without a row has its definition's full
-- values and is revision 0.
CREATE TABLE game_item_timed_states (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    -- TIMEDITEM0-RL-01: 65,535 charges; 0 is never stored.
    charges INTEGER NULL CHECK (charges BETWEEN 1 AND 65535),
    -- TIMEDITEM0-RL-02: 7 days.
    remaining_ms BIGINT NULL CHECK (remaining_ms BETWEEN 0 AND 604800000),
    -- Database time (UTC ms) at which a deadline item expires (TIMED-ITEM-0B §10.3).
    deadline_at BIGINT NULL CHECK (deadline_at > 0),
    state_revision NUMERIC(20,0) NOT NULL
        CHECK (state_revision BETWEEN 1 AND 18446744073709551615),
    -- While a deadline is set, `remaining_ms` holds the budget at the moment it was set.
    CHECK (deadline_at IS NULL OR remaining_ms IS NOT NULL)
);

-- The expiry scheduler's read (§4): only rows with a deadline.
CREATE INDEX game_item_timed_states_deadline
    ON game_item_timed_states (deadline_at) WHERE deadline_at IS NOT NULL;

-- Full values of a definition revision, pinned by the first write that names it (a content
-- revision that changes them is a new revision_ref, TIMED-ITEM-0B §4 "Content
-- compatibility"). Written only by the record guard below.
CREATE TABLE game_item_timed_definitions (
    definition_family TEXT NOT NULL CHECK (octet_length(definition_family) BETWEEN 1 AND 128),
    definition_production_key TEXT NOT NULL
        CHECK (octet_length(definition_production_key) BETWEEN 1 AND 512),
    definition_revision_ref TEXT NOT NULL
        CHECK (octet_length(definition_revision_ref) BETWEEN 1 AND 512),
    full_charges INTEGER NULL CHECK (full_charges BETWEEN 1 AND 65535),
    full_remaining_ms BIGINT NULL CHECK (full_remaining_ms BETWEEN 1 AND 604800000),
    -- A lit `continuous` form (§10).
    lit_continuous BOOLEAN NOT NULL,
    CHECK (NOT lit_continuous OR full_remaining_ms IS NOT NULL),
    PRIMARY KEY (definition_family, definition_production_key, definition_revision_ref)
);

-- One record per committed write of a timed row (§4 "Write records"): the DUR-03 §24 receipt
-- of the write. The primary key is the write's idempotency key.
CREATE TABLE game_item_timed_state_writes (
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    -- 0 for "no row yet".
    expected_revision NUMERIC(20,0) NOT NULL
        CHECK (expected_revision BETWEEN 0 AND 18446744073709551614),
    -- TimedItemCause (§12): 1 Checkpoint, 2 Expire TimeExhausted, 3 Expire ChargesExhausted,
    -- 4 Expire Deadline, 5 SetDeadline, 6 ClearDeadline, 7 PutOut; 8 npc_repair.
    cause SMALLINT NOT NULL CHECK (cause BETWEEN 1 AND 8),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    -- The holder whose `character_root` lock the write ran under; NULL for a tile owner's write
    -- of a deadline item (§10.3).
    holder_character_id UUID NULL REFERENCES game_character_roots (character_id),
    definition_before_family TEXT NOT NULL
        CHECK (octet_length(definition_before_family) BETWEEN 1 AND 128),
    definition_before_production_key TEXT NOT NULL
        CHECK (octet_length(definition_before_production_key) BETWEEN 1 AND 512),
    definition_before_revision_ref TEXT NOT NULL
        CHECK (octet_length(definition_before_revision_ref) BETWEEN 1 AND 512),
    definition_after_family TEXT NOT NULL
        CHECK (octet_length(definition_after_family) BETWEEN 1 AND 128),
    definition_after_production_key TEXT NOT NULL
        CHECK (octet_length(definition_after_production_key) BETWEEN 1 AND 512),
    definition_after_revision_ref TEXT NOT NULL
        CHECK (octet_length(definition_after_revision_ref) BETWEEN 1 AND 512),
    -- The writer's content facts of the after definition (for an inactive form, of its paired
    -- active form), checked against the pinned row above.
    full_charges INTEGER NULL CHECK (full_charges BETWEEN 1 AND 65535),
    full_remaining_ms BIGINT NULL CHECK (full_remaining_ms BETWEEN 1 AND 604800000),
    lit_continuous BOOLEAN NOT NULL,
    -- The row before (the definition's full values for an absent row) and after.
    charges_before INTEGER NULL CHECK (charges_before BETWEEN 1 AND 65535),
    remaining_ms_before BIGINT NULL CHECK (remaining_ms_before BETWEEN 0 AND 604800000),
    deadline_before BIGINT NULL CHECK (deadline_before > 0),
    charges_after INTEGER NULL CHECK (charges_after BETWEEN 1 AND 65535),
    remaining_ms_after BIGINT NULL CHECK (remaining_ms_after BETWEEN 0 AND 604800000),
    deadline_after BIGINT NULL CHECK (deadline_after > 0),
    -- Stamped below, never caller supplied.
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (item_instance_id, expected_revision)
);

CREATE FUNCTION game_item_timed_state_write_stamp() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_xact_id := pg_current_xact_id();
    NEW.committed_at := floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint;
    RETURN NEW;
END;
$$;

-- Immediate row guard: a row is inserted at revision 1 and updated by exactly +1, each time
-- with the record of this physical transaction that names its previous revision and whose
-- before and after values are the row before and after. Never deleted while the item lives.
CREATE FUNCTION game_item_timed_state_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    expected NUMERIC(20,0);
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'a timed row is never deleted' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        expected := 0;
        IF NEW.state_revision <> 1 THEN
            RAISE EXCEPTION 'a timed row is created at revision 1' USING ERRCODE = '23514';
        END IF;
    ELSE
        expected := OLD.state_revision;
        IF NEW.item_instance_id <> OLD.item_instance_id
           OR NEW.state_revision <> OLD.state_revision + 1 THEN
            RAISE EXCEPTION 'a timed row revision grows by exactly 1' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
         WHERE w.item_instance_id = NEW.item_instance_id
           AND w.expected_revision = expected
           AND w.created_xact_id = pg_current_xact_id()
           AND w.charges_after IS NOT DISTINCT FROM NEW.charges
           AND w.remaining_ms_after IS NOT DISTINCT FROM NEW.remaining_ms
           AND w.deadline_after IS NOT DISTINCT FROM NEW.deadline_at
           AND (TG_OP = 'INSERT'
                OR (w.charges_before IS NOT DISTINCT FROM OLD.charges
                    AND w.remaining_ms_before IS NOT DISTINCT FROM OLD.remaining_ms
                    AND w.deadline_before IS NOT DISTINCT FROM OLD.deadline_at))) THEN
        RAISE EXCEPTION 'a timed row write must commit with its write record'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- Commit-time guard of one write record. SECURITY DEFINER: it pins the definition facts in a
-- table the runtime role cannot write.
CREATE FUNCTION game_item_timed_state_write_consistency_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    item game_item_instances%ROWTYPE;
    timed game_item_timed_states%ROWTYPE;
    pinned game_item_timed_definitions%ROWTYPE;
    on_tile BOOLEAN;
    in_container BOOLEAN;
BEGIN
    -- 0054 admits the plain checkpoint only (see the header).
    IF NEW.cause <> 1 THEN
        RAISE EXCEPTION 'this timed write cause is not admitted yet' USING ERRCODE = '23514';
    END IF;

    INSERT INTO game_item_timed_definitions (
        definition_family, definition_production_key, definition_revision_ref,
        full_charges, full_remaining_ms, lit_continuous)
    VALUES (NEW.definition_after_family, NEW.definition_after_production_key,
            NEW.definition_after_revision_ref, NEW.full_charges, NEW.full_remaining_ms,
            NEW.lit_continuous)
    ON CONFLICT DO NOTHING;
    SELECT * INTO pinned FROM game_item_timed_definitions d
     WHERE (d.definition_family, d.definition_production_key, d.definition_revision_ref)
         = (NEW.definition_after_family, NEW.definition_after_production_key,
            NEW.definition_after_revision_ref);

    SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    SELECT * INTO timed FROM game_item_timed_states
     WHERE item_instance_id = NEW.item_instance_id;
    SELECT EXISTS (SELECT 1 FROM game_item_ground_locations g
                    WHERE g.item_instance_id = NEW.item_instance_id)
        OR EXISTS (SELECT 1 FROM game_item_house_interior_locations h
                    WHERE h.item_instance_id = NEW.item_instance_id)
      INTO on_tile;
    SELECT EXISTS (SELECT 1 FROM game_item_container_entries e
                    WHERE e.item_instance_id = NEW.item_instance_id)
        OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                    WHERE ce.item_instance_id = NEW.item_instance_id)
      INTO in_container;

    IF pinned.full_charges IS DISTINCT FROM NEW.full_charges
       OR pinned.full_remaining_ms IS DISTINCT FROM NEW.full_remaining_ms
       OR pinned.lit_continuous IS DISTINCT FROM NEW.lit_continuous
       -- A timed definition has charges or a duration.
       OR (NEW.full_charges IS NULL AND NEW.full_remaining_ms IS NULL)
       -- One timed write per item per physical transaction.
       OR EXISTS (
           SELECT 1 FROM game_item_timed_state_writes o
            WHERE o.item_instance_id = NEW.item_instance_id
              AND o.created_xact_id = pg_current_xact_id()
              AND o.expected_revision <> NEW.expected_revision)
       -- The row ends at expected + 1 with exactly the after values.
       OR timed.item_instance_id IS NULL
       OR timed.state_revision <> NEW.expected_revision + 1
       OR timed.charges IS DISTINCT FROM NEW.charges_after
       OR timed.remaining_ms IS DISTINCT FROM NEW.remaining_ms_after
       OR timed.deadline_at IS DISTINCT FROM NEW.deadline_after
       -- An absent row is the definition's full values.
       OR (NEW.expected_revision = 0 AND (
               NEW.charges_before IS DISTINCT FROM NEW.full_charges
               OR NEW.remaining_ms_before IS DISTINCT FROM NEW.full_remaining_ms
               OR NEW.deadline_before IS NOT NULL))
       -- Never above the definition, and non-NULL exactly as the definition has the value.
       OR (NEW.full_charges IS NULL) <> (NEW.charges_after IS NULL)
       OR (NEW.full_remaining_ms IS NULL) <> (NEW.remaining_ms_after IS NULL)
       OR NEW.charges_after > NEW.full_charges
       OR NEW.remaining_ms_after > NEW.full_remaining_ms
       -- The item is live at this definition.
       OR item.item_instance_id IS NULL
       OR item.lifecycle <> 1
       OR (item.definition_family, item.definition_production_key,
           item.definition_revision_ref)
          <> (NEW.definition_after_family, NEW.definition_after_production_key,
              NEW.definition_after_revision_ref)
       -- D360 arms: a deadline exactly on a lit item on a tile; a lit item in no container.
       OR (NEW.deadline_after IS NOT NULL) <> (NEW.lit_continuous AND on_tile)
       OR (NEW.lit_continuous AND in_container) THEN
        RAISE EXCEPTION 'timed write must match its row, its definition and its item'
            USING ERRCODE = '23514';
    END IF;

    -- Checkpoint (§6.3): no transform, no deadline, the item in its holder's custody, values
    -- never above the previous ones.
    IF NEW.holder_character_id IS NULL
       OR (NEW.definition_before_family, NEW.definition_before_production_key,
           NEW.definition_before_revision_ref)
          <> (NEW.definition_after_family, NEW.definition_after_production_key,
              NEW.definition_after_revision_ref)
       OR NEW.deadline_before IS NOT NULL
       OR (NEW.charges_before IS NULL) <> (NEW.charges_after IS NULL)
       OR (NEW.remaining_ms_before IS NULL) <> (NEW.remaining_ms_after IS NULL)
       OR NEW.charges_after > NEW.charges_before
       OR NEW.remaining_ms_after > NEW.remaining_ms_before
       OR (NEW.charges_after IS NOT DISTINCT FROM NEW.charges_before
           AND NEW.remaining_ms_after IS NOT DISTINCT FROM NEW.remaining_ms_before)
       OR NOT (
           EXISTS (SELECT 1 FROM game_item_container_slots s
                    WHERE s.item_instance_id = NEW.item_instance_id
                      AND s.character_id = NEW.holder_character_id)
           OR EXISTS (SELECT 1 FROM game_item_container_entries e
                       WHERE e.item_instance_id = NEW.item_instance_id
                         AND e.character_id = NEW.holder_character_id)) THEN
        RAISE EXCEPTION 'a timed checkpoint stores a lower live value of an item its holder holds'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Commit-time: a row insert or update is proven by its record (the record guard then checks
-- the rest). Reads the row's current state, so a later write in the same physical
-- transaction is judged by its own record.
CREATE FUNCTION game_item_timed_state_row_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_timed_states t
          JOIN game_item_timed_state_writes w
            ON w.item_instance_id = t.item_instance_id
           AND w.expected_revision = t.state_revision - 1
           AND w.created_xact_id = pg_current_xact_id()
         WHERE t.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'a timed row write must commit with its write record'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_item_timed_state_write_stamp BEFORE INSERT
    ON game_item_timed_state_writes FOR EACH ROW
    EXECUTE FUNCTION game_item_timed_state_write_stamp();
CREATE TRIGGER game_item_timed_state_row_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_item_timed_states FOR EACH ROW EXECUTE FUNCTION game_item_timed_state_row_guard();
CREATE CONSTRAINT TRIGGER game_item_timed_state_row_proven
    AFTER INSERT OR UPDATE ON game_item_timed_states
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_timed_state_row_proven();
CREATE CONSTRAINT TRIGGER game_item_timed_state_write_consistent
    AFTER INSERT ON game_item_timed_state_writes
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_timed_state_write_consistency_guard();
CREATE TRIGGER game_item_timed_state_writes_immutable BEFORE UPDATE OR DELETE
    ON game_item_timed_state_writes FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_timed_definitions_immutable BEFORE UPDATE OR DELETE
    ON game_item_timed_definitions FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_timed_states_no_truncate BEFORE TRUNCATE
    ON game_item_timed_states
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_timed_state_writes_no_truncate BEFORE TRUNCATE
    ON game_item_timed_state_writes
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_timed_definitions_no_truncate BEFORE TRUNCATE
    ON game_item_timed_definitions
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_timed_state_write_stamp() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_timed_state_row_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_timed_state_write_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_timed_state_row_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON
    game_item_timed_states,
    game_item_timed_state_writes,
    game_item_timed_definitions
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_timed_state_write_stamp(),
    game_item_timed_state_row_guard(),
    game_item_timed_state_write_consistency_guard(),
    game_item_timed_state_row_proven()
FROM PUBLIC;
-- Least privilege (§4): the runtime role inserts records and inserts and updates rows, never
-- deletes; the definition pins are written by the guard only.
GRANT SELECT, INSERT ON game_item_timed_state_writes TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_item_timed_states TO oteryn_game_runtime;
GRANT SELECT ON game_item_timed_definitions TO oteryn_game_runtime;
GRANT SELECT ON
    game_item_timed_states,
    game_item_timed_state_writes,
    game_item_timed_definitions
TO oteryn_game_control;
