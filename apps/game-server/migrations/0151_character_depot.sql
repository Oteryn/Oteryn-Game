-- DEPOT-1 storage foundation. Owner task 2026-10-23 overrides DEPOT-0's
-- town sharing: custody is Character + World + town, never Channel.
-- DEPOT-CONTENT-1 / DEPOT-WIRE-1 are absent. As in HOUSE-CUSTODY-1,
-- this migration grants no runtime placement/removal path. Their fenced
-- TRANSFER/audit proof must precede any grant in a later migration.
-- Inbox remains the single existing CharacterInbox owner (0076).

CREATE TABLE game_item_character_depot_locations (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    character_id UUID NOT NULL
        REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
    town_id INTEGER NOT NULL CHECK (town_id BETWEEN 0 AND 65535),
    box_number SMALLINT NOT NULL CHECK (box_number BETWEEN 1 AND 17),
    ordinal NUMERIC(20,0) NOT NULL
        CHECK (ordinal BETWEEN 1 AND 18446744073709551615),
    UNIQUE (character_id, town_id, box_number, ordinal),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances(item_instance_id, world_id)
);

-- Implicit empty boxes require no rows. A retained high-water mark prevents
-- ordinal reuse when a later, separately proven out-shape permits removal.
CREATE TABLE game_character_depot_box_counters (
    character_id UUID NOT NULL
        REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
    town_id INTEGER NOT NULL CHECK (town_id BETWEEN 0 AND 65535),
    box_number SMALLINT NOT NULL CHECK (box_number BETWEEN 1 AND 17),
    next_ordinal NUMERIC(20,0) NOT NULL
        CHECK (next_ordinal BETWEEN 2 AND 18446744073709551615),
    committed INTEGER NOT NULL CHECK (committed BETWEEN 1 AND 15000),
    PRIMARY KEY (character_id, town_id, box_number)
);

-- Lock order is Character root, item, counter. No item-only or caller-supplied
-- CharacterId is a gameplay fence; runtime cannot call or write this seam.
CREATE FUNCTION game_character_depot_insert() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_world UUID;
    v_ordinal NUMERIC;
    v_count BIGINT;
BEGIN
    SELECT world_id INTO v_world FROM game_character_roots
     WHERE character_id = NEW.character_id FOR UPDATE;
    IF NOT FOUND OR v_world <> NEW.world_id THEN
        RAISE EXCEPTION 'CharacterDepot Character World mismatch' USING ERRCODE = '23514';
    END IF;
    PERFORM 1 FROM game_item_instances
     WHERE item_instance_id = NEW.item_instance_id AND world_id = NEW.world_id
       AND lifecycle = 1 FOR NO KEY UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'CharacterDepot requires a live item of its World' USING ERRCODE = '23514';
    END IF;

    INSERT INTO game_character_depot_box_counters
        (character_id, town_id, box_number, next_ordinal, committed)
    VALUES (NEW.character_id, NEW.town_id, NEW.box_number, 2, 1)
    ON CONFLICT (character_id, town_id, box_number) DO UPDATE
       SET next_ordinal = game_character_depot_box_counters.next_ordinal + 1,
           committed = game_character_depot_box_counters.committed + 1
    RETURNING next_ordinal - 1 INTO v_ordinal;
    IF NEW.ordinal IS NOT NULL AND NEW.ordinal <> v_ordinal THEN
        RAISE EXCEPTION 'CharacterDepot ordinal must follow its box high-water mark'
            USING ERRCODE = '23514';
    END IF;
    NEW.ordinal := v_ordinal;

    -- At most seventeen counter rows; stacks count as one entry, never units.
    -- This is the database ceiling. A future runtime writer must additionally
    -- admit 2,000 Free / 15,000 current Premium entries before placement.
    SELECT sum(committed) INTO v_count FROM game_character_depot_box_counters
     WHERE character_id = NEW.character_id AND town_id = NEW.town_id;
    IF v_count > 15000 THEN
        RAISE EXCEPTION 'CharacterDepot capacity exceeded' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- Only the insertion trigger advances a counter, by exactly one. No runtime
-- grant exists; the shape guard also catches accidental owner-side edits.
CREATE FUNCTION game_character_depot_counter_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF pg_trigger_depth() > 1 THEN
        IF TG_OP = 'INSERT' AND NEW.committed = 1 AND NEW.next_ordinal = 2 THEN
            RETURN NEW;
        END IF;
        IF TG_OP = 'UPDATE'
           AND (NEW.character_id, NEW.town_id, NEW.box_number)
             = (OLD.character_id, OLD.town_id, OLD.box_number)
           AND NEW.committed = OLD.committed + 1
           AND NEW.next_ordinal = OLD.next_ordinal + 1 THEN
            RETURN NEW;
        END IF;
    END IF;
    RAISE EXCEPTION 'CharacterDepot counter changes only with placement'
        USING ERRCODE = '23514';
END;
$$;

-- BEFORE INSERT also runs when ON CONFLICT later skips the location. Prove
-- the final counter against actual custody, so such attempts cannot consume
-- capacity or ordinals without a placement. Earlier events for the same box
-- are superseded; only its final event performs the bounded count.
CREATE FUNCTION game_character_depot_counter_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_next NUMERIC;
    v_count BIGINT;
BEGIN
    SELECT next_ordinal INTO v_next FROM game_character_depot_box_counters
     WHERE character_id = NEW.character_id AND town_id = NEW.town_id
       AND box_number = NEW.box_number;
    IF v_next <> NEW.next_ordinal THEN
        RETURN NULL;
    END IF;
    SELECT count(*) INTO v_count FROM (
        SELECT 1 FROM game_item_character_depot_locations
         WHERE character_id = NEW.character_id AND town_id = NEW.town_id
           AND box_number = NEW.box_number LIMIT 15001
    ) AS entries;
    IF v_count <> NEW.committed THEN
        RAISE EXCEPTION 'CharacterDepot counter requires matching placements'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Latest union is 0128, including nested bags and their retired-parent check.
-- Every existing custody arm and contents restriction is retained.
CREATE OR REPLACE FUNCTION game_item_location_exclusive(p_item UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_lifecycle SMALLINT;
    v_locations BIGINT;
BEGIN
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
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id = p_item);
    IF v_locations <> (v_lifecycle = 1)::INT THEN
        RAISE EXCEPTION 'DUR-03 item must have exactly one location while live and none when retired'
            USING ERRCODE = '23514';
    END IF;
    IF v_lifecycle <> 1 AND EXISTS (SELECT 1 FROM game_item_nested_entries
                                     WHERE parent_item_instance_id = p_item) THEN
        RAISE EXCEPTION 'a retired item cannot hold nested contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'HouseInterior item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterInbox item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterDepot item cannot have contents' USING ERRCODE = '23514';
    END IF;
END;
$$;

CREATE TRIGGER game_character_depot_insert BEFORE INSERT
    ON game_item_character_depot_locations FOR EACH ROW
    EXECUTE FUNCTION game_character_depot_insert();
CREATE TRIGGER game_character_depot_counter_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_depot_box_counters FOR EACH ROW
    EXECUTE FUNCTION game_character_depot_counter_guard();
CREATE CONSTRAINT TRIGGER game_character_depot_counter_proven
    AFTER INSERT OR UPDATE ON game_character_depot_box_counters
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_depot_counter_proven();
CREATE CONSTRAINT TRIGGER game_character_depot_location_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_character_depot_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE TRIGGER game_character_depot_location_immutable BEFORE UPDATE OR DELETE
    ON game_item_character_depot_locations FOR EACH ROW
    EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_depot_locations_no_truncate BEFORE TRUNCATE
    ON game_item_character_depot_locations FOR EACH STATEMENT
    EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_character_depot_counters_no_truncate BEFORE TRUNCATE
    ON game_character_depot_box_counters FOR EACH STATEMENT
    EXECUTE FUNCTION game_item_reject_truncate();

DO $$
DECLARE n TEXT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_character_depot_insert', 'game_character_depot_counter_guard',
                            'game_character_depot_counter_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path = %I, pg_temp', current_schema());
END;
$$;
REVOKE ALL ON FUNCTION game_item_location_exclusive(UUID) FROM PUBLIC;
REVOKE ALL ON game_item_character_depot_locations, game_character_depot_box_counters
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
-- No columns are written by runtime in this foundation. Future write grants
-- must name every column of the corresponding fenced TRANSFER shape.
GRANT SELECT ON game_item_character_depot_locations, game_character_depot_box_counters
    TO oteryn_game_runtime, oteryn_game_control;
