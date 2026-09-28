-- DUR-03 TRANSFER into the equipped main backpack (B3 decision
-- B3-INVENTORY-DESTINATION-CAPACITY-STACKS-V1, D80-D83; child B3-1).
--
-- A player-originated TRANSFER moves one live Ground ItemInstance into the
-- character's CharacterEquipment `container` slot (an empty container into an
-- empty slot) or into a direct entry of the item in that slot (the main
-- backpack), including the D83 full-merge and top-up shapes. Each logical
-- TRANSFER is reserved and committed under its full FND-02 CommandRef
-- (game_session_id, command_id); the receipt, the audit event and every item
-- and location change commit together or not at all. There is no receipt
-- expiry and no deletion path. A TRANSFER writes no Character root,
-- progression or XP-receipt row (CHARACTER-REVISION-ITEM-TRANSACTION-
-- COMPOSITION-V1). Nested bags, other equipment slots, drop and depot stay
-- closed: an entry's parent must be the character's slot item (depth 1).

-- Item lifecycle: 1 LIVE, 2 RETIRED (DUR-03 §11.4/§11.5: a stack reduced to
-- zero retires in the same atomic outcome). A retired item keeps its identity
-- with quantity 0 and no location.
ALTER TABLE game_item_instances DROP CONSTRAINT game_item_instances_quantity_check;
ALTER TABLE game_item_instances DROP CONSTRAINT game_item_instances_lifecycle_check;
ALTER TABLE game_item_instances
    ADD COLUMN last_transaction_id UUID NULL
        CHECK (last_transaction_id IS NULL OR game_character_is_uuid_v7(last_transaction_id));
ALTER TABLE game_item_instances
    ADD CONSTRAINT game_item_instances_lifecycle_quantity CHECK (
        (lifecycle = 1 AND quantity BETWEEN 1 AND 4294967295)
        OR (lifecycle = 2 AND quantity = 0 AND last_transaction_id IS NOT NULL));

-- Repair generation 2 (Codex review 5343738115, findings 1-2): the physical
-- transaction that created a receipt, so a downstream INSERT/UPDATE/DELETE
-- can never borrow a historical receipt's logical TransactionId from an
-- already-committed physical transaction. `xid8` compares by plain value
-- (no wraparound) and is stable for the lifetime of one transaction.
ALTER TABLE game_item_mint_receipts
    ADD COLUMN created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id();
-- The same binding for audit events: a TRANSFER must be proven by an audit
-- row its own physical transaction inserted, never by a historical MINT row.
ALTER TABLE game_item_audit_outbox
    ADD COLUMN created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id();

-- CharacterEquipment slot `container` (Global's Container Slot): at most one
-- item per character, and that item is the character's main backpack.
CREATE TABLE game_item_container_slots (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots (character_id),
    item_instance_id UUID NOT NULL UNIQUE,
    world_id UUID NOT NULL,
    placed_transaction_id UUID NOT NULL UNIQUE
        CHECK (game_character_is_uuid_v7(placed_transaction_id)),
    UNIQUE (character_id, item_instance_id),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

-- Container { parent = main backpack, entry = placement ordinal }. The ordinal
-- is unique among the parent's entries; a placement takes the highest ordinal
-- plus one under the character_root row lock and never renumbers. Display
-- order is newest (highest ordinal) first. The composite FK makes the parent
-- the character's own slot item, so placement depth is 1
-- (GAMEITEM01-PLACEMENT-DEPTH).
CREATE TABLE game_item_container_entries (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    character_id UUID NOT NULL,
    parent_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    placed_transaction_id UUID NOT NULL UNIQUE
        CHECK (game_character_is_uuid_v7(placed_transaction_id)),
    CHECK (item_instance_id <> parent_item_instance_id),
    UNIQUE (parent_item_instance_id, placement_ordinal),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id),
    FOREIGN KEY (character_id, parent_item_instance_id)
        REFERENCES game_item_container_slots (character_id, item_instance_id)
);

-- One logical TRANSFER per CommandRef, forever. Committed in its own
-- transaction before the first commit pass, it binds the frozen TransactionId,
-- EventId and trusted timestamp, the intent (source item, destination kind,
-- definition facts, revisions) and the DUR03-RL-08 work units already
-- charged. The only mutation is a +1 work-unit charge up to 3.
CREATE TABLE game_item_transfer_reservations (
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL CHECK (game_character_is_uuid_v7(character_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    source_item_instance_id UUID NOT NULL CHECK (game_character_is_uuid_v7(source_item_instance_id)),
    -- 1 = CharacterEquipment container slot, 2 = main backpack entry.
    destination_kind SMALLINT NOT NULL CHECK (destination_kind IN (1,2)),
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) = 33),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    -- DUR03-RL-08: work units per logical transaction.
    work_units_used SMALLINT NOT NULL CHECK (work_units_used BETWEEN 0 AND 3),
    reserved_at BIGINT NOT NULL CHECK (reserved_at >= 0),
    PRIMARY KEY (game_session_id, command_id)
);

-- Terminal result of one CommandRef. Shapes (D83): 1 container slot, 2 new
-- backpack entry, 3 full merge (source retires into the receiver), 4 top-up
-- (receiver grows to the stack maximum, the source keeps the remainder in a
-- new entry). Exact units are conserved by the CHECKs below.
CREATE TABLE game_item_transfer_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    character_id UUID NOT NULL,
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) = 33),
    transaction_id UUID NOT NULL UNIQUE,
    event_id UUID NOT NULL UNIQUE,
    shape SMALLINT NOT NULL CHECK (shape BETWEEN 1 AND 4),
    source_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    source_quantity_before BIGINT NOT NULL CHECK (source_quantity_before BETWEEN 1 AND 4294967295),
    source_quantity_after BIGINT NOT NULL CHECK (source_quantity_after BETWEEN 0 AND 4294967295),
    receiver_item_instance_id UUID NULL REFERENCES game_item_instances (item_instance_id),
    receiver_quantity_before BIGINT NULL CHECK (receiver_quantity_before BETWEEN 1 AND 4294967295),
    receiver_quantity_after BIGINT NULL CHECK (receiver_quantity_after BETWEEN 1 AND 4294967295),
    destination_parent_item_instance_id UUID NULL,
    destination_ordinal NUMERIC(20,0) NULL
        CHECK (destination_ordinal BETWEEN 1 AND 18446744073709551615),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- The physical transaction that inserted this receipt (repair generation
    -- 2, finding 2): a downstream placement or item/location change must be
    -- proven against a receipt created in that SAME physical transaction,
    -- never a historical one reused by TransactionId value alone.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (game_session_id, command_id),
    CHECK (receiver_item_instance_id IS DISTINCT FROM source_item_instance_id),
    CHECK ((destination_parent_item_instance_id IS NULL) = (destination_ordinal IS NULL)),
    CHECK ((receiver_item_instance_id IS NULL) = (receiver_quantity_before IS NULL)
       AND (receiver_item_instance_id IS NULL) = (receiver_quantity_after IS NULL)),
    CHECK (CASE shape
        WHEN 1 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NULL
                    AND source_quantity_after = source_quantity_before
        WHEN 2 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NOT NULL
                    AND source_quantity_after = source_quantity_before
        WHEN 3 THEN receiver_item_instance_id IS NOT NULL AND destination_ordinal IS NULL
                    AND source_quantity_after = 0
                    AND receiver_quantity_after = receiver_quantity_before + source_quantity_before
        ELSE receiver_item_instance_id IS NOT NULL AND destination_ordinal IS NOT NULL
                    AND source_quantity_after > 0
                    AND receiver_quantity_after > receiver_quantity_before
                    AND receiver_quantity_after - receiver_quantity_before
                        = source_quantity_before - source_quantity_after
    END)
);

-- Guarded per-transaction evidence of the actual OLD.quantity of every item
-- row a TRANSFER touches, captured by the item UPDATE trigger below from the
-- real row, never from a client-supplied value. Append-only: a later
-- TRANSFER of the same item records a new row under its own transaction. The
-- deferred conservation guard below binds the receipt's self-reported
-- *_quantity_before to this evidence, so a runtime transaction can no longer
-- claim a fictitious before-quantity for either participant.
CREATE TABLE game_item_transfer_quantity_evidence (
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(transaction_id)),
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 1 AND 4294967295),
    PRIMARY KEY (item_instance_id, transaction_id)
);

-- Guarded evidence of the real pre-DELETE World and Channel of a Ground row,
-- captured by the Ground DELETE trigger below from the actual OLD row, never
-- from a client-supplied value (repair generation 2, finding 3). The Ground
-- row is gone by the time the deferred receipt guard runs, and TRANSFER never
-- returns an item to Ground (the insertion guard below only ever admits a
-- fresh MINT), so at most one removal, hence one evidence row, ever exists
-- per item. Only the SECURITY DEFINER capture trigger writes it.
CREATE TABLE game_item_ground_removal_evidence (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL
);

CREATE FUNCTION game_item_transfer_reservation_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.work_units_used = OLD.work_units_used + 1
       AND (to_jsonb(NEW) - 'work_units_used') = (to_jsonb(OLD) - 'work_units_used') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item TRANSFER reservation is immutable except its work-unit charge'
        USING ERRCODE = '23514';
END;
$$;

-- A live item may change only its quantity, lifecycle and last transaction,
-- and only to a new transaction; a retired item is terminal. The deferred
-- guard below proves a TRANSFER receipt of that transaction names it. Before
-- allowing the change, this trigger itself captures OLD.quantity -- the real
-- row's prior value, never anything a caller supplies -- as guarded evidence
-- for that (item, transaction) pair. It is SECURITY DEFINER so the evidence
-- table needs no write grant to the runtime role: nothing but this trigger,
-- reading the genuine OLD row, can ever populate it.
CREATE FUNCTION game_item_instance_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'quantity' - 'lifecycle' - 'last_transaction_id')
         = (to_jsonb(OLD) - 'quantity' - 'lifecycle' - 'last_transaction_id') THEN
        INSERT INTO game_item_transfer_quantity_evidence
            (item_instance_id, transaction_id, quantity_before)
        VALUES (OLD.item_instance_id, NEW.last_transaction_id, OLD.quantity);
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item record changes only through a TRANSFER'
        USING ERRCODE = '23514';
END;
$$;

CREATE FUNCTION game_item_instance_change_proven() RETURNS trigger
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
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Ground custody ends only as the source of a TRANSFER committed in the same
-- transaction (the item's last transaction is that receipt's).
CREATE FUNCTION game_item_ground_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Ground removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Ground gains a location only as a fresh MINT's own receipt, in the same
-- physical transaction, for a live item that was never placed or transferred
-- before (repair generation 2, finding 1): this closes reinsertion of a
-- Ground row after a TRANSFER (a live item both in a container and on
-- Ground) or for a retired item regaining a location. SECURITY DEFINER is
-- not needed: the runtime already holds SELECT on both relations read here.
CREATE FUNCTION game_item_ground_insertion_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i
         WHERE i.item_instance_id = NEW.item_instance_id
           AND i.world_id = NEW.world_id
           AND i.lifecycle = 1
           AND i.last_transaction_id IS NULL)
       OR NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts r
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Ground placement must be the item MINT of the current transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Guarded per-transaction evidence of the real pre-DELETE World and Channel
-- of a Ground row, captured from the actual OLD row (never a client-supplied
-- value). SECURITY DEFINER so the evidence table needs no write grant to the
-- runtime role.
CREATE FUNCTION game_item_ground_removal_evidence_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    INSERT INTO game_item_ground_removal_evidence (item_instance_id, world_id, channel_id)
    VALUES (OLD.item_instance_id, OLD.world_id, OLD.channel_id);
    RETURN OLD;
END;
$$;

-- A slot or entry placement exists only as the destination of its TRANSFER,
-- in the receipt's own physical transaction, with the receipt's shape
-- permitting a placement, the parent/ordinal matching (entries) and the item
-- live (repair generation 2, finding 2: the prior check let a placement
-- insert reuse a historical, non-placement (e.g. full-merge) receipt's
-- TransactionId for an arbitrary parent/ordinal). GAMEITEM01-CONTAINER-
-- ENTRIES-MAX: at most 20 entries per parent, made concurrency-safe (finding
-- 4) by locking the parent's container-slot row before counting, so two
-- concurrent inserts for the same parent serialize instead of racing the
-- count. SECURITY DEFINER: the row lock needs UPDATE privilege on
-- game_item_container_slots, which the runtime role is not granted.
CREATE FUNCTION game_item_placement_proven() RETURNS trigger
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
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER receipt'
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

-- Commit-time atomicity of one TRANSFER: its reservation, its audit event,
-- the exact source and receiver after-states, exactly one immediate location
-- for every live participant and none for a retired source.
CREATE FUNCTION game_item_transfer_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    src game_item_instances%ROWTYPE;
    rcv game_item_instances%ROWTYPE;
    slot_item UUID;
    src_places INTEGER;
BEGIN
    SELECT * INTO src FROM game_item_instances WHERE item_instance_id = NEW.source_item_instance_id;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = src.item_instance_id)
      INTO src_places;
    IF NOT EXISTS (
           SELECT 1 FROM game_item_transfer_reservations v
            WHERE (v.game_session_id, v.command_id, v.character_id, v.source_item_instance_id,
                   v.intent_binding, v.transaction_id, v.event_id, v.occurred_at)
                = (NEW.game_session_id, NEW.command_id, NEW.character_id,
                   NEW.source_item_instance_id, NEW.intent_binding, NEW.transaction_id,
                   NEW.event_id, NEW.occurred_at)
              AND v.world_id = src.world_id
              AND (v.destination_kind = 1) = (NEW.shape = 1))
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.source_item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id())
       -- The destination Character must be rooted in the source item's World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = src.world_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.source_item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.source_quantity_before)
       -- Repair generation 2, finding 3: the reservation's World and Channel
       -- must equal the Ground row this TRANSFER actually removed, proven
       -- from the guarded pre-DELETE evidence (the Ground row itself is gone
       -- by now).
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_reservations v
             JOIN game_item_ground_removal_evidence gre
               ON gre.item_instance_id = NEW.source_item_instance_id
              AND gre.world_id = v.world_id
              AND gre.channel_id = v.channel_id
            WHERE v.game_session_id = NEW.game_session_id AND v.command_id = NEW.command_id)
       OR src.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR src.quantity <> NEW.source_quantity_after
       OR src.lifecycle <> (CASE WHEN NEW.source_quantity_after = 0 THEN 2 ELSE 1 END)
       OR src_places <> (CASE WHEN NEW.shape = 3 THEN 0 ELSE 1 END)
       OR (NEW.shape = 1 AND NOT EXISTS (
           SELECT 1 FROM game_item_container_slots s
            WHERE s.character_id = NEW.character_id
              AND s.item_instance_id = src.item_instance_id
              AND s.placed_transaction_id = NEW.transaction_id))
       OR (NEW.shape IN (2,4) AND NOT EXISTS (
           SELECT 1 FROM game_item_container_entries e
            WHERE e.item_instance_id = src.item_instance_id
              AND e.character_id = NEW.character_id
              AND e.parent_item_instance_id = NEW.destination_parent_item_instance_id
              AND e.parent_item_instance_id = slot_item
              AND e.placement_ordinal = NEW.destination_ordinal
              AND e.placed_transaction_id = NEW.transaction_id)) THEN
        RAISE EXCEPTION 'item TRANSFER must commit its reservation, audit event and complete item state together'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.receiver_item_instance_id IS NOT NULL THEN
        SELECT * INTO rcv FROM game_item_instances
         WHERE item_instance_id = NEW.receiver_item_instance_id;
        IF rcv.last_transaction_id IS DISTINCT FROM NEW.transaction_id
           OR rcv.lifecycle <> 1
           OR rcv.quantity <> NEW.receiver_quantity_after
           OR (rcv.world_id, rcv.definition_family, rcv.definition_production_key,
               rcv.definition_revision_ref)
              IS DISTINCT FROM (src.world_id, src.definition_family,
                                src.definition_production_key, src.definition_revision_ref)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_container_entries e
                WHERE e.item_instance_id = rcv.item_instance_id
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = slot_item)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_transfer_quantity_evidence ev
                WHERE ev.item_instance_id = rcv.item_instance_id
                  AND ev.transaction_id = NEW.transaction_id
                  AND ev.quantity_before = NEW.receiver_quantity_before) THEN
            RAISE EXCEPTION 'item TRANSFER receiver must be a compatible stack in the main backpack'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_transfer_consistent
    AFTER INSERT ON game_item_transfer_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_transfer_consistency_guard();
CREATE CONSTRAINT TRIGGER game_item_instance_change_proven
    AFTER UPDATE ON game_item_instances
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_instance_change_proven();
CREATE CONSTRAINT TRIGGER game_item_ground_removal_proven
    AFTER DELETE ON game_item_ground_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_ground_removal_proven();
CREATE TRIGGER game_item_ground_removal_evidence_capture BEFORE DELETE
    ON game_item_ground_locations FOR EACH ROW
    EXECUTE FUNCTION game_item_ground_removal_evidence_capture();
CREATE CONSTRAINT TRIGGER game_item_ground_location_insert_proven
    AFTER INSERT ON game_item_ground_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_ground_insertion_guard();
CREATE CONSTRAINT TRIGGER game_item_container_slot_proven
    AFTER INSERT ON game_item_container_slots
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_placement_proven();
CREATE CONSTRAINT TRIGGER game_item_container_entry_proven
    AFTER INSERT ON game_item_container_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_placement_proven();

-- 0010 made items and Ground rows fully immutable; TRANSFER narrows that.
DROP TRIGGER game_item_instance_immutable ON game_item_instances;
CREATE TRIGGER game_item_instance_guard BEFORE UPDATE OR DELETE
    ON game_item_instances FOR EACH ROW EXECUTE FUNCTION game_item_instance_guard();
DROP TRIGGER game_item_ground_location_immutable ON game_item_ground_locations;
CREATE TRIGGER game_item_ground_location_immutable BEFORE UPDATE
    ON game_item_ground_locations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE TRIGGER game_item_container_slot_immutable BEFORE UPDATE OR DELETE
    ON game_item_container_slots FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_container_entry_immutable BEFORE UPDATE OR DELETE
    ON game_item_container_entries FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_transfer_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_item_transfer_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_transfer_reservation_guard BEFORE UPDATE OR DELETE
    ON game_item_transfer_reservations FOR EACH ROW
    EXECUTE FUNCTION game_item_transfer_reservation_guard();
CREATE TRIGGER game_item_transfer_quantity_evidence_immutable BEFORE UPDATE OR DELETE
    ON game_item_transfer_quantity_evidence FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_ground_removal_evidence_immutable BEFORE UPDATE OR DELETE
    ON game_item_ground_removal_evidence FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE TRIGGER game_item_container_slots_no_truncate BEFORE TRUNCATE
    ON game_item_container_slots
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_container_entries_no_truncate BEFORE TRUNCATE
    ON game_item_container_entries
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_transfer_receipts_no_truncate BEFORE TRUNCATE
    ON game_item_transfer_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_transfer_reservations_no_truncate BEFORE TRUNCATE
    ON game_item_transfer_reservations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_transfer_quantity_evidence_no_truncate BEFORE TRUNCATE
    ON game_item_transfer_quantity_evidence
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_ground_removal_evidence_no_truncate BEFORE TRUNCATE
    ON game_item_ground_removal_evidence
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_transfer_reservation_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_removal_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_insertion_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_removal_evidence_capture() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_transfer_consistency_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON
    game_item_container_slots,
    game_item_container_entries,
    game_item_transfer_reservations,
    game_item_transfer_receipts,
    game_item_transfer_quantity_evidence,
    game_item_ground_removal_evidence
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_transfer_reservation_guard(),
    game_item_instance_guard(),
    game_item_instance_change_proven(),
    game_item_ground_removal_proven(),
    game_item_ground_insertion_guard(),
    game_item_ground_removal_evidence_capture(),
    game_item_placement_proven(),
    game_item_transfer_consistency_guard()
FROM PUBLIC;
GRANT SELECT, INSERT ON
    game_item_container_slots,
    game_item_container_entries,
    game_item_transfer_reservations,
    game_item_transfer_receipts
TO oteryn_game_runtime;
GRANT UPDATE (quantity, lifecycle, last_transaction_id) ON game_item_instances
    TO oteryn_game_runtime;
GRANT DELETE ON game_item_ground_locations TO oteryn_game_runtime;
GRANT UPDATE (work_units_used) ON game_item_transfer_reservations TO oteryn_game_runtime;
-- No INSERT/UPDATE/DELETE grant here: only the SECURITY DEFINER
-- game_item_instance_guard() trigger, reading the real OLD row, ever
-- populates this evidence, regardless of who performs the item UPDATE.
GRANT SELECT ON game_item_transfer_quantity_evidence TO oteryn_game_runtime;
-- Same pattern: only the SECURITY DEFINER game_item_ground_removal_evidence_
-- capture() trigger, reading the real OLD row, ever populates this evidence.
GRANT SELECT ON game_item_ground_removal_evidence TO oteryn_game_runtime;
GRANT SELECT ON
    game_item_container_slots,
    game_item_container_entries,
    game_item_transfer_reservations,
    game_item_transfer_receipts,
    game_item_transfer_quantity_evidence,
    game_item_ground_removal_evidence
TO oteryn_game_control;
