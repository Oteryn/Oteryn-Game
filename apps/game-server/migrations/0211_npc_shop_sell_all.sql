-- NPC-TRADE-SELLALL-1: an NPC sell-all is one SELL trade (command 97 NPC_SHOP, action SELL_ALL)
-- of every carried live default stack of the offer item. It extends the NPC-TRADE-1 cause record
-- (0082/0114): no new table, gold owner or audit stream.
--
--   * `game_npc_trades.sell_all` marks it; only a SELL may carry it. A sell-all sells 1..2,000
--     units (SELLALL1-RL-02) in 1..20 whole item BURN lines and up to 3 coin MINTs (23 lines,
--     DUR03-RL-01/-02-NPC-SELL-ALL); every other trade keeps 1..100 units and its line bounds.
--   * The consistency guard re-proves at commit that the BURN lines are whole, of the trade's
--     item revision, in display order, sum to its units, and that no live default stack of the
--     item stays in the backpack. Every other check of 0114 is unchanged.
--
-- Rollback is a new migration that re-issues the 0114 guard and constraints, and only
-- while no sell-all row exists; after that the rows are retained evidence of burned and minted
-- value, and new sell-alls stop by withdrawing capability 81.

ALTER TABLE game_npc_trades
    ADD COLUMN sell_all BOOLEAN NOT NULL DEFAULT false,
    DROP CONSTRAINT game_npc_trades_quantity_check,
    DROP CONSTRAINT game_npc_trades_item_units_check,
    DROP CONSTRAINT game_npc_trades_conservation,
    ADD CONSTRAINT game_npc_trades_quantity_check CHECK (quantity BETWEEN 1 AND 2000),
    ADD CONSTRAINT game_npc_trades_item_units_check CHECK (item_units BETWEEN 1 AND 2000),
    ADD CONSTRAINT game_npc_trades_sell_all_shape CHECK (
        CASE WHEN sell_all THEN side = 2 AND quantity = item_units
             ELSE quantity <= 100 AND item_units <= 100 END),
    ADD CONSTRAINT game_npc_trades_conservation CHECK (CASE side
        WHEN 1 THEN burned_gold_units - minted_gold_units + bank_debit_gold_units = total_gold_units
            AND minted_gold_units <= 9999 AND line_count <= 23
        ELSE burned_gold_units = 0 AND minted_gold_units = total_gold_units
            AND total_gold_units <= 1009999 AND bank_debit_gold_units = 0
            AND line_count <= CASE WHEN sell_all THEN 23 ELSE 4 END END);

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
    sold_units BIGINT;
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
           count(*) FILTER (WHERE l.line_kind IN (2, 4)),
           coalesce(sum(l.quantity_before - l.quantity_after) FILTER (WHERE l.line_kind = 3), 0)
      INTO lines, last_line, burned, minted, item_lines, item_kind_lines, first_mint, mints,
           sold_units
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
       OR (NOT NEW.sell_all AND (item_lines <> 1 OR item_kind_lines <> 1))
       -- NPC-TRADE-SELLALL-1: 1..20 whole item BURNs whose units are the trade's, the first of
       -- them the trade's item, in display order (ordinals descend).
       OR (NEW.sell_all AND (
              item_kind_lines NOT BETWEEN 1 AND 20
           OR sold_units <> NEW.item_units
           OR NOT EXISTS (SELECT 1 FROM game_npc_trade_lines l
                           WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = 1
                             AND l.line_kind = 3 AND l.item_instance_id = NEW.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_npc_trade_lines l
                       WHERE l.transaction_id = NEW.transaction_id AND l.line_kind = 3
                         AND l.quantity_after <> 0)
           OR EXISTS (SELECT 1 FROM game_npc_trade_lines p JOIN game_npc_trade_lines l
                       USING (transaction_id)
                       WHERE l.transaction_id = NEW.transaction_id
                         AND p.line_kind = 3 AND l.line_kind = 3
                         AND p.line_ordinal < l.line_ordinal
                         AND p.placement_ordinal <= l.placement_ordinal)))
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
                  AND l.item_instance_id = e.item_instance_id)))
       -- A sell-all leaves no live default stack of the item in this backpack.
       OR (NEW.sell_all AND EXISTS (
        SELECT 1 FROM game_item_container_entries e
          JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
         WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
           AND i.lifecycle = 1
           AND (i.definition_family, i.definition_production_key, i.definition_revision_ref)
               = (NEW.item_definition_family, NEW.item_definition_production_key,
                  NEW.item_definition_revision_ref)
           AND NOT EXISTS (SELECT 1 FROM game_item_container_entries c
                            WHERE c.parent_item_instance_id = i.item_instance_id)
           AND NOT EXISTS (SELECT 1 FROM game_item_timed_states t
                            WHERE t.item_instance_id = i.item_instance_id)
           AND NOT EXISTS (SELECT 1 FROM game_item_imbuements b
                            WHERE b.item_instance_id = i.item_instance_id))) THEN
        RAISE EXCEPTION 'NPC trade lines must be the planned BURN and MINT lines with their exact item state'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
