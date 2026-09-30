-- GOLD-FEE-1a: in-transaction gold fee BURN (decision `CHARACTER-GOLD-FEE-BOUNDARY-V1`,
-- `reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md` §4, owner decisions
-- D174-D178; DUR-03 §39.3 gold fee amendment).
--
-- A fee source (CHARM-6 `CharmUnassign` first) pays a fee inside its own fenced Character
-- transaction: that transaction advances `CharacterRevision` once with the source's receipt and,
-- in the same physical transaction, burns gold coin stacks from direct entries of the
-- character's equipped main backpack. Scope of this migration only:
--   * `game_item_fee_burns`: the fee transaction's item-side record, one per TransactionId and
--     one per cause occurrence forever (the same occurrence never burns twice). It binds the
--     closed cause (`CharmUnassign { charm, occurrence }`, the only `FeeBurnCause` variant), the
--     fee, the committed Character revision, the backpack and the audit event. It is not a
--     second receipt: the source's Character receipt is the transaction's receipt;
--   * `game_item_fee_burn_lines`: 1..20 BURN lines in burn order. Every line but the last burns
--     its whole stack (the item retires, quantity 0, and its backpack entry is deleted); the last
--     may keep units in its entry;
--   * GOLD-FEE-1a admits gold coins (`oteryn:item.tibia.i3031`, worth 1) only, so the change is
--     always 0 and no change MINT exists. Gold is the lowest worth, so every burn committed here
--     is exactly the decision §4.2 plan; a fee the gold cannot pay is rejected by the writer
--     until GOLD-FEE-1b admits platinum and crystal with their change MINT;
--   * the composition proof: the fee record commits only with the Character root advanced by the
--     same physical transaction to the bound committed revision (so the 0019/0020 chain guard has
--     proven the source's one receipt at that revision), its lines, its item changes, its entry
--     removals and its audit event;
--   * backpack entries, immutable since 0011, may now be deleted, only as a whole-burn line of a
--     fee record of the same physical transaction; an audit envelope above the one-item 9,216 B
--     is admitted only for a fee event (`DUR03-RL-07-FEE-BURN-ENVELOPE-BYTES` = 24,495 B).
-- Each replaced function keeps its previous body verbatim and only adds one admitting clause.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any fee
-- record exists it drops the two tables and the new functions and triggers, restores the 0015
-- item-change proof, the entry immutability trigger and the 9,216 B envelope CHECK, and revokes
-- the entry DELETE grant. After fee records exist they and their lines are retained evidence of
-- burned value: a rollback keeps them and may only stop new writes by revoking the INSERT grants.

CREATE TABLE game_item_fee_burns (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    -- 1 = CharmUnassign, the only FeeBurnCause variant (D178).
    cause_kind SMALLINT NOT NULL CHECK (cause_kind = 1),
    cause_occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(cause_occurrence_id)),
    charm_key TEXT NOT NULL CHECK (game_character_is_charm_key(charm_key)),
    -- SHA-256 of the semantic request (cause, Character, World, fee); a changed binding for
    -- the same occurrence conflicts.
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    backpack_item_instance_id UUID NOT NULL,
    -- At most 20 x 100 x 10,000 gold units (decision §4.2).
    fee_gold_units BIGINT NOT NULL CHECK (fee_gold_units BETWEEN 1 AND 20000000),
    burned_gold_units BIGINT NOT NULL CHECK (burned_gold_units BETWEEN 1 AND 20000000),
    -- GOLD-FEE-1a: gold only, so no change.
    change_gold_units BIGINT NOT NULL CHECK (change_gold_units = 0),
    line_count SMALLINT NOT NULL CHECK (line_count BETWEEN 1 AND 20),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- Stamped by 0011's trigger, never caller supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (burned_gold_units - change_gold_units = fee_gold_units),
    UNIQUE (cause_kind, cause_occurrence_id)
);

CREATE TABLE game_item_fee_burn_lines (
    transaction_id UUID NOT NULL REFERENCES game_item_fee_burns (transaction_id),
    line_ordinal SMALLINT NOT NULL CHECK (line_ordinal BETWEEN 1 AND 20),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    -- The backpack entry before the burn (and after it, for a partial burn).
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    -- Gold only in GOLD-FEE-1a.
    coin_worth BIGINT NOT NULL CHECK (coin_worth = 1),
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 1 AND 100),
    quantity_after BIGINT NOT NULL
        CHECK (quantity_after >= 0 AND quantity_after < quantity_before),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (transaction_id, line_ordinal),
    UNIQUE (transaction_id, item_instance_id)
);

-- Commit-time atomicity of one fee transaction. SECURITY INVOKER: the runtime role reads every
-- relation here already.
CREATE FUNCTION game_item_fee_burn_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    slot_item UUID;
    lines INTEGER;
    last_line INTEGER;
    burned BIGINT;
    first_item UUID;
    last_ordinal NUMERIC(20,0);
BEGIN
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT count(*), max(l.line_ordinal),
           coalesce(sum((l.quantity_before - l.quantity_after) * l.coin_worth), 0)
      INTO lines, last_line, burned
      FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id
       AND l.created_xact_id = pg_current_xact_id();
    SELECT l.item_instance_id INTO first_item FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = 1;
    SELECT l.placement_ordinal INTO last_ordinal FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = NEW.line_count;
    IF slot_item IS DISTINCT FROM NEW.backpack_item_instance_id
       OR lines <> NEW.line_count
       OR last_line IS DISTINCT FROM NEW.line_count
       OR burned <> NEW.burned_gold_units
       -- The Character change: the root, in this World, at the bound committed revision,
       -- written by this same physical transaction. The 0019/0020 chain guard proves the one
       -- receipt of that revision.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id
              AND cr.world_id = NEW.world_id
              AND cr.character_revision = NEW.committed_character_revision
              AND cr.xmin = pg_current_xact_id()::xid)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = first_item
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'fee BURN must commit with its Character change, lines and audit event together'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          LEFT JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.transaction_id = NEW.transaction_id
           AND (i.world_id IS DISTINCT FROM NEW.world_id
                OR i.definition_family <> 'Item'
                OR i.definition_production_key <> 'oteryn:item.tibia.i3031'
                OR i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
                OR i.quantity <> l.quantity_after
                OR i.lifecycle <> (CASE WHEN l.quantity_after = 0 THEN 2 ELSE 1 END)
                -- Only the last line keeps units.
                OR (l.quantity_after > 0 AND l.line_ordinal <> NEW.line_count)
                -- The real before-quantity, captured by 0011's item guard.
                OR NOT EXISTS (
                    SELECT 1 FROM game_item_transfer_quantity_evidence ev
                     WHERE ev.item_instance_id = l.item_instance_id
                       AND ev.transaction_id = NEW.transaction_id
                       AND ev.quantity_before = l.quantity_before)
                -- A whole burn leaves no location (its entry removal is proven below); a
                -- partial burn keeps its entry in this backpack.
                OR (l.quantity_after = 0 AND (
                        EXISTS (SELECT 1 FROM game_item_container_entries e
                                 WHERE e.item_instance_id = l.item_instance_id)
                     OR EXISTS (SELECT 1 FROM game_item_container_slots s
                                 WHERE s.item_instance_id = l.item_instance_id)
                     OR EXISTS (SELECT 1 FROM game_item_ground_locations g
                                 WHERE g.item_instance_id = l.item_instance_id)
                     OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                                 WHERE ce.item_instance_id = l.item_instance_id)))
                OR (l.quantity_after > 0 AND NOT EXISTS (
                        SELECT 1 FROM game_item_container_entries e
                         WHERE e.item_instance_id = l.item_instance_id
                           AND e.character_id = NEW.character_id
                           AND e.parent_item_instance_id = NEW.backpack_item_instance_id
                           AND e.placement_ordinal = l.placement_ordinal))
                -- Display order: placement ordinals strictly descend with the line order.
                OR EXISTS (
                    SELECT 1 FROM game_item_fee_burn_lines p
                     WHERE p.transaction_id = l.transaction_id
                       AND p.line_ordinal < l.line_ordinal
                       AND p.placement_ordinal <= l.placement_ordinal)))
       -- The deterministic plan: no untouched live gold stack of this backpack comes before
       -- the last line in display order.
       OR EXISTS (
        SELECT 1 FROM game_item_container_entries e
          JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
         WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
           AND i.lifecycle = 1
           AND i.definition_production_key = 'oteryn:item.tibia.i3031'
           AND e.placement_ordinal > last_ordinal
           AND NOT EXISTS (
               SELECT 1 FROM game_item_fee_burn_lines l
                WHERE l.transaction_id = NEW.transaction_id
                  AND l.item_instance_id = e.item_instance_id)) THEN
        RAISE EXCEPTION 'fee BURN lines must be the gold stacks of the backpack in display order with their exact item state'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A backpack entry ends only as a whole-burn line of a fee record of this same physical
-- transaction, at the entry's own parent (the record's backpack of the entry's Character) and
-- ordinal, with the item's last transaction that record's.
CREATE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
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
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- An audit envelope above the one-item 9,216 B belongs to a fee event of this transaction.
CREATE FUNCTION game_item_audit_envelope_size_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF octet_length(NEW.envelope) > 9216 AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burns f
         WHERE f.event_id = NEW.event_id
           AND f.transaction_id = NEW.transaction_id
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'DUR-03 audit envelope exceeds its registered shape ceiling'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0015: a live item changes only with a TRANSFER or its own DECAY_RETIRE receipt of the same
-- physical transaction. Same body, plus: or with its fee BURN line of the same physical
-- transaction.
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
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- The one-item envelope ceiling stays enforced by the trigger above; the column bound is the
-- fee shape's registered envelope row.
ALTER TABLE game_item_audit_outbox DROP CONSTRAINT game_item_audit_outbox_envelope_check;
ALTER TABLE game_item_audit_outbox
    ADD CONSTRAINT game_item_audit_outbox_envelope_check
        CHECK (octet_length(envelope) BETWEEN 1 AND 24495);

CREATE CONSTRAINT TRIGGER game_item_fee_burn_consistent
    AFTER INSERT ON game_item_fee_burns
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_fee_burn_consistency_guard();
CREATE CONSTRAINT TRIGGER game_item_audit_envelope_size_proven
    AFTER INSERT ON game_item_audit_outbox
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_audit_envelope_size_proven();

-- 0011 made backpack entries fully immutable; a fee whole burn narrows that to UPDATE.
DROP TRIGGER game_item_container_entry_immutable ON game_item_container_entries;
CREATE TRIGGER game_item_container_entry_immutable BEFORE UPDATE
    ON game_item_container_entries FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE CONSTRAINT TRIGGER game_item_container_entry_removal_proven
    AFTER DELETE ON game_item_container_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_container_entry_removal_proven();

CREATE TRIGGER game_item_fee_burn_immutable BEFORE UPDATE OR DELETE
    ON game_item_fee_burns FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_fee_burn_line_immutable BEFORE UPDATE OR DELETE
    ON game_item_fee_burn_lines FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_fee_burns_no_truncate BEFORE TRUNCATE ON game_item_fee_burns
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_fee_burn_lines_no_truncate BEFORE TRUNCATE ON game_item_fee_burn_lines
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_fee_burns_stamp_xact BEFORE INSERT
    ON game_item_fee_burns FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_item_fee_burn_lines_stamp_xact BEFORE INSERT
    ON game_item_fee_burn_lines FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_fee_burn_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_container_entry_removal_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_audit_envelope_size_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_item_fee_burns, game_item_fee_burn_lines FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_fee_burn_consistency_guard(),
    game_item_container_entry_removal_proven(),
    game_item_audit_envelope_size_proven()
FROM PUBLIC;
-- Least privilege: the writer inserts the record and its lines and deletes whole-burned
-- entries; the item UPDATE grant it already holds (0011) is admitted for a burn only by the
-- line proof above.
GRANT SELECT, INSERT ON game_item_fee_burns, game_item_fee_burn_lines TO oteryn_game_runtime;
GRANT DELETE ON game_item_container_entries TO oteryn_game_runtime;
GRANT SELECT ON game_item_fee_burns, game_item_fee_burn_lines TO oteryn_game_control;
