-- GOLD-FEE-2: the bank part of a gold fee (decision `BANK-FEE0-COINS-THEN-BANK-V1`,
-- `reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md` §3-§5; BANK-0 Q1 = b;
-- ARCH-BATCH-ROOT-PACKETS-V1 §1.7 phase 1 and §2.5; builds on 0010, 0023, 0031 and 0071).
--
-- Coins first: when the eligible coin stacks of the payer's main backpack are worth `T` less
-- than the fee `F`, a non-junior payer burns every one of them whole, mints no change and pays
-- `F - T` from its (Account, World) bank balance by one `FEE_DEBIT` ledger entry in the same
-- physical transaction. Scope of this migration only:
--   * the fee record (0023) gains `bank_debit_gold_units` (0 when the bank is not used) and the
--     conservation `burned - change + bank_debit = fee`. `line_count` and `burned` may be 0, and
--     the backpack NULL, only for a fee paid wholly from the bank. The fee is bounded by the
--     coin part (20,000,000) plus `BANK0-RL-01`;
--   * `game_item_fee_burn_consistency_guard` is replaced: every 0031 arm is kept, written so
--     that no arm relies on a NULL first item, last line or last ordinal, and it adds the
--     coins-first arm (a bank part needs every line whole, no change and no untouched coin stack
--     of the backpack) and the same-transaction arm (a bank part if and only if exactly one
--     `FEE_DEBIT` entry of this physical transaction for this fee, with the same amount, the
--     payer root's Account, the record's World and the payer as acting character);
--   * the ledger (0071) gains kind 5 `FEE_DEBIT` and the fee reference: an entry references
--     exactly one of a bank operation and a fee record. `game_account_bank_entry_consistency_guard`
--     keeps its 0071 arms and proves a `FEE_DEBIT` entry by its fee record of this physical
--     transaction instead of a bank operation. A `FEE_DEBIT` emits no bank event;
--   * the item audit outbox (0010): `item_instance_id` may be NULL only for the event of a fee
--     paid wholly from the bank (a new deferred proof), and the two single-value CHECKs on
--     `schema_revision` and `retention_profile_id` become one tuple CHECK, `(1, V1)` or `(2, V2)`.
--     Every stored row is `(1, V1)` and stays valid; every writer still emits `(1, V1)` until
--     GOLD-FEE-ACT-2 binds V2 (§1.7, GOLD-FEE-ACT-PACKET-1);
--   * the stored fee envelope ceiling stays `DUR03-RL-07-FEE-BURN-ENVELOPE-BYTES`, re-measured
--     with the value line (unchanged: a bank part has no change MINT).
-- Existing fee records keep bank part 0 and verify as before.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any fee
-- record with a bank part and any `(2, V2)` audit row exist it restores the 0031 guard body and
-- the 0071 entry guard body, drops the outbox item proof, the fee record column and the ledger
-- column and CHECKs, restores the single-value outbox CHECKs, the NOT NULL columns and the
-- 0023/0031 fee CHECKs. After such rows exist they are retained evidence and authoritative bank
-- state (BANK-0 §3): a rollback keeps them and may only stop new bank parts in the writer.

ALTER TABLE game_item_fee_burns
    ADD COLUMN bank_debit_gold_units BIGINT NOT NULL DEFAULT 0
        CHECK (bank_debit_gold_units BETWEEN 0 AND 999999999999),
    ALTER COLUMN backpack_item_instance_id DROP NOT NULL,
    DROP CONSTRAINT game_item_fee_burns_fee_gold_units_check,
    DROP CONSTRAINT game_item_fee_burns_burned_gold_units_check,
    DROP CONSTRAINT game_item_fee_burns_line_count_check,
    DROP CONSTRAINT game_item_fee_burns_check,
    -- The coin part (20 x 100 x 10,000) plus BANK0-RL-01.
    ADD CONSTRAINT game_item_fee_burns_fee_gold_units_check
        CHECK (fee_gold_units BETWEEN 1 AND 1000019999999),
    ADD CONSTRAINT game_item_fee_burns_burned_gold_units_check
        CHECK (burned_gold_units BETWEEN 0 AND 20000000),
    ADD CONSTRAINT game_item_fee_burns_line_count_check
        CHECK (line_count BETWEEN 0 AND 20),
    ADD CONSTRAINT game_item_fee_burns_conservation
        CHECK (burned_gold_units - change_gold_units + bank_debit_gold_units = fee_gold_units),
    -- No line burns nothing, and only a fee paid wholly from the bank has no line or backpack.
    ADD CONSTRAINT game_item_fee_burns_bank_only
        CHECK ((line_count = 0) = (burned_gold_units = 0)
               AND (line_count > 0 OR bank_debit_gold_units = fee_gold_units)
               AND (backpack_item_instance_id IS NOT NULL OR line_count = 0)),
    -- Coins first: a bank part mints no change.
    ADD CONSTRAINT game_item_fee_burns_bank_without_change
        CHECK (bank_debit_gold_units = 0 OR change_gold_units = 0);

-- 1 = DEPOSIT, 2 = WITHDRAW, 3 = TRANSFER_OUT, 4 = TRANSFER_IN, 5 = FEE_DEBIT. A FEE_DEBIT entry
-- references its fee record, never a bank operation.
ALTER TABLE game_account_bank_entries
    ALTER COLUMN transaction_id DROP NOT NULL,
    ADD COLUMN fee_transaction_id UUID NULL UNIQUE,
    DROP CONSTRAINT game_account_bank_entries_kind_check,
    ADD CONSTRAINT game_account_bank_entries_kind_check CHECK (kind IN (1, 2, 3, 4, 5)),
    ADD CONSTRAINT game_account_bank_entries_one_reference
        CHECK ((transaction_id IS NULL) <> (fee_transaction_id IS NULL)),
    ADD CONSTRAINT game_account_bank_entries_fee_reference
        CHECK ((kind = 5) = (fee_transaction_id IS NOT NULL)),
    ADD CONSTRAINT game_account_bank_entries_fee_transaction_id_fkey
        FOREIGN KEY (fee_transaction_id) REFERENCES game_item_fee_burns (transaction_id)
        DEFERRABLE INITIALLY DEFERRED;

-- (schema_revision, retention_profile_id) is (1, V1) or (2, V2) (#1733 P1 4176934053).
ALTER TABLE game_item_audit_outbox
    ALTER COLUMN item_instance_id DROP NOT NULL,
    DROP CONSTRAINT game_item_audit_outbox_schema_revision_check,
    DROP CONSTRAINT game_item_audit_outbox_retention_profile_id_check,
    ADD CONSTRAINT game_item_audit_outbox_event_tuple
        CHECK ((schema_revision, retention_profile_id) IN (
            (1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1'),
            (2, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2')));

-- 0031's guard with the bank part.
CREATE OR REPLACE FUNCTION game_item_fee_burn_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    slot_item UUID;
    lines INTEGER;
    last_line INTEGER;
    burned BIGINT;
    first_item UUID;
    last_ordinal NUMERIC(20,0);
    last_worth BIGINT;
    payer_account UUID;
    debits INTEGER;
    outputs INTEGER := (NEW.change_gold_units / 100 > 0)::integer
                     + (NEW.change_gold_units % 100 > 0)::integer;
BEGIN
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT count(*), coalesce(max(l.line_ordinal), 0),
           coalesce(sum((l.quantity_before - l.quantity_after) * l.coin_worth), 0)
      INTO lines, last_line, burned
      FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id
       AND l.created_xact_id = pg_current_xact_id();
    SELECT l.item_instance_id INTO first_item FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = 1;
    SELECT l.placement_ordinal, l.coin_worth INTO last_ordinal, last_worth
      FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = NEW.line_count;
    SELECT cr.account_id INTO payer_account FROM game_character_roots cr
     WHERE cr.character_id = NEW.character_id;
    SELECT count(*) INTO debits FROM game_account_bank_entries e
     WHERE e.fee_transaction_id = NEW.transaction_id;
    IF slot_item IS DISTINCT FROM NEW.backpack_item_instance_id
       OR lines <> NEW.line_count
       OR last_line <> NEW.line_count
       OR burned <> NEW.burned_gold_units
       -- The plan burns no unit beyond the fee: the change is below the last coin's worth.
       OR (NEW.line_count > 0 AND NEW.change_gold_units >= last_worth)
       -- The Character change: the root, in this World, at the bound committed revision,
       -- written by this same physical transaction. The 0019/0020 chain guard proves the one
       -- receipt of that revision.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id
              AND cr.world_id = NEW.world_id
              AND cr.character_revision = NEW.committed_character_revision
              AND cr.xmin = pg_current_xact_id()::xid)
       -- The one event: found through the first burn line, or for a fee paid wholly from the
       -- bank through this record (its item is NULL).
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id IS NOT DISTINCT FROM first_item
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
                OR game_item_coin_worth(i.definition_production_key) IS DISTINCT FROM l.coin_worth
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
                -- Plan order: worth ascending, then display order (ordinals descend).
                OR EXISTS (
                    SELECT 1 FROM game_item_fee_burn_lines p
                     WHERE p.transaction_id = l.transaction_id
                       AND p.line_ordinal < l.line_ordinal
                       AND (p.coin_worth > l.coin_worth
                            OR (p.coin_worth = l.coin_worth
                                AND p.placement_ordinal <= l.placement_ordinal)))))
       -- The deterministic plan: no untouched live coin stack of this backpack comes before the
       -- last line in plan order. The change minted by this transaction is not an input.
       OR (NEW.line_count > 0 AND EXISTS (
        SELECT 1 FROM game_item_container_entries e
          JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
         WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
           AND i.lifecycle = 1
           AND i.definition_family = 'Item'
           AND i.minted_transaction_id <> NEW.transaction_id
           AND (game_item_coin_worth(i.definition_production_key) < last_worth
                OR (game_item_coin_worth(i.definition_production_key) = last_worth
                    AND e.placement_ordinal > last_ordinal))
           AND NOT EXISTS (
               SELECT 1 FROM game_item_fee_burn_lines l
                WHERE l.transaction_id = NEW.transaction_id
                  AND l.item_instance_id = e.item_instance_id))) THEN
        RAISE EXCEPTION 'fee BURN lines must be the coin stacks of the backpack in plan order with their exact item state'
            USING ERRCODE = '23514';
    END IF;
    -- Coins first (BANK-FEE-0 §4.1): a bank part burns every line whole, mints no change and
    -- leaves no live coin stack of the backpack untouched. No arm reads the first or last line.
    IF NEW.bank_debit_gold_units > 0 AND (
           NEW.change_gold_units <> 0
        OR EXISTS (
            SELECT 1 FROM game_item_fee_burn_lines l
             WHERE l.transaction_id = NEW.transaction_id AND l.quantity_after <> 0)
        OR EXISTS (
            SELECT 1 FROM game_item_container_entries e
              JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
             WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
               AND i.lifecycle = 1
               AND i.definition_family = 'Item'
               AND game_item_coin_worth(i.definition_production_key) IS NOT NULL
               AND NOT EXISTS (
                   SELECT 1 FROM game_item_fee_burn_lines l
                    WHERE l.transaction_id = NEW.transaction_id
                      AND l.item_instance_id = e.item_instance_id))) THEN
        RAISE EXCEPTION 'a fee with a bank part must burn every coin stack of the backpack whole first'
            USING ERRCODE = '23514';
    END IF;
    -- The same-transaction bank part (BANK-FEE-0 §4.2), both ways: a bank part if and only if
    -- exactly one FEE_DEBIT entry of this physical transaction references this fee, with its
    -- amount, the payer root's Account, this World and the payer as acting character.
    IF debits <> (NEW.bank_debit_gold_units > 0)::integer
       OR (NEW.bank_debit_gold_units > 0 AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries e
            WHERE e.fee_transaction_id = NEW.transaction_id
              AND e.kind = 5
              AND e.amount = NEW.bank_debit_gold_units
              AND e.account_id = payer_account
              AND e.world_id = NEW.world_id
              AND e.acting_character_id = NEW.character_id
              AND e.created_xact_id = pg_current_xact_id())) THEN
        RAISE EXCEPTION 'a fee bank part must commit with its one FEE_DEBIT ledger entry'
            USING ERRCODE = '23514';
    END IF;
    -- The change MINT: exactly the planned outputs, each a fresh live coin of this World with
    -- one location, its new entry in this backpack, placed after the burn lines.
    IF (SELECT count(*) FROM game_item_instances i
         WHERE i.minted_transaction_id = NEW.transaction_id) <> outputs
       OR (outputs > 0 AND NEW.change_placement_ordinal <> 1 + greatest(
              (SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                 JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
                WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
                  AND i.minted_transaction_id <> NEW.transaction_id),
              (SELECT max(l.placement_ordinal) FROM game_item_fee_burn_lines l
                WHERE l.transaction_id = NEW.transaction_id)))
       OR EXISTS (
        SELECT 1 FROM (VALUES
               (NEW.change_platinum_item_instance_id, 'oteryn:item.tibia.i3035',
                NEW.change_gold_units / 100, NEW.change_placement_ordinal),
               (NEW.change_gold_item_instance_id, 'oteryn:item.tibia.i3031',
                NEW.change_gold_units % 100,
                NEW.change_placement_ordinal + (NEW.change_gold_units / 100 > 0)::integer))
               AS o(item_instance_id, production_key, quantity, placement_ordinal)
         WHERE o.quantity > 0
           AND NOT EXISTS (
               SELECT 1 FROM game_item_instances i
                 JOIN game_item_container_entries e ON e.item_instance_id = i.item_instance_id
                WHERE i.item_instance_id = o.item_instance_id
                  AND i.world_id = NEW.world_id
                  AND i.definition_family = 'Item'
                  AND i.definition_production_key = o.production_key
                  AND i.quantity = o.quantity
                  AND i.lifecycle = 1
                  AND i.minted_transaction_id = NEW.transaction_id
                  AND i.last_transaction_id IS NULL
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = NEW.backpack_item_instance_id
                  AND e.placement_ordinal = o.placement_ordinal
                  AND e.placed_transaction_id = NEW.transaction_id
                  AND NOT EXISTS (SELECT 1 FROM game_item_container_slots s
                                   WHERE s.item_instance_id = i.item_instance_id)
                  AND NOT EXISTS (SELECT 1 FROM game_item_ground_locations g
                                   WHERE g.item_instance_id = i.item_instance_id)
                  AND NOT EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                                   WHERE ce.item_instance_id = i.item_instance_id))) THEN
        RAISE EXCEPTION 'fee change MINT must be the planned platinum and gold outputs in new backpack entries'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0071's guard, same arms, plus: a FEE_DEBIT entry is proven by its fee record of this same
-- physical transaction (amount, World and payer; the record's guard proves the Account), not by
-- a bank operation.
CREATE OR REPLACE FUNCTION game_account_bank_entry_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.kind <> 5 AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_operations o
            WHERE o.transaction_id = NEW.transaction_id
              AND o.outcome = 0
              AND o.created_xact_id = pg_current_xact_id()))
       OR (NEW.kind = 5 AND NOT EXISTS (
           SELECT 1 FROM game_item_fee_burns f
            WHERE f.transaction_id = NEW.fee_transaction_id
              AND f.bank_debit_gold_units = NEW.amount
              AND f.world_id = NEW.world_id
              AND f.character_id = NEW.acting_character_id
              AND f.created_xact_id = pg_current_xact_id()))
       OR NEW.created_xact_id <> pg_current_xact_id()
       OR (NEW.previous_entry_id IS NULL AND NEW.balance_before <> 0)
       OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries p
            WHERE p.entry_id = NEW.previous_entry_id
              AND p.account_id = NEW.account_id AND p.world_id = NEW.world_id
              AND p.balance_after = NEW.balance_before))
       OR NOT (EXISTS (
           SELECT 1 FROM game_account_bank_balances b
            WHERE b.account_id = NEW.account_id AND b.world_id = NEW.world_id
              AND b.last_entry_id = NEW.entry_id)
           OR EXISTS (
           SELECT 1 FROM game_account_bank_entries s WHERE s.previous_entry_id = NEW.entry_id))
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = CASE WHEN NEW.kind = 4 THEN NEW.recipient_character_id
                                         ELSE NEW.acting_character_id END
              AND cr.account_id = NEW.account_id
              AND cr.world_id = NEW.world_id
              AND cr.lifecycle = 1)
       OR (NEW.recipient_character_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.recipient_character_id
              AND cr.world_id = NEW.world_id AND cr.lifecycle = 1))
       OR (NEW.counterpart_entry_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries c
            WHERE c.entry_id = NEW.counterpart_entry_id
              AND c.transaction_id = NEW.transaction_id
              AND c.counterpart_entry_id = NEW.entry_id
              AND c.amount = NEW.amount
              AND c.account_id <> NEW.account_id
              AND c.kind = 7 - NEW.kind)) THEN
        RAISE EXCEPTION 'a bank ledger entry must chain from the previous entry of its live account balance'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- An audit row without an item is the event of a fee paid wholly from the bank, committed with
-- its record (no burn line) by this same physical transaction.
CREATE FUNCTION game_item_audit_item_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.item_instance_id IS NULL AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burns f
         WHERE f.event_id = NEW.event_id
           AND f.transaction_id = NEW.transaction_id
           AND f.line_count = 0
           AND f.bank_debit_gold_units = f.fee_gold_units
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a DUR-03 audit event names its item unless it is a fee paid wholly from the bank'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_audit_item_proven
    AFTER INSERT ON game_item_audit_outbox
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_audit_item_proven();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_fee_burn_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_entry_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_audit_item_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON FUNCTION game_item_audit_item_proven() FROM PUBLIC;
-- The runtime role's grants are unchanged: it already inserts fee records (0023), ledger
-- entries and balance updates (0071) and audit rows (0010).
