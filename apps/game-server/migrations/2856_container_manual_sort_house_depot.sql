-- CONTAINER-MANUAL-SORT-1 (capability 510): a slotted move may also place its entry in a
-- Named-House container or a depot box. The container sort receipt (0299) keeps its exact
-- permutation, conservation and successor proofs. Its custody arm admits, besides carried
-- custody, only:
--   * a House entry whose House root is the House of the fenced House loop proof that the same
--     command wrote in this transaction (game_house_container_loop_proofs, 1169), under the
--     receipt's exact session, connection, lease and scope generations;
--   * a depot box entry of the receipt's Character, in the town and box of that command's own
--     depot loop move receipt written in this transaction.
-- Depot entries are permutation elements with a "depot" object {town, box} and no parent or
-- nested flag; only their ordinal changes, and only as the receipt states.

-- Preserve depot ordinal uniqueness at COMMIT while permitting a rotation in one UPDATE.
DO $$ DECLARE c TEXT; BEGIN
    SELECT conname INTO STRICT c FROM pg_constraint
    WHERE conrelid='game_item_character_depot_locations'::regclass AND contype='u'
      AND pg_get_constraintdef(oid)='UNIQUE (character_id, town_id, box_number, ordinal)';
    EXECUTE format('ALTER TABLE game_item_character_depot_locations DROP CONSTRAINT %I',c);
END $$;
ALTER TABLE game_item_character_depot_locations
    ADD CONSTRAINT game_item_character_depot_locations_sort_ordinals
    UNIQUE(character_id,town_id,box_number,ordinal) DEFERRABLE INITIALLY IMMEDIATE;

CREATE OR REPLACE FUNCTION game_item_container_sort_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE v_entry JSONB; v_count INT; BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'sort receipt must be current' USING ERRCODE='23514';
    END IF;
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=NEW.world_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'sort Character world' USING ERRCODE='23514'; END IF;
    IF (SELECT count(DISTINCT x->>'item') FROM jsonb_array_elements(NEW.permutation) x)<>jsonb_array_length(NEW.permutation) THEN
        RAISE EXCEPTION 'sort repeated item' USING ERRCODE='23514';
    END IF;
    FOR v_entry IN SELECT * FROM jsonb_array_elements(NEW.permutation) LOOP
        IF v_entry ? 'depot' THEN
            IF v_entry ? 'parent' OR v_entry ? 'nested' OR NOT EXISTS (
                SELECT 1 FROM game_item_loop_move_receipts r
                WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
                  AND r.created_xact_id=pg_current_xact_id() AND r.character_id=NEW.character_id
                  AND r.world_id=NEW.world_id
                  AND r.depot_town=(v_entry->'depot'->>'town')::int
                  AND r.depot_box=(v_entry->'depot'->>'box')::int
            ) THEN
                RAISE EXCEPTION 'depot sort requires its current depot move' USING ERRCODE='23514';
            END IF;
            SELECT count(*) INTO v_count FROM game_item_character_depot_locations d
            WHERE d.item_instance_id=(v_entry->>'item')::uuid AND d.world_id=NEW.world_id
              AND d.character_id=NEW.character_id
              AND d.town_id=(v_entry->'depot'->>'town')::int
              AND d.box_number=(v_entry->'depot'->>'box')::int
              AND d.ordinal=(v_entry->>'before')::numeric;
            IF v_count<>1 THEN RAISE EXCEPTION 'sort source changed' USING ERRCODE='23514'; END IF;
            CONTINUE;
        END IF;
        IF (SELECT owner_character FROM game_item_loop_owner((v_entry->>'item')::uuid)) IS DISTINCT FROM NEW.character_id
           AND NOT EXISTS (
            SELECT 1 FROM game_house_container_root((v_entry->>'item')::uuid) h
            JOIN game_house_container_loop_proofs p ON p.world_id=h.world AND p.house_key=h.house
            JOIN game_item_loop_move_receipts r ON r.transaction_id=p.transaction_id
            WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
              AND r.created_xact_id=pg_current_xact_id() AND p.created_xact_id=pg_current_xact_id()
              AND p.game_session_id=NEW.game_session_id AND p.character_id=NEW.character_id
              AND p.connection_generation=NEW.connection_generation
              AND p.lease_generation=NEW.lease_generation AND p.scope_generation=NEW.scope_generation
              AND h.world=NEW.world_id
        ) THEN
            RAISE EXCEPTION 'sort requires carried custody' USING ERRCODE='23514';
        END IF;
        SELECT count(*) INTO v_count FROM (
            SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested FROM game_item_container_entries
            UNION ALL
            SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries
        ) e WHERE e.item_instance_id=(v_entry->>'item')::uuid AND e.world_id=NEW.world_id
          AND e.parent_item_instance_id=(v_entry->>'parent')::uuid
          AND e.placement_ordinal=(v_entry->>'before')::numeric AND e.nested=(v_entry->>'nested')::boolean;
        IF v_count<>1 THEN RAISE EXCEPTION 'sort source changed' USING ERRCODE='23514'; END IF;
    END LOOP;
    -- Each parent's (or depot box's) existing ordinal multiset must equal its successor multiset.
    IF EXISTS (
        (SELECT coalesce(x->>'parent',x->>'depot'),x->>'before' FROM jsonb_array_elements(NEW.permutation) x
         EXCEPT ALL SELECT coalesce(x->>'parent',x->>'depot'),x->>'after' FROM jsonb_array_elements(NEW.permutation) x)
        UNION ALL
        (SELECT coalesce(x->>'parent',x->>'depot'),x->>'after' FROM jsonb_array_elements(NEW.permutation) x
         EXCEPT ALL SELECT coalesce(x->>'parent',x->>'depot'),x->>'before' FROM jsonb_array_elements(NEW.permutation) x)
    ) THEN RAISE EXCEPTION 'sort must conserve ordinals' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_item_container_sort_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE v_entry JSONB; BEGIN
    FOR v_entry IN SELECT * FROM jsonb_array_elements(NEW.permutation) LOOP
        IF v_entry ? 'depot' THEN
            IF NOT EXISTS (
                SELECT 1 FROM game_item_character_depot_locations d
                WHERE d.item_instance_id=(v_entry->>'item')::uuid AND d.character_id=NEW.character_id
                  AND d.town_id=(v_entry->'depot'->>'town')::int
                  AND d.box_number=(v_entry->'depot'->>'box')::int
                  AND d.ordinal=(v_entry->>'after')::numeric
            ) THEN RAISE EXCEPTION 'sort successor differs from receipt' USING ERRCODE='23514'; END IF;
            CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM (
                SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_container_entries
                UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries
            ) e WHERE item_instance_id=(v_entry->>'item')::uuid AND parent_item_instance_id=(v_entry->>'parent')::uuid
                AND placement_ordinal=(v_entry->>'after')::numeric
        ) THEN RAISE EXCEPTION 'sort successor differs from receipt' USING ERRCODE='23514'; END IF;
    END LOOP;
    RETURN NULL;
END $$;

-- A depot location changes only its ordinal, and only as a current sort receipt states.
CREATE FUNCTION game_character_depot_location_sort_update() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
    IF (to_jsonb(NEW)-'ordinal') IS DISTINCT FROM (to_jsonb(OLD)-'ordinal')
       OR NOT EXISTS (
        SELECT 1 FROM game_item_container_sort_receipts r, jsonb_array_elements(r.permutation) x
        WHERE r.created_xact_id=pg_current_xact_id() AND r.world_id=OLD.world_id
          AND r.character_id=OLD.character_id AND x ? 'depot'
          AND x->>'item'=OLD.item_instance_id::text
          AND (x->'depot'->>'town')::int=OLD.town_id AND (x->'depot'->>'box')::int=OLD.box_number
          AND (x->>'before')::numeric=OLD.ordinal AND (x->>'after')::numeric=NEW.ordinal
    ) THEN RAISE EXCEPTION 'depot ordinal update requires exact sort receipt' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
DROP TRIGGER game_character_depot_location_immutable ON game_item_character_depot_locations;
CREATE TRIGGER game_character_depot_location_immutable BEFORE UPDATE
    ON game_item_character_depot_locations FOR EACH ROW
    EXECUTE FUNCTION game_character_depot_location_sort_update();

GRANT UPDATE(ordinal) ON game_item_character_depot_locations TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_item_container_sort_admit(),game_item_container_sort_proven(),
    game_character_depot_location_sort_update() FROM PUBLIC;
