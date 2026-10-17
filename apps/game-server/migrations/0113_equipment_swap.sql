-- CP D1282 (#2131/6227784667): additive migration 0113 for EQUIP-SWAP.
-- PARITY_PENDING: classes are receipt-attested Rust Content observations bound
-- to content_digest, not an independent SQL Content oracle. Existing guards below
-- retain their non-swap branches; location exclusivity remains the 0076 owner.
-- Equipment swap: two whole different-definition or admitted non-stackable items, four location lines,
-- two participants / six work units. Existing nine-slot and backpack ceilings unchanged.
ALTER TABLE game_character_equipment_receipts
    ADD COLUMN swap_item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
    ADD COLUMN swap_state_revision_before NUMERIC(20,0),
    ADD COLUMN swap_backpack_ordinal NUMERIC(20,0),
    ADD CONSTRAINT game_equipment_swap_shape CHECK (
       num_nonnulls(swap_item_instance_id,swap_state_revision_before,swap_backpack_ordinal)=0
       OR (num_nonnulls(swap_item_instance_id,swap_state_revision_before,swap_backpack_ordinal)=3
           AND operation=1 AND to_slot IS NOT NULL AND from_slot IS NULL
           AND swap_item_instance_id<>item_instance_id
           AND swap_state_revision_before BETWEEN 1 AND 18446744073709551614
           AND swap_backpack_ordinal BETWEEN 1 AND 18446744073709551615
           AND swap_backpack_ordinal>backpack_ordinal));

CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.last_transaction_id
     AND p.item_instance_id=NEW.item_instance_id AND p.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_house_shape_complete(NEW.last_transaction_id);RETURN NULL;END IF;

    -- Same physical transaction's second whole item in the equipment swap.
    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       WHERE r.operation=1 AND r.swap_item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.last_transaction_id AND r.swap_state_revision_before=OLD.state_revision
         AND NEW.state_revision=OLD.state_revision+1 AND NEW.quantity=OLD.quantity
         AND NEW.lifecycle=OLD.lifecycle AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    -- 0082: the BURN line of an NPC trade of this same physical transaction.
    IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
                                AND t.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    -- 0083: the used stack (and the flask stack it grows) of an item-use record of this same
    -- physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u
                WHERE u.transaction_id = NEW.last_transaction_id
                  AND u.created_xact_id = pg_current_xact_id()
                  AND (u.source_item_instance_id = NEW.item_instance_id
                       OR (u.shape = 3 AND u.flask_item_instance_id = NEW.item_instance_id))) THEN
        RETURN NULL;
    END IF;
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

CREATE OR REPLACE FUNCTION game_equipment_slot_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=5 AND p.source_kind=3
      AND p.item_instance_id=OLD.item_instance_id AND p.world_id=OLD.world_id AND p.character_id=OLD.character_id
      AND p.source_slot=OLD.slot AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       JOIN game_item_instances i ON i.item_instance_id=r.swap_item_instance_id
       WHERE r.operation=1 AND r.swap_item_instance_id=OLD.item_instance_id
         AND r.character_id=OLD.character_id AND r.to_slot=OLD.slot AND r.world_id=OLD.world_id
         AND i.lifecycle=1 AND i.world_id=OLD.world_id AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.swap_state_revision_before+1
         AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=3
        AND r.character_id=OLD.character_id AND r.source_slot=OLD.slot AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
      JOIN game_spell_equipment_consumptions c USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind=5 AND l.quantity_after=0 AND l.item_instance_id=OLD.item_instance_id AND l.source_equipment_slot=OLD.slot
       AND l.source_equipment_revision=c.revision_before AND c.character_id=OLD.character_id AND r.character_id=OLD.character_id
       AND r.world_id=OLD.world_id AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=l.transaction_id
       AND i.state_revision=l.state_revision_before+1 AND l.created_xact_id=pg_current_xact_id()
       AND r.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'equipment slot updates require delete/insert' USING ERRCODE='23514'; END IF;
    IF TG_OP='INSERT' THEN
        IF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
            WHERE r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
              AND r.item_instance_id=NEW.item_instance_id AND r.to_slot=NEW.slot AND r.operation=1
              AND r.world_id=NEW.world_id AND i.world_id=NEW.world_id AND i.lifecycle=1
              AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
              AND r.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'equipment insertion lacks actual movement receipt' USING ERRCODE='23514';
        END IF;
    ELSIF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r
        WHERE r.character_id=OLD.character_id AND r.item_instance_id=OLD.item_instance_id
          AND r.from_slot=OLD.slot AND r.operation=1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipment removal lacks actual movement receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

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
         AND u.source_parent_item_instance_id = NEW.parent_item_instance_id
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

CREATE OR REPLACE FUNCTION game_equipment_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_character_equipment_state s
        WHERE s.character_id=NEW.character_id AND s.revision=NEW.revision_after
          AND s.last_transaction_id=NEW.transaction_id AND s.combat_mode IS NOT DISTINCT FROM NEW.mode_after)
       OR NOT EXISTS(SELECT 1 FROM game_character_equipment_audit_outbox a
        WHERE a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id
          AND a.created_xact_id=pg_current_xact_id() AND a.envelope=NEW.intent) THEN
        RAISE EXCEPTION 'equipment receipt/state/audit must commit together' USING ERRCODE='23514';
    END IF;
    IF NEW.operation=1 AND NOT EXISTS(SELECT 1 FROM game_item_instances i
        WHERE i.item_instance_id=NEW.item_instance_id AND i.lifecycle=1
          AND i.last_transaction_id=NEW.transaction_id AND i.state_revision=NEW.state_revision_before+1
          AND ((NEW.to_slot IS NOT NULL AND EXISTS(SELECT 1 FROM game_character_equipment_slots s
                    WHERE s.character_id=NEW.character_id AND s.slot=NEW.to_slot AND s.item_instance_id=i.item_instance_id))
            OR (NEW.from_slot IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e
                    WHERE e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.backpack_item_instance_id
                      AND e.placement_ordinal=NEW.backpack_ordinal AND e.item_instance_id=i.item_instance_id)))) THEN
        RAISE EXCEPTION 'equipment receipt lacks actual item custody successor' USING ERRCODE='23514';
    END IF;

    IF NEW.swap_item_instance_id IS NOT NULL THEN
        IF NOT EXISTS(SELECT 1 FROM game_item_instances i
           JOIN game_item_instances incoming ON incoming.item_instance_id=NEW.item_instance_id
           JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id
           WHERE i.item_instance_id=NEW.swap_item_instance_id AND i.lifecycle=1
             AND i.world_id=NEW.world_id AND i.last_transaction_id=NEW.transaction_id
             AND i.state_revision=NEW.swap_state_revision_before+1
             -- Content class authority remains the current qualified Rust Content owner.
             -- These are receipt-attested observations bound to content_digest, not a new
             -- SQL Content oracle. CP must approve this additive guard/migration proposal.
             AND ((i.definition_production_key,i.definition_revision_ref)
                 IS DISTINCT FROM (incoming.definition_production_key,incoming.definition_revision_ref)
                 OR (i.quantity=1 AND incoming.quantity=1
                     AND convert_from(NEW.intent,'UTF8')::jsonb #>> '{items,0,stack_class}'='NON_STACKABLE'
                     AND convert_from(NEW.intent,'UTF8')::jsonb #>> '{items,1,stack_class}'='NON_STACKABLE'))
             AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
             AND e.parent_item_instance_id=NEW.backpack_item_instance_id
             AND e.placement_ordinal=NEW.swap_backpack_ordinal
             AND e.placed_transaction_id=NEW.transaction_id)
           OR EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=NEW.swap_item_instance_id)
           OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=NEW.item_instance_id)
        THEN RAISE EXCEPTION 'equipment swap lacks both exact custody successors' USING ERRCODE='23514'; END IF;
        -- The one audit envelope binds both members, not only the incoming item's intent.
        IF (convert_from(NEW.intent,'UTF8')::jsonb ->> 'schema') IS DISTINCT FROM 'OTERYN_EQUIPMENT_COMMAND/v2'
           OR (convert_from(NEW.intent,'UTF8')::jsonb -> 'swap') IS DISTINCT FROM jsonb_build_object(
               'item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.swap_item_instance_id),n) FROM generate_series(0,15) n)),
               'state_revision',NEW.swap_state_revision_before,'from_slot',NEW.to_slot,'to','MAIN_BACKPACK')
        THEN RAISE EXCEPTION 'equipment swap audit lacks exact second member' USING ERRCODE='23514'; END IF;

        IF COALESCE(convert_from(NEW.intent,'UTF8')::jsonb #>> '{items,0,stack_class}','') NOT IN ('NON_STACKABLE','STACKABLE','UNKNOWN')
           OR COALESCE(convert_from(NEW.intent,'UTF8')::jsonb #>> '{items,1,stack_class}','') NOT IN ('NON_STACKABLE','STACKABLE','UNKNOWN')
        THEN RAISE EXCEPTION 'equipment swap lacks bound stack class observations' USING ERRCODE='23514'; END IF;

        IF NOT EXISTS(SELECT 1 FROM game_item_instances incoming,game_item_instances displaced
           WHERE incoming.item_instance_id=NEW.item_instance_id
             AND displaced.item_instance_id=NEW.swap_item_instance_id
             AND convert_from(NEW.intent,'UTF8')::jsonb -> 'locations' = jsonb_build_array(
                 jsonb_build_object('item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.item_instance_id),n) FROM generate_series(0,15) n)),'operation','REMOVE','kind','MAIN_BACKPACK_ENTRY','character',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.character_id),n) FROM generate_series(0,15) n)),'world',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.world_id),n) FROM generate_series(0,15) n)),'parent',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.backpack_item_instance_id),n) FROM generate_series(0,15) n)),'ordinal',NEW.backpack_ordinal),
                 jsonb_build_object('item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.item_instance_id),n) FROM generate_series(0,15) n)),'operation','PLACE','kind','CHARACTER_EQUIPMENT','character',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.character_id),n) FROM generate_series(0,15) n)),'world',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.world_id),n) FROM generate_series(0,15) n)),'slot',NEW.to_slot),
                 jsonb_build_object('item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.swap_item_instance_id),n) FROM generate_series(0,15) n)),'operation','REMOVE','kind','CHARACTER_EQUIPMENT','character',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.character_id),n) FROM generate_series(0,15) n)),'world',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.world_id),n) FROM generate_series(0,15) n)),'slot',NEW.to_slot),
                 jsonb_build_object('item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.swap_item_instance_id),n) FROM generate_series(0,15) n)),'operation','PLACE','kind','MAIN_BACKPACK_ENTRY','character',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.character_id),n) FROM generate_series(0,15) n)),'world',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.world_id),n) FROM generate_series(0,15) n)),'parent',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.backpack_item_instance_id),n) FROM generate_series(0,15) n)),'ordinal',NEW.swap_backpack_ordinal))
             AND convert_from(NEW.intent,'UTF8')::jsonb -> 'items' = jsonb_build_array(
                 jsonb_build_object('item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.item_instance_id),n) FROM generate_series(0,15) n)),'definition',jsonb_build_object('family',incoming.definition_family,'production_key',incoming.definition_production_key,'revision_ref',incoming.definition_revision_ref),'stack_class',convert_from(NEW.intent,'UTF8')::jsonb #>> '{items,0,stack_class}','quantity',incoming.quantity,'state_revision_before',NEW.state_revision_before,'state_revision_after',NEW.state_revision_before+1),
                 jsonb_build_object('item',to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.swap_item_instance_id),n) FROM generate_series(0,15) n)),'definition',jsonb_build_object('family',displaced.definition_family,'production_key',displaced.definition_production_key,'revision_ref',displaced.definition_revision_ref),'stack_class',convert_from(NEW.intent,'UTF8')::jsonb #>> '{items,1,stack_class}','quantity',displaced.quantity,'state_revision_before',NEW.swap_state_revision_before,'state_revision_after',NEW.swap_state_revision_before+1)))
        THEN RAISE EXCEPTION 'equipment swap event lacks four exact locations and two item successors' USING ERRCODE='23514'; END IF;
    END IF;
    RETURN NULL;
END $$;
-- CREATE OR REPLACE keeps prior owner/ACL, but rebind security settings explicitly.
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_equipment_slot_proven','game_equipment_receipt_proven',
        'game_item_instance_change_proven','game_item_placement_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
END $$;
-- Existing immutable receipt/audit triggers, state successor proof, incoming entry-removal
-- proof and item single-location deferred triggers remain installed without alteration.
