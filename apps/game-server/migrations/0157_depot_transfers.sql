-- DEPOT-1 playable continuation: kind 4 is private town/box custody.
-- Existing item-loop fencing, receipt identity, conservation and all previous
-- location proofs remain active. Empty items and whole stacks only.
ALTER TABLE game_character_depot_box_counters
    DROP CONSTRAINT game_character_depot_box_counters_committed_check,
    ADD CHECK (committed BETWEEN 0 AND 15000);
ALTER TABLE game_item_loop_move_receipts
    ADD COLUMN depot_town INTEGER CHECK (depot_town BETWEEN 0 AND 65535),
    ADD COLUMN depot_box SMALLINT CHECK (depot_box BETWEEN 1 AND 17),
    ADD CHECK ((depot_town IS NULL) = (depot_box IS NULL));
ALTER TABLE game_item_loop_move_lines
    DROP CONSTRAINT game_item_loop_move_lines_from_kind_check,
    DROP CONSTRAINT game_item_loop_move_lines_to_kind_check,
    ADD CHECK (from_kind IN (0,1,2,3,4,9)),
    ADD CHECK (to_kind IN (0,1,2,3,4,9)),
    ADD CHECK (from_kind <> 4 OR to_kind IN (0,1)),
    ADD CHECK (to_kind <> 4 OR from_kind IN (1,2));

CREATE OR REPLACE FUNCTION game_item_loop_line_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    r game_item_loop_move_receipts%ROWTYPE;
    i game_item_instances%ROWTYPE;
    g game_item_ground_locations%ROWTYPE;
    v_owner UUID;
    v_depth INT;
BEGIN
    SELECT * INTO r FROM game_item_loop_move_receipts
     WHERE transaction_id = NEW.transaction_id AND created_xact_id = pg_current_xact_id();
    IF NOT FOUND OR NEW.created_xact_id <> pg_current_xact_id() THEN
        RAISE EXCEPTION 'item loop line requires its receipt of this transaction' USING ERRCODE = '23514';
    END IF;
    IF NEW.from_kind = 0 THEN
        IF EXISTS (SELECT 1 FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id) THEN
            RAISE EXCEPTION 'item loop fresh item already exists' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    IF NOT FOUND OR i.lifecycle <> 1 OR i.world_id <> r.world_id
       OR i.quantity <> NEW.quantity_before OR i.state_revision <> NEW.state_revision_before
       OR (i.definition_family, i.definition_production_key, i.definition_revision_ref)
          <> (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref) THEN
        RAISE EXCEPTION 'item loop line must name the exact current item' USING ERRCODE = '23514';
    END IF;
    -- The Game node hosts no timed or temporal lane: such an item never moves through the loop.
    IF EXISTS (SELECT 1 FROM game_item_timed_states WHERE item_instance_id = NEW.item_instance_id)
       OR EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)
       OR EXISTS (SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'item loop timed item not hosted' USING ERRCODE = '23514';
    END IF;
    IF NEW.from_kind = 4 THEN
        IF NOT EXISTS (SELECT 1 FROM game_item_character_depot_locations d
             WHERE d.item_instance_id = NEW.item_instance_id AND d.world_id = r.world_id
               AND d.character_id = r.character_id AND d.town_id = r.depot_town
               AND d.box_number = r.depot_box) THEN
            RAISE EXCEPTION 'depot withdrawal requires the exact Character town and box'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.from_kind = 3 THEN
        SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR g.world_id <> r.world_id OR g.channel_id <> r.channel_id
           OR NOT (game_item_loop_drop_ref(g.corpse_ref)
                   OR game_item_loop_map_mint_source(NEW.item_instance_id)) THEN
            RAISE EXCEPTION 'item loop pickup source is not a loop Ground item of this Channel'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    SELECT o.owner_character, o.depth INTO v_owner, v_depth FROM game_item_loop_owner(NEW.item_instance_id) o;
    IF v_owner IS DISTINCT FROM r.character_id OR v_depth < 1
       OR (NEW.from_kind = 1 AND NOT EXISTS (SELECT 1 FROM game_item_container_entries
            WHERE item_instance_id = NEW.item_instance_id AND character_id = r.character_id))
       OR (NEW.from_kind = 2 AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries
            WHERE item_instance_id = NEW.item_instance_id)) THEN
        RAISE EXCEPTION 'item loop source is not in the Character tree' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_loop_line_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    r game_item_loop_move_receipts%ROWTYPE;
    i game_item_instances%ROWTYPE;
    e game_item_container_entries%ROWTYPE;
    n game_item_nested_entries%ROWTYPE;
    g game_item_ground_locations%ROWTYPE;
    v_owner UUID;
    v_depth INT;
    v_height INT;
    v_size BIGINT;
BEGIN
    SELECT * INTO STRICT r FROM game_item_loop_move_receipts WHERE transaction_id = NEW.transaction_id;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    IF NOT FOUND OR i.world_id <> r.world_id
       OR (i.definition_family, i.definition_production_key, i.definition_revision_ref)
          <> (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref) THEN
        RAISE EXCEPTION 'item loop line item is missing' USING ERRCODE = '23514';
    END IF;
    IF NEW.from_kind = 0 THEN
        IF i.minted_transaction_id <> NEW.transaction_id OR i.last_transaction_id IS NOT NULL
           OR i.state_revision <> 1 THEN
            RAISE EXCEPTION 'item loop fresh item must be minted by its receipt' USING ERRCODE = '23514';
        END IF;
    ELSIF i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
          OR i.state_revision <> NEW.state_revision_before + 1 THEN
        RAISE EXCEPTION 'item loop item must change exactly once in its receipt' USING ERRCODE = '23514';
    END IF;
    IF NEW.to_kind = 0 THEN
        IF i.lifecycle <> 2 OR i.quantity <> 0 THEN
            RAISE EXCEPTION 'item loop retired item must be retired' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF i.lifecycle <> 1 OR i.quantity <> NEW.quantity_after OR NEW.quantity_after > 100 THEN
        RAISE EXCEPTION 'item loop item quantity differs from its line' USING ERRCODE = '23514';
    END IF;
    IF NEW.to_kind = 4 THEN
        IF NOT EXISTS (SELECT 1 FROM game_item_character_depot_locations d
             WHERE d.item_instance_id = NEW.item_instance_id AND d.world_id = r.world_id
               AND d.character_id = r.character_id AND d.town_id = r.depot_town
               AND d.box_number = r.depot_box)
           OR NEW.from_kind NOT IN (1, 2) THEN
            RAISE EXCEPTION 'depot deposit differs from receipt' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF NEW.to_kind = 1 THEN
        SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR e.character_id <> r.character_id OR e.world_id <> r.world_id
           OR e.parent_item_instance_id <> NEW.to_parent_item_instance_id
           OR e.placed_transaction_id <> NEW.transaction_id
           OR NOT EXISTS (SELECT 1 FROM game_item_container_slots s WHERE s.character_id = r.character_id
                AND s.item_instance_id = NEW.to_parent_item_instance_id)
           OR (SELECT count(*) FROM game_item_container_entries
                WHERE parent_item_instance_id = NEW.to_parent_item_instance_id) > NEW.to_capacity THEN
            RAISE EXCEPTION 'item loop main-backpack destination differs from its line' USING ERRCODE = '23514';
        END IF;
        v_depth := 0;
    ELSIF NEW.to_kind = 2 THEN
        SELECT * INTO n FROM game_item_nested_entries WHERE item_instance_id = NEW.item_instance_id;
        SELECT o.owner_character, o.depth INTO v_owner, v_depth
          FROM game_item_loop_owner(NEW.to_parent_item_instance_id) o;
        IF NOT FOUND OR n.item_instance_id IS NULL OR n.world_id <> r.world_id
           OR n.parent_item_instance_id <> NEW.to_parent_item_instance_id
           OR n.placed_transaction_id <> NEW.transaction_id
           OR v_owner IS DISTINCT FROM r.character_id OR v_depth < 1
           OR (SELECT count(*) FROM game_item_nested_entries
                WHERE parent_item_instance_id = NEW.to_parent_item_instance_id) > NEW.to_capacity THEN
            RAISE EXCEPTION 'item loop container destination differs from its line' USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.to_kind = 3 THEN
        SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR g.world_id <> r.world_id OR g.channel_id <> r.channel_id
           OR g.runtime_scope_ownership_generation <> r.scope_generation
           OR g.spatial_position <> NEW.to_spatial_position OR NOT game_item_loop_drop_ref(g.corpse_ref)
           OR (SELECT count(*) FROM game_item_ground_locations x WHERE x.world_id = g.world_id
                AND x.channel_id = g.channel_id AND x.spatial_position = g.spatial_position) > 64 THEN
            RAISE EXCEPTION 'item loop Ground destination differs from its line' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    ELSE
        SELECT o.owner_character, o.depth INTO v_owner, v_depth FROM game_item_loop_owner(NEW.item_instance_id) o;
        IF v_owner IS DISTINCT FROM r.character_id OR v_depth < 1
           OR EXISTS (SELECT 1 FROM game_item_container_entries WHERE item_instance_id = NEW.item_instance_id
                AND placed_transaction_id = NEW.transaction_id)
           OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE item_instance_id = NEW.item_instance_id
                AND placed_transaction_id = NEW.transaction_id) THEN
            RAISE EXCEPTION 'item loop unchanged item moved' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    -- BAGS-1 ceilings: depth <= 8 for every descendant, <= 500 items in the carried tree.
    SELECT t.height, t.size INTO v_height, v_size FROM game_item_loop_subtree(NEW.item_instance_id) t;
    IF v_depth + 1 + v_height > 8
       OR game_item_loop_tree_size(r.character_id) > 500 THEN
        RAISE EXCEPTION 'item loop container tree exceeds its ceilings' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_character_depot_counter_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF pg_trigger_depth() > 1 THEN
        IF TG_OP = 'INSERT' AND NEW.committed = 1 AND NEW.next_ordinal = 2 THEN
            RETURN NEW;
        END IF;
        IF TG_OP = 'UPDATE'
           AND (NEW.character_id, NEW.town_id, NEW.box_number)
             = (OLD.character_id, OLD.town_id, OLD.box_number)
           AND NEW.committed = OLD.committed - 1 AND NEW.committed >= 0
           AND NEW.next_ordinal = OLD.next_ordinal THEN
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

-- Removal is enabled only by a line admitted against this exact source before
-- custody changes. The line's deferred final-state proof covers the destination.
CREATE FUNCTION game_character_depot_remove() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    PERFORM 1 FROM game_character_roots WHERE character_id = OLD.character_id FOR UPDATE;
    IF NOT EXISTS (SELECT 1 FROM game_item_loop_move_lines l
        JOIN game_item_loop_move_receipts r USING (transaction_id)
        WHERE l.item_instance_id = OLD.item_instance_id AND l.from_kind = 4
          AND l.created_xact_id = pg_current_xact_id() AND r.created_xact_id = pg_current_xact_id()
          AND r.character_id = OLD.character_id AND r.world_id = OLD.world_id
          AND r.depot_town = OLD.town_id AND r.depot_box = OLD.box_number) THEN
        RAISE EXCEPTION 'depot removal requires its current transfer line' USING ERRCODE = '23514';
    END IF;
    UPDATE game_character_depot_box_counters SET committed = committed - 1
     WHERE character_id = OLD.character_id AND town_id = OLD.town_id AND box_number = OLD.box_number;
    RETURN OLD;
END;
$$;

-- Owner fixtures may seed the dormant storage foundation. Runtime inserts require
-- the current item-loop line; this invoker trigger cannot inherit definer rights.
CREATE FUNCTION game_character_depot_placement_admit() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT game_item_loop_line_current(NEW.item_instance_id)
       AND current_user <> pg_get_userbyid((SELECT relowner FROM pg_class WHERE oid = TG_RELID)) THEN
        RAISE EXCEPTION 'depot placement requires its current transfer line' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER game_character_depot_location_immutable ON game_item_character_depot_locations;
CREATE TRIGGER game_character_depot_location_immutable BEFORE UPDATE
    ON game_item_character_depot_locations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_depot_remove BEFORE DELETE
    ON game_item_character_depot_locations FOR EACH ROW EXECUTE FUNCTION game_character_depot_remove();
CREATE TRIGGER game_character_depot_placement_admit BEFORE INSERT
    ON game_item_character_depot_locations FOR EACH ROW EXECUTE FUNCTION game_character_depot_placement_admit();

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_loop_line_admit','game_item_loop_line_proven',
        'game_character_depot_counter_guard','game_character_depot_remove','game_character_depot_placement_admit'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
END;
$$;
GRANT INSERT (item_instance_id,world_id,character_id,town_id,box_number), DELETE
    ON game_item_character_depot_locations TO oteryn_game_runtime;
GRANT INSERT (depot_town,depot_box) ON game_item_loop_move_receipts TO oteryn_game_runtime;
