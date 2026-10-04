-- BANK-1: the Account bank balance (decision `BANK0-ACCOUNT-WORLD-BANK-BALANCE-V1`,
-- `reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md` §3-§5 and §8; owner
-- answers 1b and Q1 b; ARCH-BATCH-ROOT-PACKETS-V1 §1.7, §1.8 and §2.4).
--
-- One gold balance per (Account, World), shared by the Account's characters of that World and
-- moved only by a bank operation of one of the kinds DEPOSIT, WITHDRAW or TRANSFER. Scope of
-- this migration only:
--   * `game_account_bank_operations`: one row per operation, keyed by its runtime occurrence,
--     with the TransactionId and EventId fixed before the first attempt, the SHA-256 binding of
--     the whole request (the same occurrence and binding replay the first outcome; a changed
--     binding conflicts) and the typed outcome. A refused operation keeps its row and nothing
--     else: no ledger entry, coin line, balance change or event;
--   * `game_account_bank_entries`: the immutable ledger, one row per balance change, chained by
--     `previous_entry_id` per (Account, World); a transfer's two entries name each other;
--   * `game_account_bank_balances`: the current balance and its latest entry. No row means 0; the
--     writer's upsert creates a value-neutral zero row before it locks it;
--   * `game_account_bank_coin_lines`: the coin items a deposit consumes (inputs, by the gold fee
--     plan's selection, D175-D177) or a deposit's change and a withdrawal mint (outputs), every
--     one a CONVERSION line (BANK-0 §5);
--   * `game_account_bank_audit_outbox`: the bank event (event type 3 `BANK_OPERATION`, retention
--     `ECONOMY_LEDGER_RETENTION_V1`, P30D), one per committed operation. It is the only bank
--     relation retention deletes: an unheld row at or past `expires_at`, through
--     `game_account_bank_expire_audit`; explicit legal holds (`game_account_bank_audit_legal_holds`,
--     the registry's legal_hold_policy, as 0005 does for Character audit) keep a row past expiry;
--   * deferred guards: the balance equals its latest entry and each entry's before equals the
--     previous entry's after; the acting character (and a transfer's recipient) is a live root
--     of the entry's Account and World; a transfer's two entries commit together with equal
--     amounts on different Accounts; a deposit's input worth minus its output worth equals its
--     credit and a withdrawal's output worth equals its debit; an operation has exactly the
--     entries, lines and event its kind needs;
--   * the item proofs keep their bodies verbatim and add one admitting clause each for a coin
--     line of a bank operation of the same physical transaction: the item change and entry
--     removal proofs (0058 bodies), the MINT guard and the placement proof (0031 bodies).
-- No CharacterRevision advance (BANK-0 §4.1). Existing fee records (0023, 0031) are unchanged.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any bank
-- operation exists it drops the six tables and the new functions and triggers and restores the
-- four item proofs to their 0058 and 0031 bodies. After operations exist, balances, entries,
-- operations and coin lines are authoritative game state (BANK-0 §3): a rollback keeps them and
-- may only stop new writes by revoking the INSERT and UPDATE grants.

CREATE TABLE game_account_bank_operations (
    operation_occurrence_id UUID PRIMARY KEY
        CHECK (game_character_is_uuid_v7(operation_occurrence_id)),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    -- Fixed before the first attempt; an event with this id exists only for outcome 0.
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    -- 1 = DEPOSIT, 2 = WITHDRAW, 3 = TRANSFER.
    kind SMALLINT NOT NULL CHECK (kind IN (1, 2, 3)),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    acting_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards (account_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    -- BANK0-RL-02-DEPOSIT, -WITHDRAW and -TRANSFER.
    amount BIGINT NOT NULL CHECK (amount BETWEEN 1 AND 999999999999),
    -- The confirmed recipient. Not a foreign key: an UNKNOWN_RECIPIENT refusal records an id
    -- that may name no root; a committed transfer's guard proves the live recipient root.
    recipient_character_id UUID NULL CHECK (game_character_is_uuid_v7(recipient_character_id)),
    recipient_name_key TEXT NULL CHECK (recipient_name_key ~ '^[a-z]{2,29}$'),
    -- The planned output identity slots: a deposit's change (platinum, gold), a withdrawal's
    -- coins (crystal, platinum, gold). An unused slot never exists as an item.
    planned_crystal_item_instance_id UUID NULL
        CHECK (game_character_is_uuid_v7(planned_crystal_item_instance_id)),
    planned_platinum_item_instance_id UUID NULL
        CHECK (game_character_is_uuid_v7(planned_platinum_item_instance_id)),
    planned_gold_item_instance_id UUID NULL
        CHECK (game_character_is_uuid_v7(planned_gold_item_instance_id)),
    -- The acting character's main backpack, for a committed deposit or withdrawal.
    backpack_item_instance_id UUID NULL,
    -- 0 = OK, 1 = BALANCE_LIMIT, 2 = INSUFFICIENT_BALANCE, 3 = INSUFFICIENT_COINS, 4 = NO_ROOM,
    -- 5 = JUNIOR_ACCOUNT, 6 = UNKNOWN_RECIPIENT, 7 = RECIPIENT_CANNOT_RECEIVE_TRANSFERS,
    -- 8 = SAME_ACCOUNT (BANK-0 §4.1, §4.3).
    outcome SMALLINT NOT NULL CHECK (outcome BETWEEN 0 AND 8),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (transaction_id <> event_id),
    CHECK (kind <> 1 OR amount <= 20000000),
    CHECK (kind <> 2 OR amount <= 1009999),
    CHECK ((kind = 3) = (recipient_character_id IS NOT NULL)),
    CHECK ((kind = 3) = (recipient_name_key IS NOT NULL)),
    CHECK (CASE kind
        WHEN 1 THEN planned_crystal_item_instance_id IS NULL
                    AND planned_platinum_item_instance_id IS NOT NULL
                    AND planned_gold_item_instance_id IS NOT NULL
        WHEN 2 THEN planned_crystal_item_instance_id IS NOT NULL
                    AND planned_platinum_item_instance_id IS NOT NULL
                    AND planned_gold_item_instance_id IS NOT NULL
        ELSE planned_crystal_item_instance_id IS NULL
             AND planned_platinum_item_instance_id IS NULL
             AND planned_gold_item_instance_id IS NULL
    END),
    -- Distinct slots (a NULL slot compares unknown and passes).
    CHECK (planned_crystal_item_instance_id <> planned_platinum_item_instance_id
           AND planned_crystal_item_instance_id <> planned_gold_item_instance_id
           AND planned_platinum_item_instance_id <> planned_gold_item_instance_id),
    CHECK (CASE kind
        WHEN 1 THEN outcome IN (0, 1, 3, 4, 5)
        WHEN 2 THEN outcome IN (0, 2, 4, 5)
        ELSE outcome IN (0, 2, 5, 6, 7, 8)
    END),
    CHECK ((outcome = 0) = (envelope_sha256 IS NOT NULL)),
    CHECK (outcome = 0 OR backpack_item_instance_id IS NULL),
    CHECK ((outcome = 0 AND kind IN (1, 2)) = (backpack_item_instance_id IS NOT NULL))
);

CREATE TABLE game_account_bank_entries (
    entry_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(entry_id)),
    transaction_id UUID NOT NULL REFERENCES game_account_bank_operations (transaction_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards (account_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    -- 1 = DEPOSIT, 2 = WITHDRAW, 3 = TRANSFER_OUT, 4 = TRANSFER_IN.
    kind SMALLINT NOT NULL CHECK (kind IN (1, 2, 3, 4)),
    amount BIGINT NOT NULL CHECK (amount BETWEEN 1 AND 999999999999),
    balance_before BIGINT NOT NULL CHECK (balance_before BETWEEN 0 AND 999999999999),
    balance_after BIGINT NOT NULL CHECK (balance_after BETWEEN 0 AND 999999999999),
    -- The chain of this (Account, World): NULL only for its first entry.
    previous_entry_id UUID NULL UNIQUE REFERENCES game_account_bank_entries (entry_id),
    acting_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    recipient_character_id UUID NULL REFERENCES game_character_roots (character_id),
    counterpart_entry_id UUID NULL UNIQUE,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (balance_after = CASE WHEN kind IN (1, 4) THEN balance_before + amount
                                ELSE balance_before - amount END),
    CHECK ((kind IN (3, 4)) = (recipient_character_id IS NOT NULL)),
    CHECK ((kind IN (3, 4)) = (counterpart_entry_id IS NOT NULL)),
    CHECK (counterpart_entry_id IS DISTINCT FROM entry_id),
    CHECK (previous_entry_id IS DISTINCT FROM entry_id),
    UNIQUE (transaction_id, kind),
    UNIQUE (entry_id, account_id, world_id)
);
-- One first entry per (Account, World).
CREATE UNIQUE INDEX game_account_bank_entries_first
    ON game_account_bank_entries (account_id, world_id) WHERE previous_entry_id IS NULL;
ALTER TABLE game_account_bank_entries
    ADD CONSTRAINT game_account_bank_entries_counterpart_fkey
        FOREIGN KEY (counterpart_entry_id) REFERENCES game_account_bank_entries (entry_id)
        DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE game_account_bank_balances (
    account_id UUID NOT NULL REFERENCES game_character_account_guards (account_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    balance BIGINT NOT NULL CHECK (balance BETWEEN 0 AND 999999999999),
    last_entry_id UUID NULL UNIQUE,
    PRIMARY KEY (account_id, world_id),
    CHECK (last_entry_id IS NOT NULL OR balance = 0),
    FOREIGN KEY (last_entry_id, account_id, world_id)
        REFERENCES game_account_bank_entries (entry_id, account_id, world_id)
);

CREATE TABLE game_account_bank_coin_lines (
    transaction_id UUID NOT NULL REFERENCES game_account_bank_operations (transaction_id),
    line_ordinal SMALLINT NOT NULL CHECK (line_ordinal BETWEEN 1 AND 22),
    -- 1 = input (consumed by a deposit), 2 = output (a deposit's change, a withdrawal's coins).
    direction SMALLINT NOT NULL CHECK (direction IN (1, 2)),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    coin_worth BIGINT NOT NULL CHECK (coin_worth IN (1, 100, 10000)),
    -- The backpack entry: an input's before (and after, for the partial last input), an
    -- output's new entry.
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 0 AND 100),
    quantity_after BIGINT NOT NULL CHECK (quantity_after BETWEEN 0 AND 100),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (CASE direction
        WHEN 1 THEN quantity_before >= 1 AND quantity_after < quantity_before
        ELSE quantity_before = 0 AND quantity_after >= 1
    END),
    PRIMARY KEY (transaction_id, line_ordinal),
    UNIQUE (transaction_id, item_instance_id)
);

CREATE TABLE game_account_bank_audit_outbox (
    event_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(event_id)),
    transaction_id UUID NOT NULL UNIQUE
        REFERENCES game_account_bank_operations (transaction_id),
    transaction_ordinal INTEGER NOT NULL CHECK (transaction_ordinal = 1),
    transaction_count INTEGER NOT NULL CHECK (transaction_count = 1),
    event_type_id BIGINT NOT NULL CHECK (event_type_id = 3),
    schema_revision BIGINT NOT NULL CHECK (schema_revision = 1),
    retention_profile_id TEXT NOT NULL CHECK (retention_profile_id = 'ECONOMY_LEDGER_RETENTION_V1'),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    -- ECONOMY_LEDGER_RETENTION_V1: P30D from the envelope timestamp.
    expires_at BIGINT NOT NULL CHECK (expires_at = occurred_at + 2592000000),
    -- DUR03-RL-07-BANK-ENVELOPE-BYTES.
    envelope BYTEA NOT NULL CHECK (octet_length(envelope) BETWEEN 1 AND 25719),
    envelope_sha256 BYTEA NOT NULL CHECK (envelope_sha256 = sha256(envelope)),
    publication_state SMALLINT NOT NULL CHECK (publication_state IN (1, 2)),
    published_at BIGINT NULL CHECK (published_at IS NULL OR published_at >= occurred_at),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK ((publication_state = 2) = (published_at IS NOT NULL))
);

-- Explicit legal holds on bank audit events: reason, authorizing actor, start and affected
-- record. Release returns the record to its original expiry; a hold never deletes or copies it.
CREATE TABLE game_account_bank_audit_legal_holds (
    hold_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(hold_id)),
    event_id UUID NOT NULL CHECK (game_character_is_uuid_v7(event_id)),
    reason TEXT NOT NULL CHECK (octet_length(reason) BETWEEN 1 AND 512),
    authorizing_actor TEXT NOT NULL CHECK (octet_length(authorizing_actor) BETWEEN 1 AND 128),
    started_at BIGINT NOT NULL CHECK (started_at >= 0),
    released_at BIGINT NULL CHECK (released_at IS NULL OR released_at >= started_at),
    released_by TEXT NULL CHECK (released_by IS NULL OR octet_length(released_by) BETWEEN 1 AND 128),
    CHECK ((released_at IS NULL) = (released_by IS NULL))
);
CREATE UNIQUE INDEX game_account_bank_audit_one_active_hold
    ON game_account_bank_audit_legal_holds (event_id) WHERE released_at IS NULL;

-- Commit-time shape of one operation. SECURITY INVOKER: the runtime role reads every relation
-- here already.
CREATE FUNCTION game_account_bank_operation_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    entries INTEGER;
    inputs INTEGER;
    outputs INTEGER;
    input_worth BIGINT;
    output_worth BIGINT;
    last_input INTEGER;
    last_ordinal NUMERIC(20,0);
    last_worth BIGINT;
    slot_item UUID;
    change BIGINT;
BEGIN
    SELECT count(*) INTO entries FROM game_account_bank_entries e
     WHERE e.transaction_id = NEW.transaction_id;
    SELECT count(*) FILTER (WHERE l.direction = 1), count(*) FILTER (WHERE l.direction = 2),
           coalesce(sum((l.quantity_before - l.quantity_after) * l.coin_worth)
                    FILTER (WHERE l.direction = 1), 0),
           coalesce(sum(l.quantity_after * l.coin_worth) FILTER (WHERE l.direction = 2), 0),
           max(l.line_ordinal) FILTER (WHERE l.direction = 1)
      INTO inputs, outputs, input_worth, output_worth, last_input
      FROM game_account_bank_coin_lines l
     WHERE l.transaction_id = NEW.transaction_id;
    IF NEW.outcome <> 0 THEN
        -- A refusal changes nothing: no entry, coin line or event.
        IF entries <> 0 OR inputs + outputs <> 0 OR EXISTS (
            SELECT 1 FROM game_account_bank_audit_outbox a
             WHERE a.event_id = NEW.event_id OR a.transaction_id = NEW.transaction_id) THEN
            RAISE EXCEPTION 'a refused bank operation writes no entry, coin line or event'
                USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.acting_character_id
              AND cr.account_id = NEW.account_id
              AND cr.world_id = NEW.world_id
              AND cr.lifecycle = 1)
       OR NOT EXISTS (
           SELECT 1 FROM game_account_bank_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.world_id = NEW.world_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'a bank operation must commit with its live acting character and its event'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.kind = 3 THEN
        -- TRANSFER: TRANSFER_OUT on the sender's balance and TRANSFER_IN on a live recipient of
        -- another Account in this World, naming each other, with equal amounts; no coin line.
        IF entries <> 2 OR inputs + outputs <> 0 OR NOT EXISTS (
            SELECT 1 FROM game_account_bank_entries o
              JOIN game_account_bank_entries i ON i.entry_id = o.counterpart_entry_id
              JOIN game_character_roots r ON r.character_id = NEW.recipient_character_id
             WHERE o.transaction_id = NEW.transaction_id AND o.kind = 3
               AND i.transaction_id = NEW.transaction_id AND i.kind = 4
               AND i.counterpart_entry_id = o.entry_id
               AND o.account_id = NEW.account_id AND o.world_id = NEW.world_id
               AND i.account_id = r.account_id AND i.world_id = NEW.world_id
               AND r.world_id = NEW.world_id AND r.lifecycle = 1
               AND r.name_key = NEW.recipient_name_key
               AND i.account_id <> o.account_id
               AND o.amount = NEW.amount AND i.amount = NEW.amount
               AND o.acting_character_id = NEW.acting_character_id
               AND i.acting_character_id = NEW.acting_character_id
               AND o.recipient_character_id = NEW.recipient_character_id
               AND i.recipient_character_id = NEW.recipient_character_id) THEN
            RAISE EXCEPTION 'a bank transfer must commit its two entries together on different accounts'
                USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    -- DEPOSIT and WITHDRAW: one entry of the operation's kind on the acting Account.
    IF entries <> 1 OR NOT EXISTS (
        SELECT 1 FROM game_account_bank_entries e
         WHERE e.transaction_id = NEW.transaction_id AND e.kind = NEW.kind
           AND e.account_id = NEW.account_id AND e.world_id = NEW.world_id
           AND e.amount = NEW.amount
           AND e.acting_character_id = NEW.acting_character_id) THEN
        RAISE EXCEPTION 'a bank deposit or withdrawal must commit exactly its one ledger entry'
            USING ERRCODE = '23514';
    END IF;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.acting_character_id;
    IF slot_item IS DISTINCT FROM NEW.backpack_item_instance_id THEN
        RAISE EXCEPTION 'bank coin lines must be in the acting character''s main backpack'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.kind = 1 THEN
        change := input_worth - NEW.amount;
        SELECT l.placement_ordinal, l.coin_worth INTO last_ordinal, last_worth
          FROM game_account_bank_coin_lines l
         WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = last_input;
        -- Worth conservation: inputs minus the change equal the credit, the change is below the
        -- last input's worth (no unit consumed beyond the plan) and is minted as platinum then
        -- gold into the planned slots.
        IF inputs NOT BETWEEN 1 AND 20
           OR last_input IS DISTINCT FROM inputs
           OR change < 0
           OR change >= last_worth
           OR output_worth <> change
           OR outputs <> (change / 100 > 0)::integer + (change % 100 > 0)::integer
           OR EXISTS (
               SELECT 1 FROM game_account_bank_coin_lines l
                WHERE l.transaction_id = NEW.transaction_id AND l.direction = 2
                  AND NOT ((l.coin_worth = 100 AND l.quantity_after = change / 100
                            AND l.item_instance_id = NEW.planned_platinum_item_instance_id)
                        OR (l.coin_worth = 1 AND l.quantity_after = change % 100
                            AND l.item_instance_id = NEW.planned_gold_item_instance_id))) THEN
            RAISE EXCEPTION 'a bank deposit''s input worth minus its change must equal its credit'
                USING ERRCODE = '23514';
        END IF;
        -- The gold fee plan's selection: inputs worth ascending, then display order, and no
        -- untouched live coin stack of this backpack before the last input in that order.
        IF EXISTS (
            SELECT 1 FROM game_account_bank_coin_lines l
              JOIN game_account_bank_coin_lines p ON p.transaction_id = l.transaction_id
             WHERE l.transaction_id = NEW.transaction_id
               AND l.direction = 1 AND p.direction = 1
               AND p.line_ordinal < l.line_ordinal
               AND (p.coin_worth > l.coin_worth
                    OR (p.coin_worth = l.coin_worth AND p.placement_ordinal <= l.placement_ordinal)))
           OR EXISTS (
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
                   SELECT 1 FROM game_account_bank_coin_lines l
                    WHERE l.transaction_id = NEW.transaction_id
                      AND l.item_instance_id = e.item_instance_id)) THEN
            RAISE EXCEPTION 'bank deposit inputs must be the coin stacks of the backpack in plan order'
                USING ERRCODE = '23514';
        END IF;
    ELSE
        -- WITHDRAW: the canonical split into the planned slots, crystal then platinum then gold.
        IF inputs <> 0
           OR output_worth <> NEW.amount
           OR outputs <> (NEW.amount / 10000 > 0)::integer
                       + (NEW.amount % 10000 / 100 > 0)::integer
                       + (NEW.amount % 100 > 0)::integer
           OR EXISTS (
               SELECT 1 FROM game_account_bank_coin_lines l
                WHERE l.transaction_id = NEW.transaction_id
                  AND NOT ((l.coin_worth = 10000 AND l.quantity_after = NEW.amount / 10000
                            AND l.item_instance_id = NEW.planned_crystal_item_instance_id)
                        OR (l.coin_worth = 100 AND l.quantity_after = NEW.amount % 10000 / 100
                            AND l.item_instance_id = NEW.planned_platinum_item_instance_id)
                        OR (l.coin_worth = 1 AND l.quantity_after = NEW.amount % 100
                            AND l.item_instance_id = NEW.planned_gold_item_instance_id))) THEN
            RAISE EXCEPTION 'a bank withdrawal''s output worth must equal its debit'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    -- Every line's exact item state. Inputs precede outputs; outputs take consecutive new
    -- entries after every other entry of the backpack.
    IF EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          LEFT JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.transaction_id = NEW.transaction_id
           AND (i.world_id IS DISTINCT FROM NEW.world_id
                OR i.definition_family <> 'Item'
                OR game_item_coin_worth(i.definition_production_key) IS DISTINCT FROM l.coin_worth
                OR (l.direction = 2 AND l.line_ordinal <= coalesce(last_input, 0))
                OR (l.direction = 1 AND (
                        i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
                     OR i.quantity <> l.quantity_after
                     OR i.lifecycle <> (CASE WHEN l.quantity_after = 0 THEN 2 ELSE 1 END)
                     OR (l.quantity_after > 0 AND l.line_ordinal <> last_input)
                     OR NOT EXISTS (
                         SELECT 1 FROM game_item_transfer_quantity_evidence ev
                          WHERE ev.item_instance_id = l.item_instance_id
                            AND ev.transaction_id = NEW.transaction_id
                            AND ev.quantity_before = l.quantity_before)
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
                                AND e.character_id = NEW.acting_character_id
                                AND e.parent_item_instance_id = NEW.backpack_item_instance_id
                                AND e.placement_ordinal = l.placement_ordinal))))
                OR (l.direction = 2 AND (
                        i.minted_transaction_id <> NEW.transaction_id
                     OR i.last_transaction_id IS NOT NULL
                     OR i.quantity <> l.quantity_after
                     OR i.lifecycle <> 1
                     OR NOT EXISTS (
                         SELECT 1 FROM game_item_container_entries e
                          WHERE e.item_instance_id = l.item_instance_id
                            AND e.character_id = NEW.acting_character_id
                            AND e.parent_item_instance_id = NEW.backpack_item_instance_id
                            AND e.placement_ordinal = l.placement_ordinal
                            AND e.placed_transaction_id = NEW.transaction_id)
                     OR EXISTS (SELECT 1 FROM game_item_container_slots s
                                 WHERE s.item_instance_id = l.item_instance_id)
                     OR EXISTS (SELECT 1 FROM game_item_ground_locations g
                                 WHERE g.item_instance_id = l.item_instance_id)
                     OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                                 WHERE ce.item_instance_id = l.item_instance_id)
                     OR l.placement_ordinal <= coalesce((
                            SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                              JOIN game_item_instances o ON o.item_instance_id = e.item_instance_id
                             WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
                               AND o.minted_transaction_id <> NEW.transaction_id), 0)
                     OR l.placement_ordinal <= coalesce((
                            SELECT max(p.placement_ordinal) FROM game_account_bank_coin_lines p
                             WHERE p.transaction_id = NEW.transaction_id AND p.direction = 1), 0)
                     -- Outputs: worth descending in consecutive entries.
                     OR EXISTS (
                         SELECT 1 FROM game_account_bank_coin_lines p
                          WHERE p.transaction_id = NEW.transaction_id AND p.direction = 2
                            AND p.line_ordinal < l.line_ordinal
                            AND (p.coin_worth <= l.coin_worth
                                 OR (p.line_ordinal = l.line_ordinal - 1
                                     AND p.placement_ordinal + 1 <> l.placement_ordinal))))))) THEN
        RAISE EXCEPTION 'bank coin lines must match their exact item state'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Commit-time chain and party proof of one ledger entry: its operation is of this physical
-- transaction, its before is the previous entry's after on the same (Account, World), it is
-- reached by the balance row or by a later entry, and its party is a live root of that Account
-- and World (the acting character, or a transfer's recipient for TRANSFER_IN).
CREATE FUNCTION game_account_bank_entry_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
           SELECT 1 FROM game_account_bank_operations o
            WHERE o.transaction_id = NEW.transaction_id
              AND o.outcome = 0
              AND o.created_xact_id = pg_current_xact_id())
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

-- The balance row equals the after value and id of its latest entry. Inserted only as the
-- value-neutral zero row; each update moves it by one new entry of this physical transaction
-- whose before and previous entry are the old balance and its latest entry.
CREATE FUNCTION game_account_bank_balance_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.balance <> 0 OR NEW.last_entry_id IS NOT NULL THEN
            RAISE EXCEPTION 'a bank balance row starts at zero with no entry'
                USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF NEW.account_id <> OLD.account_id
       OR NEW.world_id <> OLD.world_id
       OR NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries e
            WHERE e.entry_id = NEW.last_entry_id
              AND e.account_id = NEW.account_id AND e.world_id = NEW.world_id
              AND e.balance_after = NEW.balance
              AND e.balance_before = OLD.balance
              AND e.previous_entry_id IS NOT DISTINCT FROM OLD.last_entry_id
              AND e.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a bank balance must equal its latest ledger entry'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A coin line belongs to a committed operation of this same physical transaction.
CREATE FUNCTION game_account_bank_coin_line_operation_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_account_bank_operations o
         WHERE o.transaction_id = NEW.transaction_id
           AND o.outcome = 0
           AND o.kind IN (1, 2)
           AND o.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a bank coin line must commit with its operation'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A bank event belongs to the committed operation of this same physical transaction.
CREATE FUNCTION game_account_bank_event_operation_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_account_bank_operations o
         WHERE o.transaction_id = NEW.transaction_id
           AND o.event_id = NEW.event_id
           AND o.outcome = 0
           AND o.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a bank event must commit with its operation'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Audit rows are immutable except the one-way publication mark; the only deletion is ordinary
-- ECONOMY_LEDGER_RETENTION_V1 expiry of an unheld record.
CREATE FUNCTION game_account_bank_audit_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF OLD.publication_state = 1 AND NEW.publication_state = 2
           AND (to_jsonb(NEW) - 'publication_state' - 'published_at')
             = (to_jsonb(OLD) - 'publication_state' - 'published_at') THEN
            RETURN NEW;
        END IF;
    ELSE
        -- Serialize with hold placement, then check holds in a later statement that sees every
        -- committed hold.
        PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:bank-audit-retention', 0));
        IF OLD.expires_at <= floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT
           AND NOT EXISTS (SELECT 1 FROM game_account_bank_audit_legal_holds
                           WHERE event_id = OLD.event_id AND released_at IS NULL) THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION 'bank audit record is immutable until unheld ordinary expiry'
        USING ERRCODE = '23514';
END;
$$;

CREATE FUNCTION game_account_bank_audit_hold_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- Every hold change takes the same retention lock as audit deletion.
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:bank-audit-retention', 0));
    IF TG_OP = 'INSERT' THEN
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.released_at IS NULL AND NEW.released_at IS NOT NULL
       AND (NEW.hold_id, NEW.event_id, NEW.reason, NEW.authorizing_actor, NEW.started_at)
           IS NOT DISTINCT FROM (OLD.hold_id, OLD.event_id, OLD.reason, OLD.authorizing_actor, OLD.started_at) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'bank audit legal hold is append-only until its single release'
        USING ERRCODE = '23514';
END;
$$;

-- Operator-only: place an explicit legal hold on a retained bank audit event. An exact replay
-- (same event, reason and actor) returns the active hold. EXECUTE is granted to no role here.
CREATE FUNCTION game_account_bank_place_legal_hold(p_event_id UUID, p_reason TEXT, p_actor TEXT)
RETURNS UUID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_hold game_account_bank_audit_legal_holds%ROWTYPE;
    v_hold_id UUID;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:bank-audit-retention', 0));
    PERFORM 1 FROM game_account_bank_audit_outbox WHERE event_id = p_event_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'bank audit event is not retained' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_hold FROM game_account_bank_audit_legal_holds
        WHERE event_id = p_event_id AND released_at IS NULL;
    IF FOUND THEN
        IF v_hold.reason = p_reason AND v_hold.authorizing_actor = p_actor THEN
            RETURN v_hold.hold_id;
        END IF;
        RAISE EXCEPTION 'bank audit event already has an active legal hold' USING ERRCODE = '23505';
    END IF;
    INSERT INTO game_account_bank_audit_legal_holds(hold_id, event_id, reason, authorizing_actor, started_at)
    VALUES (game_character_uuid_v7(), p_event_id, p_reason, p_actor,
        floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT)
    RETURNING hold_id INTO v_hold_id;
    RETURN v_hold_id;
END;
$$;

-- Operator-only: release an active legal hold once; the event returns to its original expiry.
CREATE FUNCTION game_account_bank_release_legal_hold(p_hold_id UUID, p_actor TEXT)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    UPDATE game_account_bank_audit_legal_holds
       SET released_at = greatest(started_at, floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT),
           released_by = p_actor
     WHERE hold_id = p_hold_id AND released_at IS NULL;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'bank audit legal hold is not active' USING ERRCODE = '23514';
    END IF;
END;
$$;

-- Ordinary bank audit expiry as the one runtime deletion boundary (as 0006 does for Character
-- audit): the runtime role holds no DELETE on the outbox, and the row guard still refuses any
-- record that is unexpired or under an unreleased legal hold.
CREATE FUNCTION game_account_bank_expire_audit(p_batch INTEGER) RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_deleted BIGINT;
BEGIN
    IF p_batch IS NULL OR p_batch < 1 OR p_batch > 64 THEN
        RAISE EXCEPTION 'bank audit expiry batch is out of bounds' USING ERRCODE = '22023';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:bank-audit-retention', 0));
    WITH deleted AS (
        DELETE FROM game_account_bank_audit_outbox WHERE event_id IN (
            SELECT a.event_id FROM game_account_bank_audit_outbox a
             WHERE a.expires_at <= floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT
               AND NOT EXISTS (SELECT 1 FROM game_account_bank_audit_legal_holds h
                               WHERE h.event_id = a.event_id AND h.released_at IS NULL)
             ORDER BY a.expires_at, a.event_id LIMIT p_batch FOR UPDATE)
        RETURNING 1)
    SELECT count(*) INTO v_deleted FROM deleted;
    RETURN v_deleted;
END;
$$;

CREATE FUNCTION game_account_bank_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'bank record is immutable' USING ERRCODE = '23514';
END;
$$;

-- 0058's body plus: or with its input coin line of a bank deposit of the same physical
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
           AND w.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
                                             AND o.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.direction = 1
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0058's body plus: or as the entry of a wholly consumed input of a bank deposit of this same
-- physical transaction, at the entry's own parent (the operation's backpack of the entry's
-- Character) and ordinal, with the item's last transaction that operation's.
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
           AND w.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.direction = 1
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND o.acting_character_id = OLD.character_id
           AND o.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND o.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0031's guard, same body, plus: or an output coin line of a bank operation of the same
-- physical transaction in its new entry (the operation's guard proves the rest).
CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND r.destination_parent_item_instance_id IS NULL
           AND g.world_id = NEW.world_id
           AND g.channel_id = r.death_channel_id
           AND g.runtime_scope_ownership_generation = r.death_scope_ownership_generation
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key,
                       v.loot_table_revision_ref, v.loot_purpose_key, v.draw_ordinal,
                       v.intent_binding, v.transaction_id, v.event_id, v.item_instance_id,
                       v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key,
                       r.loot_table_revision_ref, r.loot_purpose_key, r.draw_ordinal,
                       r.intent_binding, r.transaction_id, r.event_id, r.item_instance_id,
                       r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_reward_claim_mint_receipts r
          JOIN game_item_container_entries e ON e.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.created_xact_id = pg_current_xact_id()
           AND e.placed_transaction_id = r.transaction_id
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_corpse_container_entries e ON e.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND r.destination_parent_item_instance_id = e.parent_item_instance_id
           AND r.destination_ordinal = e.placement_ordinal
           AND e.world_id = NEW.world_id
           AND e.placed_transaction_id = r.transaction_id
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key,
                       v.loot_table_revision_ref, v.loot_purpose_key, v.draw_ordinal,
                       v.intent_binding, v.transaction_id, v.event_id, v.item_instance_id,
                       v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key,
                       r.loot_table_revision_ref, r.loot_purpose_key, r.draw_ordinal,
                       r.intent_binding, r.transaction_id, r.event_id, r.item_instance_id,
                       r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)
           -- The parent must be a LIVE corpse of the SAME death (§39.4): its
           -- own CORPSE_MATERIALIZATION receipt, same full death tuple, still
           -- live on Ground (never yet DECAY_RETIREd).
           AND EXISTS (
               SELECT 1 FROM game_item_mint_receipts cr
                 JOIN game_item_ground_locations cg ON cg.item_instance_id = cr.item_instance_id
                 JOIN game_item_instances ci ON ci.item_instance_id = cr.item_instance_id
                WHERE cr.item_instance_id = e.parent_item_instance_id
                  AND cr.loot_purpose_key = 'CORPSE_MATERIALIZATION'
                  AND cr.death_world_id = r.death_world_id
                  AND cr.death_channel_id = r.death_channel_id
                  AND cr.death_scope_ownership_generation = r.death_scope_ownership_generation
                  AND cr.death_actor_local_id = r.death_actor_local_id
                  AND cr.death_actor_local_generation = r.death_actor_local_generation
                  AND ci.lifecycle = 1)) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_fee_burns f
          JOIN game_item_container_entries e ON e.item_instance_id = NEW.item_instance_id
         WHERE f.transaction_id = NEW.minted_transaction_id
           AND f.created_xact_id = pg_current_xact_id()
           AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                        f.change_gold_item_instance_id)
           AND e.placed_transaction_id = f.transaction_id) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
          JOIN game_item_container_entries e ON e.item_instance_id = NEW.item_instance_id
         WHERE l.transaction_id = NEW.minted_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.direction = 2
           AND l.created_xact_id = pg_current_xact_id()
           AND o.created_xact_id = pg_current_xact_id()
           AND e.placed_transaction_id = o.transaction_id) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;

-- 0031's guard, same body, plus: or the new entry of an output coin line of a bank operation of
-- the same physical transaction, in that operation's backpack, for a live item. The
-- container-slot branch and the concurrency-safe GAMEITEM01-CONTAINER-ENTRIES-MAX count are
-- unchanged.
CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_TABLE_NAME = 'game_item_container_slots' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape = 1
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER receipt'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_item_container_entries' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape IN (2,4)
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_reward_claim_mint_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.item_instance_id = NEW.item_instance_id
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_item_fee_burns f
             WHERE f.transaction_id = NEW.placed_transaction_id
               AND f.character_id = NEW.character_id
               AND f.backpack_item_instance_id = NEW.parent_item_instance_id
               AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                            f.change_gold_item_instance_id)
               AND f.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_account_bank_coin_lines l
              JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
             WHERE o.transaction_id = NEW.placed_transaction_id
               AND o.acting_character_id = NEW.character_id
               AND o.backpack_item_instance_id = NEW.parent_item_instance_id
               AND l.item_instance_id = NEW.item_instance_id
               AND l.direction = 2
               AND l.placement_ordinal = NEW.placement_ordinal
               AND l.created_xact_id = pg_current_xact_id()
               AND o.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER, reward-claim MINT or fee change receipt'
                USING ERRCODE = '23514';
        END IF;
        PERFORM 1 FROM game_item_container_slots
         WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
        IF (SELECT count(*) FROM game_item_container_entries e
             WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
            RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_account_bank_operation_consistent
    AFTER INSERT ON game_account_bank_operations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_operation_consistency_guard();
CREATE CONSTRAINT TRIGGER game_account_bank_entry_consistent
    AFTER INSERT ON game_account_bank_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_entry_consistency_guard();
CREATE CONSTRAINT TRIGGER game_account_bank_balance_consistent
    AFTER INSERT OR UPDATE ON game_account_bank_balances
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_balance_consistency_guard();
CREATE CONSTRAINT TRIGGER game_account_bank_coin_line_operation_proven
    AFTER INSERT ON game_account_bank_coin_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_coin_line_operation_proven();
CREATE CONSTRAINT TRIGGER game_account_bank_event_operation_proven
    AFTER INSERT ON game_account_bank_audit_outbox
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_event_operation_proven();

CREATE TRIGGER game_account_bank_operation_immutable BEFORE UPDATE OR DELETE
    ON game_account_bank_operations FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_account_bank_entry_immutable BEFORE UPDATE OR DELETE
    ON game_account_bank_entries FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_account_bank_coin_line_immutable BEFORE UPDATE OR DELETE
    ON game_account_bank_coin_lines FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_account_bank_balance_undeletable BEFORE DELETE
    ON game_account_bank_balances FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_account_bank_audit_guard BEFORE UPDATE OR DELETE
    ON game_account_bank_audit_outbox FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_audit_guard();
CREATE TRIGGER game_account_bank_audit_hold_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_account_bank_audit_legal_holds FOR EACH ROW
    EXECUTE FUNCTION game_account_bank_audit_hold_guard();
CREATE TRIGGER game_account_bank_audit_legal_holds_no_truncate BEFORE TRUNCATE
    ON game_account_bank_audit_legal_holds FOR EACH STATEMENT
    EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_account_bank_operations_no_truncate BEFORE TRUNCATE
    ON game_account_bank_operations FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_account_bank_entries_no_truncate BEFORE TRUNCATE
    ON game_account_bank_entries FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_account_bank_balances_no_truncate BEFORE TRUNCATE
    ON game_account_bank_balances FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_account_bank_coin_lines_no_truncate BEFORE TRUNCATE
    ON game_account_bank_coin_lines FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_account_bank_audit_outbox_no_truncate BEFORE TRUNCATE
    ON game_account_bank_audit_outbox FOR EACH STATEMENT
    EXECUTE FUNCTION game_item_reject_truncate();
-- Stamped, never caller supplied.
CREATE TRIGGER game_account_bank_operations_stamp_xact BEFORE INSERT
    ON game_account_bank_operations FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_account_bank_entries_stamp_xact BEFORE INSERT
    ON game_account_bank_entries FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_account_bank_coin_lines_stamp_xact BEFORE INSERT
    ON game_account_bank_coin_lines FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_account_bank_audit_outbox_stamp_xact BEFORE INSERT
    ON game_account_bank_audit_outbox FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_account_bank_operation_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_entry_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_balance_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_coin_line_operation_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_event_operation_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_audit_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_immutable() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_audit_hold_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_place_legal_hold(uuid, text, text) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_release_legal_hold(uuid, text) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_account_bank_expire_audit(integer) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_container_entry_removal_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON
    game_account_bank_operations,
    game_account_bank_entries,
    game_account_bank_balances,
    game_account_bank_coin_lines,
    game_account_bank_audit_outbox,
    game_account_bank_audit_legal_holds
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_account_bank_operation_consistency_guard(),
    game_account_bank_entry_consistency_guard(),
    game_account_bank_balance_consistency_guard(),
    game_account_bank_coin_line_operation_proven(),
    game_account_bank_event_operation_proven(),
    game_account_bank_audit_guard(),
    game_account_bank_immutable(),
    game_account_bank_audit_hold_guard(),
    game_account_bank_place_legal_hold(uuid, text, text),
    game_account_bank_release_legal_hold(uuid, text),
    game_account_bank_expire_audit(integer)
FROM PUBLIC;
-- BANK-0 §3: never DELETE authoritative bank state. The runtime role inserts operations, entries, lines and events and
-- upserts and updates balances; the item grants it already holds (0010, 0011, 0023) are
-- admitted for a bank coin line only by the proofs above.
GRANT SELECT, INSERT ON
    game_account_bank_operations,
    game_account_bank_entries,
    game_account_bank_coin_lines,
    game_account_bank_audit_outbox
TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_account_bank_balances TO oteryn_game_runtime;
-- Retention deletes only expired, unheld audit rows, through the bounded expiry function.
GRANT EXECUTE ON FUNCTION game_account_bank_expire_audit(integer) TO oteryn_game_runtime;
GRANT SELECT ON
    game_account_bank_operations,
    game_account_bank_entries,
    game_account_bank_balances,
    game_account_bank_coin_lines,
    game_account_bank_audit_outbox,
    game_account_bank_audit_legal_holds
TO oteryn_game_control;
