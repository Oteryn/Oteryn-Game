-- BAGS-USE-1 (migration 0131): BAGS-0 §4.2/§8, ITEM-USE-0 §4/§11.
-- A source entry may be below the fenced Character's main backpack, at depth 1..8.
-- The flask still returns only to that Character's main backpack. Equipment, foreign,
-- Ground/corpse trees and deeper paths remain refused. All older proof arms are retained.
-- Latest bodies: consistency 0083; removal 0095 (trigger recreated by 0128);
-- nested proof 0128; placement 0113 (trigger recreated by 0128).
-- game_item_nested_entry_exclusive is a trigger, not a function: its 0128
-- game_item_location_exclusivity_guard already counts nested custody and retired contents.
-- Keep that exact exclusivity rule and trigger unchanged.
-- Rollback is a new migration; keep existing item-use records and audit evidence.

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_runtime') THEN
        RAISE EXCEPTION 'oteryn_game_runtime role is required';
    END IF;
END $$;

CREATE OR REPLACE FUNCTION game_item_use_consistency_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    src game_item_instances%ROWTYPE;
    fl game_item_instances%ROWTYPE;
    slot_item UUID;
BEGIN
    SELECT * INTO src FROM game_item_instances WHERE item_instance_id = NEW.source_item_instance_id;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    IF src.item_instance_id IS NULL
       -- The source parent is the main backpack or a container below it, with room
       -- for the source edge within BAGS0-RL-01. Equipment and foreign trees stay refused.
       OR slot_item IS NULL
       OR NOT EXISTS (SELECT 1 FROM game_item_loop_owner(NEW.source_parent_item_instance_id) o
                       WHERE o.owner_character = NEW.character_id AND o.depth BETWEEN 0 AND 7)
       OR src.world_id <> NEW.world_id
       OR (src.definition_family, src.definition_production_key, src.definition_revision_ref)
          <> (NEW.source_definition_family, NEW.source_definition_production_key,
              NEW.source_definition_revision_ref)
       -- Exactly one unit left it, in this transaction.
       OR src.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR src.quantity <> NEW.source_quantity_after
       OR src.lifecycle <> (CASE WHEN NEW.source_quantity_after = 0 THEN 2 ELSE 1 END)
       -- The real before-quantity, captured by the item guard.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.source_item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.source_quantity_before)
       -- A spent stack leaves no location (its entry removal is proven by the removal trigger);
       -- a stack with units left keeps its entry.
       OR (NEW.source_quantity_after = 0 AND (
               EXISTS (SELECT 1 FROM game_item_container_entries e
                        WHERE e.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries ne
                        WHERE ne.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_container_slots s
                        WHERE s.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_ground_locations g
                        WHERE g.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                        WHERE ce.item_instance_id = NEW.source_item_instance_id)))
       OR (NEW.source_quantity_after > 0 AND NOT EXISTS (
               SELECT 1 FROM (
                   SELECT item_instance_id, parent_item_instance_id, placement_ordinal
                     FROM game_item_container_entries WHERE character_id = NEW.character_id
                   UNION ALL
                   SELECT item_instance_id, parent_item_instance_id, placement_ordinal
                     FROM game_item_nested_entries
               ) e
                WHERE e.item_instance_id = NEW.source_item_instance_id
                  AND e.parent_item_instance_id = NEW.source_parent_item_instance_id
                  AND e.placement_ordinal = NEW.source_placement_ordinal))
       -- The Character is in this World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = NEW.world_id)
       -- Its one audit event commits in this same physical transaction.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.source_item_instance_id
              AND a.event_type_id = 2
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'item use must commit with its used stack, its audit event and its custody together'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.flask_item_instance_id IS NOT NULL THEN
        SELECT * INTO fl FROM game_item_instances WHERE item_instance_id = NEW.flask_item_instance_id;
        IF fl.item_instance_id IS NULL
           OR fl.world_id <> NEW.world_id
           OR fl.lifecycle <> 1
           OR fl.quantity <> NEW.flask_quantity_after
           OR (fl.definition_family, fl.definition_production_key, fl.definition_revision_ref)
              <> (NEW.flask_definition_family, NEW.flask_definition_production_key,
                  NEW.flask_definition_revision_ref)
           -- The flask lies in the same backpack, at its recorded entry.
           OR NOT EXISTS (
               SELECT 1 FROM game_item_container_entries e
                WHERE e.item_instance_id = NEW.flask_item_instance_id
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = slot_item
                  AND e.placement_ordinal = NEW.flask_placement_ordinal)
           OR (NEW.shape = 3 AND (
                   fl.last_transaction_id IS DISTINCT FROM NEW.transaction_id
                OR NOT EXISTS (
                    SELECT 1 FROM game_item_transfer_quantity_evidence ev
                     WHERE ev.item_instance_id = NEW.flask_item_instance_id
                       AND ev.transaction_id = NEW.transaction_id
                       AND ev.quantity_before = NEW.flask_quantity_before)))
           -- A fresh flask item is created by this transaction, with its entry placed by it.
           OR (NEW.shape = 4 AND (
                   fl.minted_transaction_id <> NEW.transaction_id
                OR NOT EXISTS (
                    SELECT 1 FROM game_item_container_entries e
                     WHERE e.item_instance_id = NEW.flask_item_instance_id
                       AND e.placed_transaction_id = NEW.transaction_id))) THEN
            RAISE EXCEPTION 'item use flask must match its stack or fresh item and entry'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=5 AND p.source_kind=1
      AND p.item_instance_id=OLD.item_instance_id AND p.character_id=OLD.character_id AND p.world_id=OLD.world_id
      AND p.source_parent=OLD.parent_item_instance_id AND p.source_ordinal=OLD.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

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

    -- 0083: the entry of a used stack spent to zero, by the item-use record of this same
    -- physical transaction.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_use_records u
          JOIN game_item_instances i ON i.item_instance_id = u.source_item_instance_id
         WHERE u.source_item_instance_id = OLD.item_instance_id
           AND u.source_quantity_after = 0
           AND u.character_id = OLD.character_id
           AND u.source_parent_item_instance_id = OLD.parent_item_instance_id
           AND EXISTS (SELECT 1 FROM game_item_loop_owner(OLD.parent_item_instance_id) o
                        WHERE o.owner_character = u.character_id AND o.depth BETWEEN 0 AND 7)
           AND u.source_placement_ordinal = OLD.placement_ordinal
           AND i.last_transaction_id = u.transaction_id
           AND i.lifecycle = 2
           AND u.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
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

CREATE OR REPLACE FUNCTION game_item_nested_entry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'INSERT' AND game_item_loop_line_current(NEW.item_instance_id) THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'DELETE' AND game_item_loop_line_current(OLD.item_instance_id) THEN
        RETURN NULL;
    END IF;
    -- BAGS-USE-1: the source entry spent to zero, in this same physical transaction.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_use_records u
          JOIN game_item_instances i ON i.item_instance_id = u.source_item_instance_id
          CROSS JOIN LATERAL game_item_loop_owner(OLD.parent_item_instance_id) o
         WHERE u.source_item_instance_id = OLD.item_instance_id
           AND u.world_id = OLD.world_id
           AND u.source_quantity_after = 0
           AND u.source_parent_item_instance_id = OLD.parent_item_instance_id
           AND u.source_placement_ordinal = OLD.placement_ordinal
           AND o.owner_character = u.character_id AND o.depth BETWEEN 1 AND 7
           AND i.last_transaction_id = u.transaction_id AND i.lifecycle = 2 AND i.quantity = 0
           AND u.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'nested container custody changes only by an item loop move' USING ERRCODE = '23514';
END;
$$;

CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=6 AND p.transaction_id=NEW.placed_transaction_id
      AND p.item_instance_id=NEW.item_instance_id AND p.world_id=NEW.world_id AND p.character_id=NEW.character_id
      AND p.destination_parent=NEW.parent_item_instance_id AND p.destination_ordinal=NEW.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    -- Guard record-field access before preparing the entry-only swap expression:
    -- the same trigger also receives slot rows, which have no parent or ordinal.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(
       SELECT 1 FROM game_character_equipment_receipts r
       JOIN game_item_instances i ON i.item_instance_id=r.swap_item_instance_id
       WHERE r.operation=1 AND r.swap_item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
         AND r.backpack_item_instance_id=NEW.parent_item_instance_id
         AND r.swap_backpack_ordinal=NEW.placement_ordinal AND r.world_id=NEW.world_id
         AND i.lifecycle=1 AND i.world_id=NEW.world_id AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.swap_state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'equipment swap backpack entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    END IF;
    IF TG_TABLE_NAME='game_item_container_entries' THEN
    -- 0082: the new entry of an NPC trade MINT line of this same physical transaction.
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
    -- 0083: the fresh flask item of an item-use record of this same physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u JOIN game_item_instances i ON i.item_instance_id = NEW.item_instance_id
       WHERE u.shape = 4 AND u.flask_item_instance_id = NEW.item_instance_id
         AND u.transaction_id = NEW.placed_transaction_id AND u.character_id = NEW.character_id
         AND EXISTS (SELECT 1 FROM game_item_container_slots s
                      WHERE s.character_id = u.character_id
                        AND s.item_instance_id = NEW.parent_item_instance_id)
         AND u.flask_placement_ordinal = NEW.placement_ordinal
         AND i.lifecycle = 1 AND u.created_xact_id = pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
           RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
       END IF;
       RETURN NULL;
    END IF;
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
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_use_consistency_guard',
                             'game_item_container_entry_removal_proven',
                             'game_item_nested_entry_proven', 'game_item_placement_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
END $$;
