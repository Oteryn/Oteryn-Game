-- LOCAL CANDIDATE CHARACTER-EQUIPMENT-1. Existing container-slot baseline is unchanged.
-- Real ItemInstances move between a direct backpack entry and one non-container equipment slot.
-- No premium, learning, Wheel allocation, combat mode or item-stat grant is seeded.
CREATE TABLE game_character_equipment_state (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
    combat_mode SMALLINT CHECK(combat_mode IN (1,2,3)),
    last_transaction_id UUID,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_character_equipment_receipts (
    transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    event_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(event_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
    lease_generation NUMERIC(20,0) NOT NULL CHECK(lease_generation>0),
    world_id UUID NOT NULL, channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
    content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
    revision_before NUMERIC(20,0) NOT NULL CHECK(revision_before>0),
    revision_after NUMERIC(20,0) NOT NULL CHECK(revision_after=revision_before+1 AND revision_after<=18446744073709551615),
    mode_before SMALLINT CHECK(mode_before IN(1,2,3)), mode_after SMALLINT CHECK(mode_after IN(1,2,3)),
    operation SMALLINT NOT NULL CHECK(operation IN(1,2)), -- movement, explicit combat-mode change
    item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
    state_revision_before NUMERIC(20,0),
    from_slot SMALLINT CHECK(from_slot BETWEEN 1 AND 10 AND from_slot<>9),
    to_slot SMALLINT CHECK(to_slot BETWEEN 1 AND 10 AND to_slot<>9),
    backpack_item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
    backpack_ordinal NUMERIC(20,0),
    binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
    intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 65536 AND binding=sha256(intent)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id),
    CHECK((operation=1 AND item_instance_id IS NOT NULL AND state_revision_before>0
       AND num_nonnulls(from_slot,to_slot)=1 AND backpack_item_instance_id IS NOT NULL AND backpack_ordinal>0
       AND mode_before IS NOT DISTINCT FROM mode_after)
       OR (operation=2 AND item_instance_id IS NULL AND from_slot IS NULL AND to_slot IS NULL
       AND mode_after IS NOT NULL))
);
CREATE TABLE game_character_equipment_slots (
    character_id UUID NOT NULL REFERENCES game_character_equipment_state(character_id),
    slot SMALLINT NOT NULL CHECK(slot BETWEEN 1 AND 10 AND slot<>9),
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
    world_id UUID NOT NULL,
    placed_transaction_id UUID NOT NULL REFERENCES game_character_equipment_receipts(transaction_id),
    PRIMARY KEY(character_id,slot)
);
CREATE TABLE game_character_equipment_audit_outbox (
    event_id UUID PRIMARY KEY REFERENCES game_character_equipment_receipts(event_id),
    transaction_id UUID UNIQUE NOT NULL REFERENCES game_character_equipment_receipts(transaction_id),
    envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 65536),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_equipment_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' THEN
        IF NEW.revision<>1 OR NEW.combat_mode IS NOT NULL OR NEW.last_transaction_id IS NOT NULL THEN
            RAISE EXCEPTION 'equipment seed has known empty slots and unknown combat mode' USING ERRCODE='23514';
        END IF;
    ELSIF (to_jsonb(NEW)-'revision'-'combat_mode'-'last_transaction_id')<>(to_jsonb(OLD)-'revision'-'combat_mode'-'last_transaction_id')
       OR NEW.revision<>OLD.revision+1 OR NEW.last_transaction_id IS NULL
       OR NEW.last_transaction_id IS NOT DISTINCT FROM OLD.last_transaction_id THEN
        RAISE EXCEPTION 'equipment state requires exact receipt-bound successor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_equipment_state_guard BEFORE INSERT OR UPDATE ON game_character_equipment_state
FOR EACH ROW EXECUTE FUNCTION game_equipment_state_guard();
CREATE FUNCTION game_equipment_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r
        WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id
          AND r.revision_after=NEW.revision AND r.revision_before=OLD.revision
          AND r.mode_before IS NOT DISTINCT FROM OLD.combat_mode
          AND r.mode_after IS NOT DISTINCT FROM NEW.combat_mode
          AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipment successor lacks same-transaction receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_equipment_state_proven AFTER UPDATE ON game_character_equipment_state
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_equipment_state_proven();
CREATE FUNCTION game_equipment_slot_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
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
CREATE CONSTRAINT TRIGGER game_equipment_slot_proven AFTER INSERT OR DELETE ON game_character_equipment_slots
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_equipment_slot_proven();
CREATE TRIGGER game_equipment_slot_no_update BEFORE UPDATE ON game_character_equipment_slots
FOR EACH ROW EXECUTE FUNCTION game_equipment_slot_proven();
CREATE FUNCTION game_equipment_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
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
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_equipment_receipt_proven AFTER INSERT ON game_character_equipment_receipts
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_equipment_receipt_proven();

-- Guard extensions from the final 0033 bodies are appended by the owner below.

-- Existing 0025 location exclusivity, exact same old branches plus equipped custody.
CREATE OR REPLACE FUNCTION game_item_location_exclusive(p_item UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_lifecycle SMALLINT;
    v_locations BIGINT;
BEGIN
    -- The row lock serializes two transactions that each place the same item
    -- in a different location table: the later one waits here and then
    -- counts the earlier one's committed row. Every existing writer already
    -- holds this lock from its own item UPDATE.
    SELECT lifecycle INTO v_lifecycle FROM game_item_instances
     WHERE item_instance_id = p_item FOR NO KEY UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    v_locations :=
          (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item);
    IF v_locations <> (v_lifecycle = 1)::INT THEN
        RAISE EXCEPTION 'DUR-03 item must have exactly one location while live and none when retired'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'HouseInterior item cannot have contents' USING ERRCODE = '23514';
    END IF;
END;
$$;
CREATE CONSTRAINT TRIGGER game_equipment_location_exclusive AFTER INSERT OR DELETE ON game_character_equipment_slots
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_location_exclusivity_guard();

-- Exact final 0033 body, with the narrow same-transaction equipment branch first.
CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Exact final 0033 body, with the narrow same-transaction equipment branch first.
CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Exact final 0033 body, with the narrow same-transaction equipment branch first.
CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_TABLE_NAME='game_item_container_entries' AND EXISTS(
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
    IF TG_TABLE_NAME='game_item_container_entries' AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.operation_kind=1 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
        AND l.destination_parent_item_instance_id=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
        AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514'; END IF;
       RETURN NULL;
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

CREATE FUNCTION game_equipment_reject_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'equipment owner cannot be deleted' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_equipment_state_no_delete BEFORE DELETE ON game_character_equipment_state
FOR EACH ROW EXECUTE FUNCTION game_equipment_reject_delete();
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_character_equipment_receipts','game_character_equipment_audit_outbox'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n || '_immutable',n);
    END LOOP;
    FOREACH n IN ARRAY ARRAY['game_character_equipment_state','game_character_equipment_slots','game_character_equipment_receipts','game_character_equipment_audit_outbox'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',n || '_no_truncate',n);
    END LOOP;
    FOREACH n IN ARRAY ARRAY['game_equipment_state_guard','game_equipment_state_proven','game_equipment_slot_proven','game_equipment_receipt_proven','game_equipment_reject_delete','game_item_instance_change_proven','game_item_container_entry_removal_proven','game_item_placement_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON game_character_equipment_state,game_character_equipment_slots,game_character_equipment_receipts,game_character_equipment_audit_outbox FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_equipment_state TO oteryn_game_runtime;
GRANT SELECT,INSERT,DELETE ON game_character_equipment_slots TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_equipment_receipts,game_character_equipment_audit_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_character_equipment_state,game_character_equipment_slots,game_character_equipment_receipts,game_character_equipment_audit_outbox TO oteryn_game_control;
