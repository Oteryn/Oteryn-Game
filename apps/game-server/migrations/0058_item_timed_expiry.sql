-- TIMED-RT-1b (control-plane lease 0058; decision
-- `reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md` §8,
-- §12; DUR-03 §39.3 timed-item amendment).
--
-- Scope of this migration only:
--   * the expiry causes `Expire TimeExhausted` (2) and `Expire ChargesExhausted` (3) of a held
--     timed item, in two shapes: the expiry TRANSFORM (PRESERVE_INSTANCE, in place: the item
--     keeps its instance, quantity and location and takes its decay target's definition, the row
--     reset to the target's full values, or spent when the target is not timed, at revision + 1)
--     and the expiry burn (the item to RETIRED, quantity 0, its one container entry removed, the
--     row left as it was);
--   * the write record carries the before definition's facts (pinned as the after ones are) and
--     the expiry's audit event id; every expiry commits with its one `timed_expiry` event;
--   * the item guards admit the in-place definition change and the burn's entry removal only
--     with the expiry record of the same physical transaction;
--   * a checkpoint never stores 0 ms (decision §8: a value reaching 0 is an expiry).
-- The deadline causes (4-6), PutOut (7) and `npc_repair` (8) stay refused (RT-1c,
-- TIMED-REPAIR-1). A slot is never burned: an equipment slot is immutable, so an item in a slot
-- can only transform.

ALTER TABLE game_item_timed_state_writes
    -- The writer's content facts of the before definition of an expiry (NULL for a
    -- checkpoint, whose before definition is its after one).
    ADD COLUMN before_full_charges INTEGER NULL CHECK (before_full_charges BETWEEN 1 AND 65535),
    ADD COLUMN before_full_remaining_ms BIGINT NULL
        CHECK (before_full_remaining_ms BETWEEN 1 AND 604800000),
    ADD COLUMN before_lit_continuous BOOLEAN NULL,
    -- The expiry's audit event (event type 2, operation `timed_expiry`); NULL for a checkpoint.
    ADD COLUMN event_id UUID NULL CHECK (game_character_is_uuid_v7(event_id)),
    ADD CHECK ((cause IN (2, 3)) = (event_id IS NOT NULL)),
    ADD CHECK ((cause IN (2, 3)) = (before_lit_continuous IS NOT NULL)),
    ADD CHECK (cause IN (2, 3) OR (before_full_charges IS NULL AND before_full_remaining_ms IS NULL));

-- Commit-time guard of one write record (0054's, widened). SECURITY DEFINER: it pins the
-- definition facts in a table the runtime role cannot write.
CREATE OR REPLACE FUNCTION game_item_timed_state_write_consistency_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    item game_item_instances%ROWTYPE;
    timed game_item_timed_states%ROWTYPE;
    pinned game_item_timed_definitions%ROWTYPE;
    pinned_before game_item_timed_definitions%ROWTYPE;
    on_tile BOOLEAN;
    in_container BOOLEAN;
    same_definition BOOLEAN;
    target_timed BOOLEAN;
BEGIN
    -- 0058 admits the plain checkpoint and the two held expiries (see the header).
    IF NEW.cause NOT IN (1, 2, 3) THEN
        RAISE EXCEPTION 'this timed write cause is not admitted yet' USING ERRCODE = '23514';
    END IF;

    same_definition :=
        (NEW.definition_before_family, NEW.definition_before_production_key,
         NEW.definition_before_revision_ref)
      = (NEW.definition_after_family, NEW.definition_after_production_key,
         NEW.definition_after_revision_ref);
    -- An expiry TRANSFORM may target a definition that is not timed; it pins nothing.
    target_timed := NEW.full_charges IS NOT NULL OR NEW.full_remaining_ms IS NOT NULL;

    IF target_timed THEN
        INSERT INTO game_item_timed_definitions (
            definition_family, definition_production_key, definition_revision_ref,
            full_charges, full_remaining_ms, lit_continuous)
        VALUES (NEW.definition_after_family, NEW.definition_after_production_key,
                NEW.definition_after_revision_ref, NEW.full_charges, NEW.full_remaining_ms,
                NEW.lit_continuous)
        ON CONFLICT DO NOTHING;
    END IF;
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

    IF (target_timed AND (
           pinned.full_charges IS DISTINCT FROM NEW.full_charges
           OR pinned.full_remaining_ms IS DISTINCT FROM NEW.full_remaining_ms
           OR pinned.lit_continuous IS DISTINCT FROM NEW.lit_continuous))
       -- A target that is not timed: no pin exists for it and it is not lit.
       OR (NOT target_timed AND (pinned.definition_family IS NOT NULL OR NEW.lit_continuous))
       -- Only an expiry TRANSFORM has a target that is not timed.
       OR (NOT target_timed AND (NEW.cause = 1 OR same_definition))
       -- An expiry keeps the item's definition family.
       OR NEW.definition_after_family IS DISTINCT FROM NEW.definition_before_family
       -- One timed write per physical transaction (`DUR03-RL-01-TIMED`: every timed shape
       -- touches one item).
       OR EXISTS (
           SELECT 1 FROM game_item_timed_state_writes o
            WHERE o.created_xact_id = pg_current_xact_id()
              AND (o.item_instance_id, o.expected_revision)
                  <> (NEW.item_instance_id, NEW.expected_revision))
       -- Values never above the after definition, non-NULL exactly as it has the value.
       OR (NEW.full_charges IS NULL) <> (NEW.charges_after IS NULL)
       OR (NEW.full_remaining_ms IS NULL) <> (NEW.remaining_ms_after IS NULL)
       OR NEW.charges_after > NEW.full_charges
       OR NEW.remaining_ms_after > NEW.full_remaining_ms
       -- Decision §8: a stored 0 ms is an expiry, never a stored value.
       OR NEW.remaining_ms_after = 0
       OR item.item_instance_id IS NULL
       -- D360 arms: a deadline exactly on a lit item on a tile; a lit item in no container.
       OR (NEW.deadline_after IS NOT NULL) <> (NEW.lit_continuous AND on_tile)
       OR (NEW.lit_continuous AND in_container) THEN
        RAISE EXCEPTION 'timed write must match its row, its definition and its item'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.cause = 1 THEN
        -- Checkpoint (§6.3): the row ends at expected + 1 with the after values (an absent row
        -- before is the definition's full values); no transform, no deadline, the live item in
        -- its holder's custody, values never above the previous ones.
        IF timed.item_instance_id IS NULL
           OR timed.state_revision <> NEW.expected_revision + 1
           OR timed.charges IS DISTINCT FROM NEW.charges_after
           OR timed.remaining_ms IS DISTINCT FROM NEW.remaining_ms_after
           OR timed.deadline_at IS DISTINCT FROM NEW.deadline_after
           OR (NEW.expected_revision = 0 AND (
                   NEW.charges_before IS DISTINCT FROM NEW.full_charges
                   OR NEW.remaining_ms_before IS DISTINCT FROM NEW.full_remaining_ms
                   OR NEW.deadline_before IS NOT NULL))
           OR item.lifecycle <> 1
           OR (item.definition_family, item.definition_production_key,
               item.definition_revision_ref)
              <> (NEW.definition_after_family, NEW.definition_after_production_key,
                  NEW.definition_after_revision_ref)
           OR NEW.holder_character_id IS NULL
           OR NOT same_definition
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
    END IF;

    -- Expiry (§8). The before definition's facts are pinned as the after ones are.
    INSERT INTO game_item_timed_definitions (
        definition_family, definition_production_key, definition_revision_ref,
        full_charges, full_remaining_ms, lit_continuous)
    VALUES (NEW.definition_before_family, NEW.definition_before_production_key,
            NEW.definition_before_revision_ref, NEW.before_full_charges,
            NEW.before_full_remaining_ms, NEW.before_lit_continuous)
    ON CONFLICT DO NOTHING;
    SELECT * INTO pinned_before FROM game_item_timed_definitions d
     WHERE (d.definition_family, d.definition_production_key, d.definition_revision_ref)
         = (NEW.definition_before_family, NEW.definition_before_production_key,
            NEW.definition_before_revision_ref);

    IF pinned_before.full_charges IS DISTINCT FROM NEW.before_full_charges
       OR pinned_before.full_remaining_ms IS DISTINCT FROM NEW.before_full_remaining_ms
       OR pinned_before.lit_continuous IS DISTINCT FROM NEW.before_lit_continuous
       OR NEW.holder_character_id IS NULL
       -- The reason is a value the before definition has (TimeExhausted 2, ChargesExhausted 3).
       OR (NEW.cause = 2 AND NEW.before_full_remaining_ms IS NULL)
       OR (NEW.cause = 3 AND NEW.before_full_charges IS NULL)
       -- No deadline on either side (a deadline expiry is RT-1c's cause 4).
       OR NEW.deadline_before IS NOT NULL
       OR NEW.deadline_after IS NOT NULL
       -- The row before: the before definition's values, never above them, its full values
       -- when absent.
       OR (NEW.before_full_charges IS NULL) <> (NEW.charges_before IS NULL)
       OR (NEW.before_full_remaining_ms IS NULL) <> (NEW.remaining_ms_before IS NULL)
       OR NEW.charges_before > NEW.before_full_charges
       OR NEW.remaining_ms_before > NEW.before_full_remaining_ms
       OR (NEW.expected_revision = 0 AND (
               NEW.charges_before IS DISTINCT FROM NEW.before_full_charges
               OR NEW.remaining_ms_before IS DISTINCT FROM NEW.before_full_remaining_ms))
       -- The item's last transaction is this one.
       OR item.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       -- Its one `timed_expiry` event commits in this same physical transaction.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id
              AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.event_type_id = 2
              AND a.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a timed expiry must match its before definition, its row and its event'
            USING ERRCODE = '23514';
    END IF;

    IF NOT same_definition THEN
        -- Expiry TRANSFORM: the same live item, held by its holder, now at the target; the row
        -- at expected + 1 with the target's full values, or spent.
        IF item.lifecycle <> 1
           OR (item.definition_family, item.definition_production_key,
               item.definition_revision_ref)
              <> (NEW.definition_after_family, NEW.definition_after_production_key,
                  NEW.definition_after_revision_ref)
           OR timed.item_instance_id IS NULL
           OR timed.state_revision <> NEW.expected_revision + 1
           OR timed.charges IS DISTINCT FROM NEW.charges_after
           OR timed.remaining_ms IS DISTINCT FROM NEW.remaining_ms_after
           OR timed.deadline_at IS NOT NULL
           OR NEW.charges_after IS DISTINCT FROM NEW.full_charges
           OR NEW.remaining_ms_after IS DISTINCT FROM NEW.full_remaining_ms
           OR NOT (
               EXISTS (SELECT 1 FROM game_item_container_slots s
                        WHERE s.item_instance_id = NEW.item_instance_id
                          AND s.character_id = NEW.holder_character_id)
               OR EXISTS (SELECT 1 FROM game_item_container_entries e
                           WHERE e.item_instance_id = NEW.item_instance_id
                             AND e.character_id = NEW.holder_character_id)) THEN
            RAISE EXCEPTION 'a timed expiry TRANSFORM keeps its held item and resets its row'
                USING ERRCODE = '23514';
        END IF;
    ELSE
        -- Expiry burn: the item RETIRED (its entry removal is proven by
        -- game_item_container_entry_removal_proven against this record's holder); the row left
        -- as it was (absent at expected 0) and the record's after equal to its before; no
        -- contents.
        IF item.lifecycle <> 2
           OR item.quantity <> 0
           OR NEW.full_charges IS DISTINCT FROM NEW.before_full_charges
           OR NEW.full_remaining_ms IS DISTINCT FROM NEW.before_full_remaining_ms
           OR NEW.lit_continuous IS DISTINCT FROM NEW.before_lit_continuous
           OR NEW.charges_after IS DISTINCT FROM NEW.charges_before
           OR NEW.remaining_ms_after IS DISTINCT FROM NEW.remaining_ms_before
           OR (NEW.expected_revision = 0 AND timed.item_instance_id IS NOT NULL)
           OR (NEW.expected_revision > 0 AND (
                   timed.item_instance_id IS NULL
                   OR timed.state_revision <> NEW.expected_revision
                   OR timed.charges IS DISTINCT FROM NEW.charges_before
                   OR timed.remaining_ms IS DISTINCT FROM NEW.remaining_ms_before
                   OR timed.deadline_at IS NOT NULL))
           OR EXISTS (SELECT 1 FROM game_item_container_entries e
                       WHERE e.parent_item_instance_id = NEW.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                       WHERE ce.parent_item_instance_id = NEW.item_instance_id) THEN
            RAISE EXCEPTION 'a timed expiry burn retires its item and leaves its row'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- 0011's guard plus the expiry TRANSFORM: a live item may also change its definition in place
-- (quantity, lifecycle and everything else unchanged) to a new transaction that is an expiry
-- record of this physical transaction from exactly that definition to exactly the new one. The
-- record is inserted before the item update.
CREATE OR REPLACE FUNCTION game_item_instance_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'quantity' - 'lifecycle' - 'last_transaction_id')
         = (to_jsonb(OLD) - 'quantity' - 'lifecycle' - 'last_transaction_id') THEN
        INSERT INTO game_item_transfer_quantity_evidence
            (item_instance_id, transaction_id, quantity_before)
        VALUES (OLD.item_instance_id, NEW.last_transaction_id, OLD.quantity);
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1 AND NEW.lifecycle = 1
       AND NEW.quantity = OLD.quantity
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'definition_production_key'
                - 'definition_revision_ref' - 'last_transaction_id')
         = (to_jsonb(OLD) - 'definition_production_key'
                - 'definition_revision_ref' - 'last_transaction_id')
       AND EXISTS (
           SELECT 1 FROM game_item_timed_state_writes w
            WHERE w.item_instance_id = NEW.item_instance_id
              AND w.transaction_id = NEW.last_transaction_id
              AND w.cause IN (2, 3)
              AND w.created_xact_id = pg_current_xact_id()
              AND (w.definition_before_family, w.definition_before_production_key,
                   w.definition_before_revision_ref)
                = (OLD.definition_family, OLD.definition_production_key,
                   OLD.definition_revision_ref)
              AND (w.definition_after_family, w.definition_after_production_key,
                   w.definition_after_revision_ref)
                = (NEW.definition_family, NEW.definition_production_key,
                   NEW.definition_revision_ref)) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item record changes only through a TRANSFER'
        USING ERRCODE = '23514';
END;
$$;

-- 0023's body plus: or with its timed expiry record of the same physical transaction.
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
           AND d.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          -- The line's fee record is this same physical transaction's own record, never a
          -- committed one a later line is appended to.
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
                                    AND f.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
         WHERE w.transaction_id = NEW.last_transaction_id
           AND w.item_instance_id = NEW.item_instance_id
           AND w.cause IN (2, 3)
           AND w.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0023's body plus: or as the entry of an expiry burn of this same physical transaction, held
-- by the record's holder, with the item's last transaction that record's.
CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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
           AND f.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
          JOIN game_item_instances i ON i.item_instance_id = w.item_instance_id
         WHERE w.item_instance_id = OLD.item_instance_id
           AND w.cause IN (2, 3)
           AND w.holder_character_id = OLD.character_id
           AND (w.definition_before_family, w.definition_before_production_key,
                w.definition_before_revision_ref)
             = (w.definition_after_family, w.definition_after_production_key,
                w.definition_after_revision_ref)
           AND i.lifecycle = 2
           AND i.last_transaction_id = w.transaction_id
           AND w.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_timed_state_write_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_container_entry_removal_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

-- The expiry TRANSFORM's in-place definition change (proven by the guard above). The family
-- never changes: a target is always an Item definition.
GRANT UPDATE (definition_production_key, definition_revision_ref)
    ON game_item_instances TO oteryn_game_runtime;
