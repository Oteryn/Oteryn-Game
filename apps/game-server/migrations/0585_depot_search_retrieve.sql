-- E08 depot search retrieve-all (command 182 retrieve): one command moves every matching
-- depot or Inbox item, nested entries included, into the main backpack. Canary 52550d5e
-- src/creatures/players/player.cpp:10813 retrieveAllItemsFromDepotSearch over the items that
-- requestDepotSearchItem (:10763) finds with a ContainerIterator. Each item is one child loop
-- move receipt (batch ordinal > 0) under the command's terminal receipt below, as quick loot.

CREATE TABLE game_depot_search_retrieve_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    request BYTEA NOT NULL CHECK(octet_length(request) = 5),
    found BOOLEAN NOT NULL,
    retrieved BIGINT NOT NULL CHECK(retrieved >= 0 AND (found OR retrieved = 0)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id, command_id)
);
CREATE TRIGGER game_depot_search_retrieve_receipts_immutable BEFORE UPDATE OR DELETE
    ON game_depot_search_retrieve_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
REVOKE ALL ON game_depot_search_retrieve_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_depot_search_retrieve_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_depot_search_retrieve_receipts TO oteryn_game_control;

-- Sources 6 (an Inbox root) and 7 (an entry nested under a depot or Inbox root) leave only
-- for the carried tree, or retire into a carried stack.
DO $$
DECLARE c TEXT;
BEGIN
    SELECT conname INTO STRICT c FROM pg_constraint
     WHERE conrelid = 'game_item_loop_move_lines'::regclass AND contype = 'c'
       AND pg_get_constraintdef(oid) = 'CHECK ((from_kind = ANY (ARRAY[0, 1, 2, 3, 4, 5, 9])))';
    EXECUTE format('ALTER TABLE game_item_loop_move_lines DROP CONSTRAINT %I', c);
END $$;
ALTER TABLE game_item_loop_move_lines
    ADD CHECK(from_kind IN(0,1,2,3,4,5,6,7,9)),
    ADD CHECK(from_kind NOT IN(6,7) OR to_kind IN(0,1,2));

-- The Character whose depot or Inbox root holds the item at any nesting depth, or NULL.
-- The Store Gold Pouch is not depot custody: its coins already count as carried.
CREATE FUNCTION game_depot_search_custody_owner(p_item UUID) RETURNS UUID
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE
    v_item UUID := p_item;
    v_parent UUID;
    v_owner UUID;
BEGIN
    FOR hop IN 1..17 LOOP
        SELECT parent_item_instance_id INTO v_parent FROM game_item_nested_entries
         WHERE item_instance_id = v_item;
        IF NOT FOUND THEN
            EXIT;
        END IF;
        v_item := v_parent;
    END LOOP;
    IF v_item = p_item THEN
        RETURN NULL;
    END IF;
    SELECT character_id INTO v_owner FROM game_item_character_depot_locations
     WHERE item_instance_id = v_item;
    IF FOUND THEN
        RETURN v_owner;
    END IF;
    SELECT x.character_id INTO v_owner FROM game_item_character_inbox_locations x
      JOIN game_item_instances i USING(item_instance_id)
     WHERE x.item_instance_id = v_item AND i.definition_production_key <> 'oteryn:item.tibia.i23721';
    RETURN v_owner;
END $$;

-- Admission of the two depot-search sources, inserted ahead of the depot withdrawal arm of
-- the latest installed body (0227), so its other arms and attributes are retained.
DO $migration$
DECLARE body TEXT;
BEGIN
    body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    IF position('    IF NEW.from_kind = 4 THEN' IN body) = 0 OR position('from_kind = 6' IN body) <> 0 THEN
        RAISE EXCEPTION 'depot search requires the 0227 item loop admission';
    END IF;
    body := replace(body, '    IF NEW.from_kind = 4 THEN', $arm$    IF NEW.from_kind = 6 THEN
        IF r.quick_loot_ordinal = 0 OR NOT EXISTS (SELECT 1 FROM game_item_character_inbox_locations x
             WHERE x.item_instance_id = NEW.item_instance_id AND x.world_id = r.world_id
               AND x.character_id = r.character_id) THEN
            RAISE EXCEPTION 'depot search Inbox retrieval requires the Character Inbox item in a batch'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.from_kind = 7 THEN
        IF r.quick_loot_ordinal = 0
           OR NOT EXISTS (SELECT 1 FROM game_item_nested_entries WHERE item_instance_id = NEW.item_instance_id)
           OR game_depot_search_custody_owner(NEW.item_instance_id) IS DISTINCT FROM r.character_id THEN
            RAISE EXCEPTION 'depot search nested retrieval requires the Character depot or Inbox tree in a batch'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.from_kind = 4 THEN$arm$);
    EXECUTE body;
END $migration$;

-- A batch child is proven by either terminal receipt: quick loot or depot search.
CREATE OR REPLACE FUNCTION game_quick_loot_child_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.quick_loot_ordinal>0 AND NOT EXISTS (
        SELECT 1 FROM game_quick_loot_receipts q
        WHERE q.game_session_id=NEW.game_session_id AND q.command_id=NEW.command_id
          AND q.character_id=NEW.character_id AND q.created_xact_id=NEW.created_xact_id)
       AND NOT EXISTS (
        SELECT 1 FROM game_depot_search_retrieve_receipts q
        WHERE q.game_session_id=NEW.game_session_id AND q.command_id=NEW.command_id
          AND q.character_id=NEW.character_id AND q.created_xact_id=NEW.created_xact_id) THEN
        RAISE EXCEPTION 'quick loot child requires the terminal batch receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

-- One retrieval empties several entries of one box in one transaction: each counter update
-- queues this deferred proof, and only the event naming the counter's final state checks it
-- (0151 compared every earlier event's count with the final placements).
CREATE OR REPLACE FUNCTION game_character_depot_counter_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_next NUMERIC;
    v_committed INT;
    v_count BIGINT;
BEGIN
    SELECT next_ordinal, committed INTO v_next, v_committed FROM game_character_depot_box_counters
     WHERE character_id = NEW.character_id AND town_id = NEW.town_id
       AND box_number = NEW.box_number;
    IF v_next <> NEW.next_ordinal OR v_committed <> NEW.committed THEN
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

-- An Inbox row leaves only with a current depot-search line for its exact Character and World.
CREATE FUNCTION game_character_inbox_remove() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    PERFORM 1 FROM game_character_roots WHERE character_id = OLD.character_id FOR UPDATE;
    IF NOT EXISTS (SELECT 1 FROM game_item_loop_move_lines l
        JOIN game_item_loop_move_receipts r USING (transaction_id)
        WHERE l.item_instance_id = OLD.item_instance_id AND l.from_kind = 6
          AND l.created_xact_id = pg_current_xact_id() AND r.created_xact_id = pg_current_xact_id()
          AND r.character_id = OLD.character_id AND r.world_id = OLD.world_id
          AND r.quick_loot_ordinal > 0) THEN
        RAISE EXCEPTION 'Inbox removal requires its current depot search line' USING ERRCODE = '23514';
    END IF;
    RETURN OLD;
END $$;
DROP TRIGGER game_item_character_inbox_location_immutable ON game_item_character_inbox_locations;
CREATE TRIGGER game_item_character_inbox_location_immutable BEFORE UPDATE
    ON game_item_character_inbox_locations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_inbox_remove BEFORE DELETE
    ON game_item_character_inbox_locations FOR EACH ROW EXECUTE FUNCTION game_character_inbox_remove();
-- The Inbox stays closed to runtime DML: the item loop removes a retrieved root through this
-- one narrow seam, and the BEFORE DELETE proof above still requires its current line.
CREATE FUNCTION game_depot_search_inbox_remove(p_item UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    DELETE FROM game_item_character_inbox_locations WHERE item_instance_id = p_item;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'depot search Inbox removal requires the Inbox row' USING ERRCODE = '23514';
    END IF;
END $$;

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_quick_loot_child_proven','game_character_inbox_remove',
                            'game_character_depot_counter_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_depot_search_custody_owner(UUID) SET search_path=pg_catalog,%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_depot_search_inbox_remove(UUID) SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_depot_search_custody_owner(UUID), game_depot_search_inbox_remove(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_depot_search_custody_owner(UUID), game_depot_search_inbox_remove(UUID)
    TO oteryn_game_runtime;
