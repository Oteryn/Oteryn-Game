-- HOUSE-CUSTODY-1 (HOUSE-CUSTODY-0 §3.1-§3.3, §3.5; DUR-03 §5.2 as amended;
-- EXP-HOUSES-01 §14-§15): storage only.
--
-- The `HouseInterior` location family, its `HousingReclaimProvenance` and one
-- deferred item-level exclusivity guard over every location table. There is
-- no scope kind, no writer, no transfer shape and no reset code here; those
-- belong to the house interior runtime child and to MAP-OVERLAY-1. The
-- runtime role gets no grant on the house tables (§3.5): the closure is
-- enforced by the database.
--
-- Checks this migration cannot make, owned by the writer and the rebuild:
-- the position is a tile of exactly this house in the active bundle (§3.1),
-- the ordinal is the highest plus one under the house lock, and the reclaim
-- subject comes from the placer's own inventory or equipment (§3.2).

-- HouseInterior { house_id: HouseId {world_id, house_key}, spatial_position,
-- stack_ordinal }. `HouseId` is revision-free: the House content key without
-- a content revision. World-scoped, no ChannelId. Only items without
-- contents may enter (§3.1); the exclusivity guard rejects any child entry
-- under a house item.
CREATE TABLE game_item_house_interior_locations (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    -- The House content key (house catalogue contract §2.1), no revision.
    house_key TEXT NOT NULL CHECK (octet_length(house_key) <= 512
                                   AND house_key ~ '^oteryn:content\.house\.[a-z0-9_]+$'),
    spatial_position BYTEA NOT NULL CHECK (octet_length(spatial_position) BETWEEN 1 AND 128),
    stack_ordinal NUMERIC(20,0) NOT NULL
        CHECK (stack_ordinal BETWEEN 1 AND 18446744073709551615),
    placed_transaction_id UUID NOT NULL UNIQUE
        CHECK (game_character_is_uuid_v7(placed_transaction_id)),
    UNIQUE (world_id, house_key, spatial_position, stack_ordinal),
    UNIQUE (item_instance_id, world_id, house_key, placed_transaction_id),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

-- HousingReclaimProvenance {house_id, item_instance_id, reclaim_subject,
-- placement_transaction_id, provenance_revision}. At most one per item (the
-- primary key), 1:1 with a live HouseInterior row of the same house and
-- naming that row's placement transaction: the deferred FK below covers a
-- provenance without its row, the constraint trigger covers a row without
-- its provenance. The first-slice subject is the
-- placing CharacterId; ON DELETE RESTRICT keeps that Character from being
-- deleted while a provenance names it (EXP-HOUSES-01 §15). It is not a
-- location and grants no authority.
CREATE TABLE game_item_house_reclaim_provenance (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL,
    reclaim_subject_character_id UUID NOT NULL
        REFERENCES game_character_roots (character_id) ON DELETE RESTRICT,
    placement_transaction_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(placement_transaction_id)),
    provenance_revision NUMERIC(20,0) NOT NULL
        CHECK (provenance_revision BETWEEN 1 AND 18446744073709551615),
    -- The physical transaction that last inserted or updated this row, stamped
    -- by trigger (never caller-supplied). A new location row needs a provenance
    -- written in its own transaction, so a replacement row can never reuse an
    -- unchanged provenance and its old placement transaction.
    written_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY (item_instance_id, world_id, house_key, placement_transaction_id)
        REFERENCES game_item_house_interior_locations
            (item_instance_id, world_id, house_key, placed_transaction_id)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_item_house_reclaim_provenance_subject
    ON game_item_house_reclaim_provenance (reclaim_subject_character_id);

-- Every placement transaction ever used by a HouseInterior row, append-only.
-- A deleted row keeps its entry, so a later row (a move, a re-placement) can
-- never reuse a historical placement transaction, and the provenance, bound
-- to its row's placement by the FK above, can never return to one.
CREATE TABLE game_item_house_placement_transactions (
    placed_transaction_id UUID PRIMARY KEY,
    item_instance_id UUID NOT NULL
);

-- SECURITY DEFINER: nothing but this trigger writes the ledger.
CREATE FUNCTION game_item_house_placement_record() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_item_house_placement_transactions (placed_transaction_id, item_instance_id)
    VALUES (NEW.placed_transaction_id, NEW.item_instance_id);
    RETURN NEW;
EXCEPTION WHEN unique_violation THEN
    RAISE EXCEPTION 'HouseInterior placement transaction was already used'
        USING ERRCODE = '23505';
END;
$$;

-- A HouseInterior row commits only with its provenance, checked when the row
-- is inserted. A provenance is deleted only when its item leaves the house
-- (§3.2): a delete while the item still has a house row at commit fails, so a
-- delete and reinsert cannot replace the subject or bypass the update guard.
CREATE FUNCTION game_item_house_interior_provenance_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF EXISTS (SELECT 1 FROM game_item_house_interior_locations h
                    WHERE h.item_instance_id = OLD.item_instance_id) THEN
            RAISE EXCEPTION 'HousingReclaimProvenance is deleted only when its item leaves the house'
                USING ERRCODE = '23514';
        END IF;
    ELSIF EXISTS (SELECT 1 FROM game_item_house_interior_locations h
                WHERE h.item_instance_id = NEW.item_instance_id
                  AND NOT EXISTS (SELECT 1 FROM game_item_house_reclaim_provenance p
                                   WHERE p.item_instance_id = h.item_instance_id
                                     AND p.world_id = h.world_id
                                     AND p.house_key = h.house_key
                                     AND p.placement_transaction_id = h.placed_transaction_id
                                     AND p.written_xact_id = pg_current_xact_id())) THEN
        RAISE EXCEPTION 'HouseInterior item must commit with its HousingReclaimProvenance'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A move is a delete and an insert, as for Ground. A same-house tile move
-- keeps the provenance: it takes the new row's placement transaction (the FK
-- above) with a +1 revision, and nothing else changes (§3.2).
CREATE FUNCTION game_item_house_reclaim_provenance_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.provenance_revision = OLD.provenance_revision + 1
       AND NEW.placement_transaction_id <> OLD.placement_transaction_id
       AND (NEW.item_instance_id, NEW.world_id, NEW.house_key, NEW.reclaim_subject_character_id)
         = (OLD.item_instance_id, OLD.world_id, OLD.house_key, OLD.reclaim_subject_character_id) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'HousingReclaimProvenance changes only by one revision of the same placement'
        USING ERRCODE = '23514';
END;
$$;

-- §3.3: a live item has exactly one row across every location table, a
-- retired item has none, and a HouseInterior item has no contents. Any later
-- location family joins this function in its own migration. SECURITY DEFINER:
-- the guard fires on runtime writes to the other location tables and must
-- see the house table, on which the runtime role has no grant.
CREATE FUNCTION game_item_location_exclusive(p_item UUID) RETURNS void
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
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item);
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

CREATE FUNCTION game_item_location_exclusivity_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP <> 'DELETE' THEN
        PERFORM game_item_location_exclusive(NEW.item_instance_id);
        IF TG_TABLE_NAME IN ('game_item_container_entries', 'game_item_corpse_container_entries') THEN
            PERFORM game_item_location_exclusive(NEW.parent_item_instance_id);
        END IF;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        PERFORM game_item_location_exclusive(OLD.item_instance_id);
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_instance_location_exclusive
    AFTER INSERT OR UPDATE ON game_item_instances
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_ground_location_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_ground_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_container_slot_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_container_slots
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_container_entry_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_container_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_corpse_container_entry_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_corpse_container_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_house_interior_location_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_house_interior_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_house_interior_provenance_proven
    AFTER INSERT ON game_item_house_interior_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_house_interior_provenance_proven();
CREATE CONSTRAINT TRIGGER game_item_house_reclaim_provenance_retired_proven
    AFTER DELETE ON game_item_house_reclaim_provenance
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_house_interior_provenance_proven();

CREATE TRIGGER game_item_house_interior_location_immutable BEFORE UPDATE
    ON game_item_house_interior_locations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_house_placement_record BEFORE INSERT
    ON game_item_house_interior_locations FOR EACH ROW
    EXECUTE FUNCTION game_item_house_placement_record();
CREATE TRIGGER game_item_house_placement_transaction_immutable BEFORE UPDATE OR DELETE
    ON game_item_house_placement_transactions FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_house_placement_transactions_no_truncate BEFORE TRUNCATE
    ON game_item_house_placement_transactions
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_house_reclaim_provenance_guard BEFORE UPDATE
    ON game_item_house_reclaim_provenance FOR EACH ROW
    EXECUTE FUNCTION game_item_house_reclaim_provenance_guard();
CREATE TRIGGER game_item_house_interior_locations_no_truncate BEFORE TRUNCATE
    ON game_item_house_interior_locations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_house_reclaim_provenance_no_truncate BEFORE TRUNCATE
    ON game_item_house_reclaim_provenance
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_item_house_reclaim_provenance_stamp_xact() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.written_xact_id := pg_current_xact_id();
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_item_house_reclaim_provenance_stamp_xact BEFORE INSERT OR UPDATE
    ON game_item_house_reclaim_provenance FOR EACH ROW
    EXECUTE FUNCTION game_item_house_reclaim_provenance_stamp_xact();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_house_interior_provenance_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_house_reclaim_provenance_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_location_exclusivity_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_house_reclaim_provenance_stamp_xact() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_house_placement_record() SET search_path = %I, pg_temp', current_schema());
END $$;

-- The guard does not re-check rows that already exist, so the migration
-- does it once and fails closed on any existing violation.
DO $$
DECLARE
    v_item UUID;
BEGIN
    FOR v_item IN SELECT item_instance_id FROM game_item_instances LOOP
        PERFORM game_item_location_exclusive(v_item);
    END LOOP;
END $$;

REVOKE ALL ON
    game_item_house_interior_locations,
    game_item_house_reclaim_provenance,
    game_item_house_placement_transactions
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_house_interior_provenance_proven(),
    game_item_house_reclaim_provenance_guard(),
    game_item_house_reclaim_provenance_stamp_xact(),
    game_item_house_placement_record(),
    game_item_location_exclusive(UUID),
    game_item_location_exclusivity_guard()
FROM PUBLIC;
-- §3.5: no grant to oteryn_game_runtime until the house ownership child opens
-- the house tables. Control reads them like every other item table.
GRANT SELECT ON
    game_item_house_interior_locations,
    game_item_house_reclaim_provenance,
    game_item_house_placement_transactions
TO oteryn_game_control;
