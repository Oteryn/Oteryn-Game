-- GOLD-FEE-1b: platinum and crystal fee inputs and the change MINT (decision
-- `CHARACTER-GOLD-FEE-BOUNDARY-V1`, `reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md`
-- §4.2-§4.6, owner decisions D175-D178; builds on 0023).
--
-- A fee now burns gold, platinum and crystal stacks of the equipped main backpack's direct
-- entries by worth ascending, then display order, and mints the change back in the same physical
-- transaction: `change / 100` platinum coins, then `change % 100` gold coins, each one fresh
-- ItemInstance in a new direct backpack entry placed after the burn lines (the highest ordinal
-- the backpack held before the burn, plus one, then the next). Scope of this migration only:
--   * the fee record gains the change: its worth (0..9,999), the two output identity slots fixed
--     with the TransactionId (platinum, gold; an unused slot never exists as an item) and the
--     first change entry's ordinal. A 0023 record keeps change 0 and no slots;
--   * a line's coin worth is 1, 100 or 10,000 and must match its coin (`game_item_coin_worth`);
--   * `game_item_fee_burn_consistency_guard` is replaced: every 0023 arm is kept, the line order
--     and the untouched-stack check are generalised to worth ascending then display order, the
--     change must be below the last line's coin worth (no unit is burned beyond the plan), and
--     the change MINT must be exactly the planned outputs with their entries;
--   * `game_item_mint_consistency_guard` (0013 body) and `game_item_placement_proven` (0012 body)
--     keep their bodies verbatim and add one admitting clause each: a change output of a fee
--     record of the same physical transaction;
--   * the two change outputs share the fee TransactionId, so `game_item_instances` is unique per
--     (minted TransactionId, definition key) instead of per minted TransactionId, and backpack
--     entries no longer carry a unique placed TransactionId. Every other MINT and placement
--     path is still bound one-to-one to its own receipt by the guards;
--   * the stored fee envelope ceiling is the re-measured `DUR03-RL-07-FEE-BURN-ENVELOPE-BYTES`.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any fee
-- record with change exists it restores the 0023 guard body, the 0013/0012 function bodies, the
-- 24,495 B envelope CHECK, the gold-only line worth, change 0 and both unique constraints, and
-- drops the three columns and `game_item_coin_worth`. After such records exist they, their
-- lines and their minted change are retained evidence: a rollback keeps them and may only stop
-- new writes by refusing change in the writer.

-- The closed worth table (decision §4.2) in gold units; NULL for any other key.
CREATE FUNCTION game_item_coin_worth(production_key TEXT) RETURNS BIGINT
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE production_key
        WHEN 'oteryn:item.tibia.i3031' THEN 1
        WHEN 'oteryn:item.tibia.i3035' THEN 100
        WHEN 'oteryn:item.tibia.i3043' THEN 10000
    END::bigint
$$;

ALTER TABLE game_item_instances DROP CONSTRAINT game_item_instances_minted_transaction_id_key;
ALTER TABLE game_item_instances
    ADD CONSTRAINT game_item_instances_minted_transaction_key
        UNIQUE (minted_transaction_id, definition_production_key);
ALTER TABLE game_item_container_entries
    DROP CONSTRAINT game_item_container_entries_placed_transaction_id_key;

ALTER TABLE game_item_fee_burn_lines DROP CONSTRAINT game_item_fee_burn_lines_coin_worth_check;
ALTER TABLE game_item_fee_burn_lines
    ADD CONSTRAINT game_item_fee_burn_lines_coin_worth_check
        CHECK (coin_worth IN (1, 100, 10000));

ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_change_gold_units_check;
ALTER TABLE game_item_fee_burns
    ADD CONSTRAINT game_item_fee_burns_change_gold_units_check
        CHECK (change_gold_units BETWEEN 0 AND 9999),
    ADD COLUMN change_platinum_item_instance_id UUID NULL
        CHECK (game_character_is_uuid_v7(change_platinum_item_instance_id)),
    ADD COLUMN change_gold_item_instance_id UUID NULL
        CHECK (game_character_is_uuid_v7(change_gold_item_instance_id)),
    -- The first change entry; the second, when both outputs exist, is the next ordinal.
    ADD COLUMN change_placement_ordinal NUMERIC(20,0) NULL
        CHECK (change_placement_ordinal BETWEEN 1 AND 18446744073709551615),
    ADD CONSTRAINT game_item_fee_burns_change_slots CHECK (
        (change_platinum_item_instance_id IS NULL) = (change_gold_item_instance_id IS NULL)
        AND change_platinum_item_instance_id <> change_gold_item_instance_id
        AND (change_gold_units = 0 OR change_platinum_item_instance_id IS NOT NULL)
        AND (change_gold_units = 0) = (change_placement_ordinal IS NULL));

-- 0023's guard, with platinum and crystal inputs and the change MINT.
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
    outputs INTEGER := (NEW.change_gold_units / 100 > 0)::integer
                     + (NEW.change_gold_units % 100 > 0)::integer;
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
    SELECT l.placement_ordinal, l.coin_worth INTO last_ordinal, last_worth
      FROM game_item_fee_burn_lines l
     WHERE l.transaction_id = NEW.transaction_id AND l.line_ordinal = NEW.line_count;
    IF slot_item IS DISTINCT FROM NEW.backpack_item_instance_id
       OR lines <> NEW.line_count
       OR last_line IS DISTINCT FROM NEW.line_count
       OR burned <> NEW.burned_gold_units
       -- The plan burns no unit beyond the fee: the change is below the last coin's worth.
       OR NEW.change_gold_units >= last_worth
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
               SELECT 1 FROM game_item_fee_burn_lines l
                WHERE l.transaction_id = NEW.transaction_id
                  AND l.item_instance_id = e.item_instance_id)) THEN
        RAISE EXCEPTION 'fee BURN lines must be the coin stacks of the backpack in plan order with their exact item state'
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

-- 0013's guard, same body, plus: or a change output of a fee record of the same physical
-- transaction in its new entry (the fee record's guard proves the rest).
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
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;

-- 0012's guard, same body, plus: or the new entry of a change output of a fee record of the
-- same physical transaction, in that record's backpack, for a live item. The container-slot
-- branch and the concurrency-safe GAMEITEM01-CONTAINER-ENTRIES-MAX count are unchanged.
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

-- The stored fee envelope ceiling: `DUR03-RL-07-FEE-BURN-ENVELOPE-BYTES`, re-measured with the
-- change MINT lines. The one-item 9,216 B stays enforced by 0023's trigger.
ALTER TABLE game_item_audit_outbox DROP CONSTRAINT game_item_audit_outbox_envelope_check;
ALTER TABLE game_item_audit_outbox
    ADD CONSTRAINT game_item_audit_outbox_envelope_check
        CHECK (octet_length(envelope) BETWEEN 1 AND 25712);

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_coin_worth(TEXT) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_fee_burn_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

-- The runtime role's grants are unchanged: it already inserts items (0010), backpack entries
-- (0011) and fee records (0023). `game_item_coin_worth` is pure and, like
-- `game_character_is_uuid_v7`, keeps its default EXECUTE: the SECURITY INVOKER guard calls it
-- as the runtime role.
