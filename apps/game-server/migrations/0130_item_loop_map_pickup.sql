-- MAP-PICKUP-1: admit the unchanged Ground row of a proven map-item MINT.
-- Loop-drop identity and its partial index keep their original meaning. Map-MINT
-- stack_ordinal is the positive append ordinal assigned by the existing Ground
-- INSERT trigger (0034); it is not the base placement's top-level ordinal.
CREATE FUNCTION game_item_loop_map_mint_source(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_map_item_mint_receipts m
        JOIN game_item_instances i USING (item_instance_id)
        JOIN game_item_ground_locations g USING (item_instance_id)
        WHERE m.item_instance_id = p_item
          AND i.world_id = m.world_id AND g.world_id = m.world_id
          AND g.channel_id = m.channel_id
          AND i.lifecycle = 1 AND i.state_revision = 1
          AND i.last_transaction_id IS NULL
          AND i.minted_transaction_id = m.transaction_id AND i.quantity = m.quantity
          AND (i.definition_family, i.definition_production_key, i.definition_revision_ref)
              = (m.definition_family, m.definition_production_key, m.definition_revision_ref)
          AND g.runtime_scope_ownership_generation = m.runtime_scope_ownership_generation
          AND g.spatial_position = m.spatial_position
          AND g.corpse_ref = convert_to('map-item-materialization-v1', 'UTF8')
          AND g.map_revision = m.map_revision AND g.content_revision = m.content_revision
          AND g.native_room_placement_context = m.base_bundle_digest
          AND g.stack_ordinal > 0)
$$;

-- Replace only the immediate source admission; the deferred final-state and
-- removal proofs from 0128 already prove custody for both source kinds.
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

-- Follow the later-function replacement precedent (0075): restore pinned paths
-- after CREATE OR REPLACE. The original admission function's owner is retained;
-- the new helper has the same migration owner and no PUBLIC execution grant.
DO $$ DECLARE f TEXT; BEGIN
    FOREACH f IN ARRAY ARRAY['game_item_loop_map_mint_source', 'game_item_loop_line_admit'] LOOP
        IF f = 'game_item_loop_map_mint_source' THEN
            EXECUTE format('ALTER FUNCTION %I(UUID) SET search_path = pg_catalog, %I, pg_temp', f, current_schema());
        ELSE
            EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp', f, current_schema());
        END IF;
    END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_item_loop_map_mint_source(UUID), game_item_loop_line_admit() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_loop_map_mint_source(UUID) TO oteryn_game_runtime;
