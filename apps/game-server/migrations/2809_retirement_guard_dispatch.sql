-- MAX501: dispatch the existing terminal Item checks without walking receipt
-- families that cannot match. This is an invocation-local classification, not
-- an authority proof: every deferred World/Activity proof still runs.
DO $migration$
DECLARE
    definition TEXT;
    source TEXT;
    anchor TEXT := $anchor$    IF TG_OP = 'UPDATE' THEN
        v_instance_guard_old_json := to_jsonb(OLD);
        v_instance_guard_new_json := to_jsonb(NEW);
    END IF;$anchor$;
    terminal_arm TEXT := $arm$
    IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND OLD.quantity>0
       AND NEW.lifecycle=2 AND NEW.quantity=0
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
           =(v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
       AND EXISTS(SELECT 1 FROM game_item_decay_retire_reservations r
           WHERE r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
             AND r.transaction_id=NEW.last_transaction_id AND r.quantity_before=OLD.quantity
             AND r.cause_kind IN('WORLD_RESET','ACTIVITY_RETIREMENT'))
       AND NOT game_starting_item_current(NEW.item_instance_id)
       AND NOT EXISTS(
        -- Presence is intentionally broader than each original receipt arm.
        -- Any candidate falls through to the untouched installed guard.
        SELECT 1 FROM game_item_imbuement_records WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_house_disposition_wrap_records WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_item_marriage_records WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_carried_step_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_floor_transform_expiry_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_floor_transform_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_floor_assembly_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_house_timed_decoration_expiries WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_item_consume_transform_records WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_bazaar_item_transitions WHERE transaction_id=NEW.last_transaction_id
        -- Craft also covers the skinning JOIN; timed writes cover House remains.
        UNION ALL SELECT 1 FROM game_item_craft_records WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_item_write_once_transforms WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_item_timed_state_writes WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_map_seed_drop_records WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_forge_operations WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_house_furniture_wrap_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_market_item_lines WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_character_stash_item_lines WHERE transaction_id=NEW.last_transaction_id
        -- The trade receipt joins its header on occurrence_id.
        UNION ALL SELECT 1 FROM game_player_trades WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_item_house_shape_proofs WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_spell_field_temporal_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_spell_item_temporal_receipts WHERE transaction_id=NEW.last_transaction_id
        -- These earlier arms do not require the new transaction identifier.
        UNION ALL SELECT 1 FROM game_house_furniture_destroy_receipts
            WHERE target=OLD.item_instance_id AND created_xact_id=pg_current_xact_id()
        UNION ALL SELECT 1 FROM game_mail_operations
            WHERE root_item_instance_id=OLD.item_instance_id AND created_xact_id=pg_current_xact_id()
        UNION ALL SELECT 1 FROM game_quest_callback_consumptions
            WHERE item_instance_id=OLD.item_instance_id AND created_xact_id=pg_current_xact_id()
       ) THEN
        -- Exactly the shared terminal arm's mutation and evidence.
        NEW.state_revision := OLD.state_revision + 1;
        INSERT INTO game_item_transfer_quantity_evidence
            (item_instance_id,transaction_id,quantity_before)
        VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
        RETURN NEW;
    END IF;
$arm$;
BEGIN
    SELECT pg_get_functiondef(p.oid),p.prosrc INTO definition,source
      FROM pg_proc p WHERE p.oid='game_item_instance_guard()'::regprocedure;
    -- The selector inventory covers this complete installed body, including
    -- both 0880 prepends and both 0884 predicate extensions. A changed chain
    -- must be reviewed before this specialization can be installed.
    IF encode(sha256(convert_to(source,'UTF8')),'hex')
           <> '68d7197f98a8e20ed1fc3ff4985a569c50ec057d0b9ff2c50448db3d73952482'
       OR (length(definition)-length(replace(definition,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected installed terminal Item guard';
    END IF;
    EXECUTE replace(definition,anchor,anchor||terminal_arm);
END $migration$;

-- A retired Item needs zero locations. An EXISTS over the same complete
-- custody union proves that directly, even without assuming uniqueness.
-- The locked lifecycle read and all live-Item behavior remain unchanged.
DO $migration$
DECLARE
    definition TEXT;
    source TEXT;
    anchor TEXT := '    v_locations := (SELECT count(*) FROM game_item_market_escrow_locations WHERE item_instance_id=p_item) +';
    terminal_arm TEXT := $arm$
    IF v_lifecycle<>1 THEN
        IF EXISTS(
            SELECT 1 FROM game_item_market_escrow_locations WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id=p_item
            UNION ALL SELECT 1 FROM game_bazaar_item_locations WHERE item_instance_id=p_item
        ) THEN
            RAISE EXCEPTION 'DUR-03 item must have exactly one location while live and none when retired'
                USING ERRCODE='23514';
        END IF;
        IF EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=p_item) THEN
            RAISE EXCEPTION 'a retired item cannot hold nested contents' USING ERRCODE='23514';
        END IF;
        -- The remaining original checks require House, Inbox or Depot
        -- custody, whose absence was just established.
        RETURN;
    END IF;
$arm$;
BEGIN
    SELECT pg_get_functiondef(p.oid),p.prosrc INTO definition,source
      FROM pg_proc p WHERE p.oid='game_item_location_exclusive(uuid)'::regprocedure;
    IF encode(sha256(convert_to(source,'UTF8')),'hex')
           <> 'd444528a81d0a859050af88c7a9415c880ef1ea250f773866a9735be721be81d'
       OR (length(definition)-length(replace(definition,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected installed Item location proof';
    END IF;
    EXECUTE replace(definition,anchor,terminal_arm||anchor);
END $migration$;

-- Keep both child/parent absence predicates, expressed as point probes over
-- their existing keys. No authority, reservation, receipt or audit check moves.
DO $migration$
DECLARE
    function_name TEXT;
    relation_name TEXT;
    definition TEXT;
    anchor TEXT;
    replacement TEXT;
BEGIN
    FOREACH function_name IN ARRAY ARRAY[
        'game_world_reset_item_proven(uuid)',
        'game_activity_retirement_item_proven(uuid)'
    ] LOOP
        definition := pg_get_functiondef(function_name::regprocedure);
        FOREACH relation_name IN ARRAY ARRAY[
            'game_item_container_entries','game_item_nested_entries'
        ] LOOP
            anchor := format('EXISTS(SELECT 1 FROM %I WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)',relation_name);
            replacement := format('(EXISTS(SELECT 1 FROM %I WHERE item_instance_id=p_item) OR EXISTS(SELECT 1 FROM %I WHERE parent_item_instance_id=p_item))',relation_name,relation_name);
            IF (length(definition)-length(replace(definition,anchor,'')))/length(anchor)<>1 THEN
                RAISE EXCEPTION 'unexpected installed retirement custody predicate in %',function_name;
            END IF;
            definition := replace(definition,anchor,replacement);
        END LOOP;
        -- pg_get_functiondef preserves security mode/search_path, while
        -- CREATE OR REPLACE preserves the OID, ownership and grants.
        EXECUTE definition;
    END LOOP;
END $migration$;
