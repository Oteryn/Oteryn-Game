-- NPC-TRADE-1: NPC BUY and SELL as item-only transactions (NPC-0 §5,
-- `reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md`; boundary gate
-- `NPC_SINGLE_TRADE_COMMIT_V1`; builds on the coin fee plan of 0023/0031).
--
-- A trade commits in one physical transaction without a CharacterRevision advance (§5.2,
-- composition §3 rule 1): no Character root, progression or receipt row is written. Scope:
--   * `game_npc_trades`: the DUR-03 cause record, one per (occurrence, Character) forever and one
--     per CommandRef. It binds `NpcTradeCause {npc, offer, side, occurrence}`, quantity, unit
--     price, the traded item, the backpack, the audit event and the TransactionId;
--   * `game_npc_trade_lines`: every line of the trade in order. BUY: the coin BURN lines of the
--     fee plan (`FeeBurnCause::NpcTrade`, worth ascending then display order), the change MINT
--     (platinum, then gold) and the bought item's MINT, each MINT in a new direct entry after the
--     highest ordinal the backpack held. SELL: one BURN of the sold item (part of a stack, or the
--     whole item) and the coin MINT (crystal, platinum, gold, each at most 100) in new entries;
--   * `game_npc_trade_consistency_guard`: the record commits only with its lines, the exact item
--     changes, entries and the one audit event of the same physical transaction;
--   * the shared item guards keep their latest bodies verbatim (0075, 0077, 0023) and add one
--     admitting arm each for a line of a trade record of the same physical transaction: the item
--     change, the whole-burn entry removal (§5.5 widens the 0023 proof), the MINT entry
--     placement, the MINT itself and the audit envelope above the one-item 9,216 B;
--   * a traded item is never a coin, so its MINT keeps the 0031 (minted TransactionId,
--     definition key) uniqueness with the change coins.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any trade
-- record exists it drops the two tables, the guard and its triggers, and re-issues the five
-- shared functions without their trade arm. After trade records exist they and their lines are
-- retained evidence of burned and minted value: a rollback keeps them and may only stop new
-- writes by revoking the INSERT grant.

CREATE TABLE game_npc_trades (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    -- One occurrence per command 8, bound 1:1 to its CommandRef (§5.1).
    occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(occurrence_id)),
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    -- SHA-256 of the semantic request; a changed binding for the same occurrence conflicts.
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    -- The NPC (its catalogue definition key) and the offer at the bound catalogue revision.
    npc_key TEXT NOT NULL CHECK (octet_length(npc_key) BETWEEN 1 AND 512),
    catalogue_revision NUMERIC(20,0) NOT NULL
        CHECK (catalogue_revision BETWEEN 0 AND 18446744073709551615),
    offer_index INTEGER NOT NULL CHECK (offer_index >= 0),
    -- 1 = BUY, 2 = SELL.
    side SMALLINT NOT NULL CHECK (side IN (1, 2)),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    unit_price BIGINT NOT NULL CHECK (unit_price BETWEEN 0 AND 20000000),
    -- unit_price x quantity: at most what 20 coin stacks hold (BUY), or three (SELL). A price of
    -- 0 burns no coin (BUY) or mints none (SELL): NPC-0 §5.3 and §5.4.
    total_gold_units BIGINT NOT NULL CHECK (total_gold_units BETWEEN 0 AND 20000000),
    item_definition_family TEXT NOT NULL
        CHECK (octet_length(item_definition_family) BETWEEN 1 AND 128),
    item_definition_production_key TEXT NOT NULL
        CHECK (octet_length(item_definition_production_key) BETWEEN 1 AND 512),
    item_definition_revision_ref TEXT NOT NULL
        CHECK (octet_length(item_definition_revision_ref) BETWEEN 1 AND 512),
    -- The bought (minted) or sold (burned) item and its units.
    item_instance_id UUID NOT NULL CHECK (game_character_is_uuid_v7(item_instance_id)),
    item_units BIGINT NOT NULL CHECK (item_units BETWEEN 1 AND 100),
    backpack_item_instance_id UUID NOT NULL,
    -- Coins burned (BUY) and minted (BUY change, SELL proceeds), in gold units.
    burned_gold_units BIGINT NOT NULL CHECK (burned_gold_units BETWEEN 0 AND 20000000),
    minted_gold_units BIGINT NOT NULL CHECK (minted_gold_units BETWEEN 0 AND 1009999),
    line_count SMALLINT NOT NULL CHECK (line_count BETWEEN 1 AND 23),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (total_gold_units = unit_price * quantity),
    CHECK (game_item_coin_worth(item_definition_production_key) IS NULL),
    CHECK (CASE side
             WHEN 1 THEN burned_gold_units - minted_gold_units = total_gold_units
                         AND minted_gold_units <= 9999 AND line_count <= 23
             ELSE burned_gold_units = 0 AND minted_gold_units = total_gold_units
                  AND total_gold_units <= 1009999 AND line_count <= 4
           END),
    UNIQUE (occurrence_id, character_id),
    UNIQUE (game_session_id, command_id)
);

CREATE TABLE game_npc_trade_lines (
    transaction_id UUID NOT NULL REFERENCES game_npc_trades (transaction_id),
    line_ordinal SMALLINT NOT NULL CHECK (line_ordinal BETWEEN 1 AND 23),
    -- 1 = coin BURN, 2 = coin MINT, 3 = item BURN, 4 = item MINT.
    line_kind SMALLINT NOT NULL CHECK (line_kind BETWEEN 1 AND 4),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    -- The backpack entry before a BURN (and after a partial one), or the new entry of a MINT.
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    coin_worth BIGINT NULL CHECK (coin_worth IN (1, 100, 10000)),
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 0 AND 100),
    quantity_after BIGINT NOT NULL CHECK (quantity_after BETWEEN 0 AND 100),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK ((coin_worth IS NOT NULL) = (line_kind IN (1, 2))),
    CHECK (CASE WHEN line_kind IN (1, 3)
                THEN quantity_before >= 1 AND quantity_after < quantity_before
                ELSE quantity_before = 0 AND quantity_after >= 1 END),
    PRIMARY KEY (transaction_id, line_ordinal),
    UNIQUE (transaction_id, item_instance_id)
);

-- Commit-time atomicity of one trade. SECURITY INVOKER: the runtime role reads every relation
-- here already.
CREATE FUNCTION game_npc_trade_consistency_guard() RETURNS trigger
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
       OR (NEW.side = 1 AND CASE WHEN last_worth IS NULL THEN NEW.total_gold_units <> 0
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

-- A trade line belongs to a trade record of this same physical transaction.
CREATE FUNCTION game_npc_trade_line_record_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_npc_trades t
         WHERE t.transaction_id = NEW.transaction_id
           AND t.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'NPC trade line must commit with its trade record'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0023's envelope guard: a trade event of this transaction is admitted up to the trade shape's
-- registered envelope row (27,408 B, `DUR03-RL-07-NPC-TRADE-ENVELOPE-BYTES`); a fee event keeps
-- its own row (25,712 B, formerly the column bound) and every other event the one-item row.
CREATE OR REPLACE FUNCTION game_item_audit_envelope_size_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF octet_length(NEW.envelope) <= 9216 THEN
        RETURN NULL;
    END IF;
    IF octet_length(NEW.envelope) <= 27408 AND EXISTS (
        SELECT 1 FROM game_npc_trades t
         WHERE t.event_id = NEW.event_id
           AND t.transaction_id = NEW.transaction_id
           AND t.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF octet_length(NEW.envelope) <= 25712 AND EXISTS (
        SELECT 1 FROM game_item_fee_burns f
         WHERE f.event_id = NEW.event_id
           AND f.transaction_id = NEW.transaction_id
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'DUR-03 audit envelope exceeds its registered shape ceiling'
        USING ERRCODE = '23514';
END;
$$;

-- 0075's item change proof, plus the trade BURN arm first. Every existing branch is unchanged.
CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- 0082: the BURN line of an NPC trade of this same physical transaction.
    IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
                                AND t.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       WHERE r.operation=1 AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.last_transaction_id AND r.state_revision_before=OLD.state_revision
         AND NEW.state_revision=OLD.state_revision+1 AND NEW.quantity=OLD.quantity
         AND NEW.lifecycle=OLD.lifecycle AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=NEW.item_instance_id AND e.transaction_id=NEW.last_transaction_id
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind<>1
         AND l.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
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

-- 0075's entry removal proof, plus the trade whole-BURN arm first. Every existing branch is
-- unchanged.
CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- 0082: the whole BURN of an NPC trade line of this same physical transaction (NPC-0 §5.5).
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND t.character_id = OLD.character_id
           AND t.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND i.lifecycle = 2
           AND l.created_xact_id = pg_current_xact_id()
           AND t.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND (l.operation_kind=6 OR r.character_id=OLD.character_id)
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.to_slot IS NOT NULL AND r.item_instance_id=OLD.item_instance_id
         AND r.character_id=OLD.character_id AND r.backpack_item_instance_id=OLD.parent_item_instance_id
         AND r.backpack_ordinal=OLD.placement_ordinal AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
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

-- 0075's placement proof, plus the trade MINT entry arm first. Every existing branch is unchanged.
CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- 0082: the new entry of an NPC trade MINT line of this same physical transaction. NEW has
    -- entry fields only on the entries table, so the table test is its own IF.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(
       SELECT 1 FROM game_npc_trade_lines l
         JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
         JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
        WHERE l.transaction_id = NEW.placed_transaction_id
          AND l.item_instance_id = NEW.item_instance_id
          AND l.line_kind IN (2, 4)
          AND l.placement_ordinal = NEW.placement_ordinal
          AND t.character_id = NEW.character_id
          AND t.backpack_item_instance_id = NEW.parent_item_instance_id
          AND i.lifecycle = 1
          AND l.created_xact_id = pg_current_xact_id()
          AND t.created_xact_id = pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    END IF;

    IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(
       SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.from_slot IS NOT NULL AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
         AND r.backpack_item_instance_id=NEW.parent_item_instance_id AND r.backpack_ordinal=NEW.placement_ordinal
         AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
         AND i.lifecycle=1 AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'equipment backpack entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.operation_kind=1 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
        AND l.destination_parent_item_instance_id=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
        AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514'; END IF;
       RETURN NULL;
    END IF;
    END IF;
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

-- 0077's MINT consistency guard, plus the trade MINT arm first. Every existing branch is
-- unchanged.
CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- 0082: an NPC trade MINT line of this same physical transaction, placed in its new entry.
    IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
         JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
         JOIN game_item_container_entries e ON e.item_instance_id = l.item_instance_id
        WHERE l.item_instance_id = NEW.item_instance_id
          AND l.transaction_id = NEW.minted_transaction_id
          AND l.line_kind IN (2, 4)
          AND e.placed_transaction_id = l.transaction_id
          AND l.created_xact_id = pg_current_xact_id()
          AND t.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_native_map_item_receipts r JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id)
      WHERE r.item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.minted_transaction_id
      AND r.world_id=NEW.world_id AND r.definition_family=NEW.definition_family AND r.definition_key=NEW.definition_production_key
      AND r.definition_revision=NEW.definition_revision_ref AND r.quantity=NEW.quantity
      AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND g.runtime_scope_ownership_generation=r.scope_generation
      AND g.spatial_position=r.spatial_position AND g.map_revision=r.map_revision AND g.content_revision=r.content_revision
      AND g.native_room_placement_context=r.placement_context AND a.event_id=r.event_id AND a.transaction_id=r.transaction_id
      AND a.envelope=r.source_intent AND a.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
         AND l.transaction_id=NEW.minted_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
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
    -- MAP-OVERLAY-1b: a map-item MINT with its Ground line, receipt and audit
    -- event in this transaction, exactly as reserved.
    IF EXISTS (
        SELECT 1
          FROM game_map_item_mint_receipts r
          JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
          JOIN game_map_item_mint_reservations v
            ON (v.game_session_id, v.command_id) = (r.game_session_id, r.command_id)
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.world_id = NEW.world_id
           AND r.created_xact_id = pg_current_xact_id()
           AND (r.definition_family, r.definition_production_key, r.definition_revision_ref)
               = (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref)
           AND r.quantity = NEW.quantity
           AND g.world_id = r.world_id
           AND g.channel_id = r.channel_id
           AND g.runtime_scope_ownership_generation = r.runtime_scope_ownership_generation
           AND g.spatial_position = r.spatial_position
           AND g.map_revision = r.map_revision
           AND g.native_room_placement_context = r.base_bundle_digest
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.created_xact_id = pg_current_xact_id()
           AND sha256(a.envelope) = r.envelope_sha256
           AND (v.character_id, v.world_id, v.channel_id, v.base_bundle_digest, v.placement_key,
                v.reset_epoch, v.intent_binding, v.transaction_id, v.event_id,
                v.item_instance_id, v.occurred_at)
               = (r.character_id, r.world_id, r.channel_id, r.base_bundle_digest, r.placement_key,
                  r.reset_epoch, r.intent_binding, r.transaction_id, r.event_id,
                  r.item_instance_id, r.occurred_at)) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;
-- The one-item and fee envelope ceilings stay enforced by the size proof above; the column bound
-- is the NPC BUY shape's registered envelope row (`DUR03-RL` NPC trade rows), the largest shape.
ALTER TABLE game_item_audit_outbox DROP CONSTRAINT game_item_audit_outbox_envelope_check;
ALTER TABLE game_item_audit_outbox
    ADD CONSTRAINT game_item_audit_outbox_envelope_check
        CHECK (octet_length(envelope) BETWEEN 1 AND 27408);

CREATE CONSTRAINT TRIGGER game_npc_trade_consistent
    AFTER INSERT ON game_npc_trades
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_npc_trade_consistency_guard();
CREATE CONSTRAINT TRIGGER game_npc_trade_line_record_proven
    AFTER INSERT ON game_npc_trade_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_npc_trade_line_record_proven();

CREATE TRIGGER game_npc_trade_immutable BEFORE UPDATE OR DELETE
    ON game_npc_trades FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_npc_trade_line_immutable BEFORE UPDATE OR DELETE
    ON game_npc_trade_lines FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_npc_trades_no_truncate BEFORE TRUNCATE ON game_npc_trades
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_npc_trade_lines_no_truncate BEFORE TRUNCATE ON game_npc_trade_lines
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_npc_trades_stamp_xact BEFORE INSERT
    ON game_npc_trades FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_npc_trade_lines_stamp_xact BEFORE INSERT
    ON game_npc_trade_lines FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

-- CREATE OR REPLACE clears function-level settings; restore the search_path pins.
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_npc_trade_consistency_guard','game_npc_trade_line_record_proven',
                          'game_item_audit_envelope_size_proven','game_item_instance_change_proven',
                          'game_item_container_entry_removal_proven','game_item_placement_proven',
                          'game_item_mint_consistency_guard'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',f);
 END LOOP;
END $$;

REVOKE ALL ON game_npc_trades, game_npc_trade_lines FROM PUBLIC;
-- Least privilege: the writer inserts the record and its lines; the item INSERT and UPDATE,
-- entry INSERT and DELETE and audit INSERT grants it already holds are admitted for a trade only
-- by the line proofs above.
GRANT SELECT, INSERT ON game_npc_trades, game_npc_trade_lines TO oteryn_game_runtime;
GRANT SELECT ON game_npc_trades, game_npc_trade_lines TO oteryn_game_control;
