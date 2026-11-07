-- NPC-ECON-1: the native Bank owner's retained resolved confirmation continuation.
-- This is pending work, never a physical-SQL-sent marker, current NPC/session authority,
-- raw talk journal, balance, grant or second money writer. Native BANK-1 decides the money
-- operation and its immutable outcome under its existing fences and occurrence lock.
CREATE TABLE game_npc_bank_confirmations (
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    npc_identity BYTEA NOT NULL CHECK (octet_length(npc_identity) = 16),
    npc_key TEXT NOT NULL CHECK (octet_length(npc_key) BETWEEN 1 AND 512),
    origin_channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(origin_channel_id)),
    origin_scope_generation NUMERIC(20,0) NOT NULL CHECK (origin_scope_generation BETWEEN 1 AND 18446744073709551615),
    origin_connection_generation NUMERIC(20,0) NOT NULL CHECK (origin_connection_generation BETWEEN 1 AND 18446744073709551615),
    origin_character_lease_generation NUMERIC(20,0) NOT NULL CHECK (origin_character_lease_generation BETWEEN 1 AND 18446744073709551615),
    origin_node_id UUID NOT NULL CHECK (game_character_is_uuid_v7(origin_node_id)),
    origin_node_registration_revision NUMERIC(20,0) NOT NULL CHECK (origin_node_registration_revision BETWEEN 1 AND 18446744073709551615),
    npc_generation NUMERIC(20,0) NOT NULL CHECK (npc_generation BETWEEN 1 AND 18446744073709551615),
    catalogue_revision NUMERIC(20,0) NOT NULL CHECK (catalogue_revision BETWEEN 1 AND 18446744073709551615),
    prompt_generation NUMERIC(20,0) NOT NULL CHECK (prompt_generation BETWEEN 1 AND 18446744073709551615),
    operation_occurrence_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(operation_occurrence_id)),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    frozen_request BYTEA NOT NULL CHECK (octet_length(frozen_request) BETWEEN 1 AND 8192),
    recipient_name TEXT NULL CHECK (game_character_is_name(recipient_name)),
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (game_session_id, command_id)
);

CREATE FUNCTION game_npc_bank_confirmation_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NOT OLD.completed AND NEW.completed
       AND (to_jsonb(NEW) - 'completed') = (to_jsonb(OLD) - 'completed') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'NPC Bank confirmation is immutable except terminal completion' USING ERRCODE = '23514';
END $$;
CREATE TRIGGER game_npc_bank_confirmation_immutable
BEFORE UPDATE OR DELETE ON game_npc_bank_confirmations
FOR EACH ROW EXECUTE FUNCTION game_npc_bank_confirmation_immutable();
CREATE TRIGGER game_npc_bank_confirmation_no_truncate
BEFORE TRUNCATE ON game_npc_bank_confirmations
FOR EACH STATEMENT EXECUTE FUNCTION game_npc_bank_confirmation_immutable();

CREATE FUNCTION game_npc_bank_confirmation_proof() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    -- INSERT cannot claim completed work. The only terminal transition proves the actual
    -- native operation, including typed refusals; no balance/event is synthesized here.
    IF TG_OP = 'INSERT' AND NEW.completed THEN
        RAISE EXCEPTION 'NPC Bank continuation must first be enqueued' USING ERRCODE = '23514';
    END IF;
    IF NEW.completed AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_operations o
        WHERE o.operation_occurrence_id = NEW.operation_occurrence_id
          AND o.acting_character_id = NEW.character_id AND o.world_id = NEW.world_id
          AND o.request_binding = NEW.request_binding
    ) THEN
        RAISE EXCEPTION 'NPC Bank completion has no exact native outcome' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_bank_confirmation_proof
AFTER INSERT OR UPDATE ON game_npc_bank_confirmations
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_bank_confirmation_proof();
GRANT SELECT, INSERT, UPDATE(completed) ON game_npc_bank_confirmations TO oteryn_game_runtime;
GRANT SELECT ON game_npc_bank_confirmations TO oteryn_game_control;
-- Retain original identity and resolved bytes with the native operation. No opportunistic
-- cleanup or expiry turns unknown pending work into a fresh operation.

-- BANK_FEE0 NPC BUY: extend the existing trade cause, one actual FEE_DEBIT and one trade event.
-- No FeeBurn/Charm record, extra Bank event, or second money writer is created.
ALTER TABLE game_npc_trades
    ADD COLUMN bank_debit_gold_units BIGINT NOT NULL DEFAULT 0
        CHECK (bank_debit_gold_units BETWEEN 0 AND 999999999999),
    DROP CONSTRAINT game_npc_trades_unit_price_check,
    DROP CONSTRAINT game_npc_trades_total_gold_units_check,
    DROP CONSTRAINT game_npc_trades_check1,
    ADD CONSTRAINT game_npc_trades_unit_price_check CHECK (unit_price BETWEEN 0 AND 1000019999999),
    ADD CONSTRAINT game_npc_trades_total_gold_units_check CHECK (total_gold_units BETWEEN 0 AND 1000019999999),
    ADD CONSTRAINT game_npc_trades_conservation CHECK (CASE side
        WHEN 1 THEN burned_gold_units - minted_gold_units + bank_debit_gold_units = total_gold_units
            AND minted_gold_units <= 9999 AND line_count <= 23
        ELSE burned_gold_units = 0 AND minted_gold_units = total_gold_units
            AND total_gold_units <= 1009999 AND bank_debit_gold_units = 0 AND line_count <= 4 END),
    ADD CONSTRAINT game_npc_trades_bank_without_change CHECK (bank_debit_gold_units=0 OR minted_gold_units=0);

ALTER TABLE game_account_bank_entries
    ADD COLUMN npc_trade_transaction_id UUID NULL UNIQUE REFERENCES game_npc_trades(transaction_id)
        DEFERRABLE INITIALLY DEFERRED,
    DROP CONSTRAINT game_account_bank_entries_one_reference,
    DROP CONSTRAINT game_account_bank_entries_fee_reference,
    ADD CONSTRAINT game_account_bank_entries_one_reference
        CHECK (num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id)=1),
    ADD CONSTRAINT game_account_bank_entries_fee_reference
        CHECK ((kind=5) = (num_nonnulls(fee_transaction_id,npc_trade_transaction_id)=1));

CREATE OR REPLACE FUNCTION game_account_bank_entry_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.kind <> 5 AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_operations o
            WHERE o.transaction_id = NEW.transaction_id
              AND o.outcome = 0
              AND o.created_xact_id = pg_current_xact_id()))
       OR (NEW.kind = 5 AND NOT (EXISTS (
           SELECT 1 FROM game_item_fee_burns f
            WHERE f.transaction_id = NEW.fee_transaction_id
              AND f.bank_debit_gold_units = NEW.amount
              AND f.world_id = NEW.world_id
              AND f.character_id = NEW.acting_character_id
              AND f.created_xact_id = pg_current_xact_id()) OR EXISTS (
           SELECT 1 FROM game_npc_trades t
            WHERE t.transaction_id = NEW.npc_trade_transaction_id
              AND t.side = 1 AND t.bank_debit_gold_units = NEW.amount
              AND t.world_id = NEW.world_id AND t.character_id = NEW.acting_character_id
              AND t.created_xact_id = pg_current_xact_id())))
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

CREATE OR REPLACE FUNCTION game_npc_trade_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    slot_item UUID;
    lines INTEGER;
    last_line INTEGER;
    burned BIGINT;
    minted BIGINT;
    item_lines INTEGER;
    item_kind_lines INTEGER;
    last_ordinal NUMERIC(20,0);
    last_worth BIGINT;
    first_mint NUMERIC(20,0);
    mints INTEGER;
BEGIN
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT count(*), max(l.line_ordinal),
           coalesce(sum((l.quantity_before - l.quantity_after) * l.coin_worth)
                        FILTER (WHERE l.line_kind = 1), 0),
           coalesce(sum(l.quantity_after * l.coin_worth) FILTER (WHERE l.line_kind = 2), 0),
           count(*) FILTER (WHERE l.line_kind = CASE NEW.side WHEN 1 THEN 4 ELSE 3 END
                              AND l.item_instance_id = NEW.item_instance_id
                              AND abs(l.quantity_after - l.quantity_before) = NEW.item_units),
           count(*) FILTER (WHERE l.line_kind = CASE NEW.side WHEN 1 THEN 4 ELSE 3 END),
           min(l.placement_ordinal) FILTER (WHERE l.line_kind IN (2, 4)),
           count(*) FILTER (WHERE l.line_kind IN (2, 4))
      INTO lines, last_line, burned, minted, item_lines, item_kind_lines, first_mint, mints
      FROM game_npc_trade_lines l
     WHERE l.transaction_id = NEW.transaction_id
       AND l.created_xact_id = pg_current_xact_id();
    SELECT l.placement_ordinal, l.coin_worth INTO last_ordinal, last_worth
      FROM game_npc_trade_lines l
     WHERE l.transaction_id = NEW.transaction_id AND l.line_kind = 1
     ORDER BY l.line_ordinal DESC LIMIT 1;
    IF slot_item IS DISTINCT FROM NEW.backpack_item_instance_id
       OR lines <> NEW.line_count
       OR last_line IS DISTINCT FROM NEW.line_count
       OR burned <> NEW.burned_gold_units
       OR minted <> NEW.minted_gold_units
       -- Exactly one item line: the bought item's MINT or the sold item's BURN, and no line of
       -- the other side's kind. Every line of the item kind is counted, so a second item line of
       -- another identity is refused too.
       OR item_lines <> 1
       OR item_kind_lines <> 1
       OR EXISTS (SELECT 1 FROM game_npc_trade_lines l
                   WHERE l.transaction_id = NEW.transaction_id
                     AND l.line_kind IN (CASE NEW.side WHEN 1 THEN 3 ELSE 1 END,
                                         CASE NEW.side WHEN 1 THEN 3 ELSE 4 END))
       -- BUY burns no unit beyond the price: the change is below the last coin's worth. Only a
       -- price of 0 burns no coin.
       OR (NEW.side = 1 AND CASE WHEN last_worth IS NULL THEN NEW.total_gold_units - NEW.bank_debit_gold_units <> 0
                                 ELSE NEW.minted_gold_units >= last_worth END)
       -- Lines in order: the BURN lines first, then the MINT lines in consecutive new entries
       -- after the highest ordinal the backpack held before the trade.
       OR EXISTS (SELECT 1 FROM game_npc_trade_lines p JOIN game_npc_trade_lines l
                   USING (transaction_id)
                   WHERE l.transaction_id = NEW.transaction_id
                     AND p.line_ordinal < l.line_ordinal
                     AND p.line_kind IN (2, 4) AND l.line_kind IN (1, 3))
       OR EXISTS (SELECT 1 FROM game_npc_trade_lines l
                   WHERE l.transaction_id = NEW.transaction_id AND l.line_kind IN (2, 4)
                     AND l.placement_ordinal <> first_mint + (l.line_ordinal - (NEW.line_count - mints + 1)))
       OR first_mint <> 1 + greatest(
              (SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                 JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
                WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
                  AND i.minted_transaction_id <> NEW.transaction_id),
              (SELECT max(l.placement_ordinal) FROM game_npc_trade_lines l
                WHERE l.transaction_id = NEW.transaction_id AND l.line_kind IN (1, 3)), 0)
       -- MINT outputs: BUY's change is platinum then gold, SELL's coins crystal, platinum, gold;
       -- each coin present only when positive, so worth strictly descends.
       OR EXISTS (SELECT 1 FROM game_npc_trade_lines p JOIN game_npc_trade_lines l
                   USING (transaction_id)
                   WHERE l.transaction_id = NEW.transaction_id
                     AND p.line_kind = 2 AND l.line_kind = 2
                     AND p.line_ordinal < l.line_ordinal AND p.coin_worth <= l.coin_worth)
       OR EXISTS (SELECT 1 FROM game_npc_trade_lines l
                   WHERE l.transaction_id = NEW.transaction_id AND l.line_kind = 2
                     AND ((NEW.side = 1 AND l.coin_worth = 10000)
                          OR (l.coin_worth < 10000 AND l.quantity_after > 99)))
       -- Every item this transaction minted is one of its MINT lines.
       OR (SELECT count(*) FROM game_item_instances i
            WHERE i.minted_transaction_id = NEW.transaction_id) <> mints
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'NPC trade must commit with its lines and audit event together'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_npc_trade_lines l
          LEFT JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.transaction_id = NEW.transaction_id
           AND (i.world_id IS DISTINCT FROM NEW.world_id
                OR i.quantity <> l.quantity_after
                OR i.lifecycle <> (CASE WHEN l.quantity_after = 0 THEN 2 ELSE 1 END)
                OR (l.line_kind IN (1, 2) AND (i.definition_family <> 'Item'
                    OR game_item_coin_worth(i.definition_production_key) IS DISTINCT FROM l.coin_worth))
                OR (l.line_kind IN (3, 4) AND (i.definition_family, i.definition_production_key,
                                               i.definition_revision_ref)
                    IS DISTINCT FROM (NEW.item_definition_family,
                                      NEW.item_definition_production_key,
                                      NEW.item_definition_revision_ref))
                -- A BURN: the real before-quantity, captured by 0011's item guard; only the
                -- last coin line keeps units.
                OR (l.line_kind IN (1, 3) AND (
                        i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
                     OR i.minted_transaction_id = NEW.transaction_id
                     OR NOT EXISTS (
                        SELECT 1 FROM game_item_transfer_quantity_evidence ev
                         WHERE ev.item_instance_id = l.item_instance_id
                           AND ev.transaction_id = NEW.transaction_id
                           AND ev.quantity_before = l.quantity_before)
                     OR (l.line_kind = 1 AND l.quantity_after > 0 AND EXISTS (
                        SELECT 1 FROM game_npc_trade_lines n
                         WHERE n.transaction_id = l.transaction_id AND n.line_kind = 1
                           AND n.line_ordinal > l.line_ordinal))
                     -- A burned coin and the sold item have no contents and their default
                     -- state (no timed row).
                     OR (l.line_kind IN (1, 3) AND (
                        EXISTS (SELECT 1 FROM game_item_container_entries c
                                 WHERE c.parent_item_instance_id = l.item_instance_id)
                     OR EXISTS (SELECT 1 FROM game_item_timed_states t
                                 WHERE t.item_instance_id = l.item_instance_id)))))
                -- A MINT: a fresh item of this transaction.
                OR (l.line_kind IN (2, 4) AND (
                        i.minted_transaction_id <> NEW.transaction_id
                     OR i.last_transaction_id IS NOT NULL))
                -- A whole BURN leaves no location (its entry removal is proven by the removal
                -- guard); a partial BURN and a MINT have their entry in this backpack.
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
                           AND e.placement_ordinal = l.placement_ordinal
                           AND (l.line_kind IN (1, 3)
                                OR e.placed_transaction_id = NEW.transaction_id)))
                -- Coin BURN plan order: worth ascending, then display order (ordinals descend).
                OR (l.line_kind = 1 AND EXISTS (
                    SELECT 1 FROM game_npc_trade_lines p
                     WHERE p.transaction_id = l.transaction_id
                       AND p.line_kind = 1
                       AND p.line_ordinal < l.line_ordinal
                       AND (p.coin_worth > l.coin_worth
                            OR (p.coin_worth = l.coin_worth
                                AND p.placement_ordinal <= l.placement_ordinal))))))
       -- The deterministic fee plan: no untouched live coin stack of this backpack comes before
       -- the last BURN line in plan order. Coins minted by this transaction are not inputs.
       OR (NEW.side = 1 AND EXISTS (
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
               SELECT 1 FROM game_npc_trade_lines l
                WHERE l.transaction_id = NEW.transaction_id
                  AND l.item_instance_id = e.item_instance_id))) THEN
        RAISE EXCEPTION 'NPC trade lines must be the planned BURN and MINT lines with their exact item state'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_npc_trade_bank_part_proven() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE payer_account UUID; debits INTEGER;
BEGIN
    SELECT account_id INTO payer_account FROM game_character_roots WHERE character_id=NEW.character_id;
    SELECT count(*) INTO debits FROM game_account_bank_entries WHERE npc_trade_transaction_id=NEW.transaction_id;
    IF (NEW.bank_debit_gold_units=0 AND debits<>0)
       OR (NEW.bank_debit_gold_units>0 AND (debits<>1 OR NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries e WHERE e.npc_trade_transaction_id=NEW.transaction_id
             AND e.kind=5 AND e.account_id=payer_account AND e.world_id=NEW.world_id
             AND e.acting_character_id=NEW.character_id AND e.amount=NEW.bank_debit_gold_units
             AND e.created_xact_id=pg_current_xact_id())))
       OR (NEW.bank_debit_gold_units>0 AND (NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=NEW.transaction_id
             AND a.event_id=NEW.event_id AND a.schema_revision=2
             AND a.retention_profile_id='DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2')
           OR EXISTS (SELECT 1 FROM game_npc_trade_lines l WHERE l.transaction_id=NEW.transaction_id
                      AND l.line_kind=1 AND l.quantity_after<>0)
           OR EXISTS (SELECT 1 FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
                      WHERE e.parent_item_instance_id=NEW.backpack_item_instance_id AND i.lifecycle=1
                        AND i.definition_family='Item' AND game_item_coin_worth(i.definition_production_key) IS NOT NULL
                        AND i.minted_transaction_id<>NEW.transaction_id))) THEN
        RAISE EXCEPTION 'NPC BUY bank part requires all eligible coins and one exact same-transaction FEE_DEBIT' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_trade_bank_part_proven
AFTER INSERT ON game_npc_trades DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_npc_trade_bank_part_proven();
