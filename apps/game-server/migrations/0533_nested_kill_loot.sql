-- W22a: only fresh loot below a fresh container in this kill's fresh corpse.
-- Existing Ground, corpse, item-loop, spell and use paths retain their full guards.
CREATE FUNCTION game_kill_nested_loot_mint_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE proven_corpse UUID;
BEGIN
    WITH RECURSIVE ancestry AS (
        SELECT r.item_instance_id, r.destination_parent_item_instance_id AS parent,
               ARRAY[r.item_instance_id] AS visited, 0 AS depth
          FROM game_item_mint_receipts r
          JOIN game_item_instances i USING (item_instance_id)
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = p_item
           AND r.loot_purpose_key <> 'CORPSE_MATERIALIZATION'
           AND r.created_xact_id = pg_current_xact_id()
           AND i.minted_transaction_id = r.transaction_id AND i.lifecycle = 1
           AND i.world_id = r.death_world_id AND i.state_revision = 1
           AND a.transaction_id = r.transaction_id AND a.item_instance_id = p_item
           AND a.occurred_at = r.occurred_at AND a.envelope_sha256 = r.envelope_sha256
           AND a.created_xact_id = pg_current_xact_id()
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key, v.loot_table_revision_ref,
                       v.loot_purpose_key, v.draw_ordinal, v.intent_binding, v.transaction_id,
                       v.event_id, v.item_instance_id, v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key, r.loot_table_revision_ref,
                       r.loot_purpose_key, r.draw_ordinal, r.intent_binding, r.transaction_id,
                       r.event_id, r.item_instance_id, r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)
        UNION ALL
        SELECT m.item_instance_id, m.destination_parent_item_instance_id,
               path.visited || m.item_instance_id, path.depth + 1
          FROM ancestry path
          JOIN game_item_mint_receipts m ON m.item_instance_id = path.parent
          JOIN game_item_instances i ON i.item_instance_id = m.item_instance_id
          JOIN game_item_mint_receipts child ON child.item_instance_id = path.item_instance_id
         WHERE path.depth < 8 AND NOT m.item_instance_id = ANY(path.visited)
           AND m.created_xact_id = pg_current_xact_id()
           AND i.lifecycle = 1 AND i.quantity = 1
           AND i.minted_transaction_id = m.transaction_id
           AND (m.death_world_id, m.death_channel_id, m.death_scope_ownership_generation,
                m.death_actor_local_id, m.death_actor_local_generation)
             = (child.death_world_id, child.death_channel_id, child.death_scope_ownership_generation,
                child.death_actor_local_id, child.death_actor_local_generation)
           AND EXISTS (
               SELECT 1 FROM (
                   SELECT * FROM game_item_corpse_container_entries
                   UNION ALL SELECT * FROM game_item_nested_entries
               ) e WHERE e.item_instance_id = child.item_instance_id
                 AND e.parent_item_instance_id = m.item_instance_id
                 AND e.world_id = child.death_world_id
                 AND e.placed_transaction_id = child.transaction_id
                 AND e.placement_ordinal = child.destination_ordinal)
           AND NOT EXISTS (SELECT 1 FROM game_item_ground_locations g
                            WHERE g.item_instance_id = child.item_instance_id)
           AND NOT EXISTS (SELECT 1 FROM game_item_container_entries e
                            WHERE e.item_instance_id = child.item_instance_id)
    )
    SELECT r.item_instance_id INTO proven_corpse
      FROM ancestry path
      JOIN game_item_mint_receipts r USING (item_instance_id)
      JOIN game_item_ground_locations g USING (item_instance_id)
     WHERE path.depth >= 2 AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
       AND r.draw_ordinal = 0 AND path.parent IS NULL
       AND r.created_xact_id = pg_current_xact_id()
       AND g.world_id = r.death_world_id AND g.channel_id = r.death_channel_id
       AND g.runtime_scope_ownership_generation = r.death_scope_ownership_generation
     FOR UPDATE OF r;
    RETURN proven_corpse IS NOT NULL;
END;
$$;

-- Extend the installed definitions: later migrations add craft, market, stash,
-- atelier, callback and coin-exchange proof arms dynamically. A copied historical
-- CREATE FUNCTION body would discard those accepted owners.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; insert_at INT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_corpse_container_entry_proven',
        'game_item_mint_consistency_guard', 'game_item_nested_entry_proven'] LOOP
        body := pg_get_functiondef((n || '()')::regprocedure);
        IF n = 'game_item_corpse_container_entry_proven' THEN
            -- Keep the receipt and single-location checks before the new arm.
            insert_at := position('    -- Lock the corpse''s own receipt row' IN body);
            arm := $arm$
    IF game_kill_nested_loot_mint_proven(NEW.item_instance_id) THEN
        -- Only the same-transaction nested MINT arm may bypass the direct-corpse lock.
        IF (SELECT count(*) FROM game_item_corpse_container_entries e
             WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 30 THEN
            RAISE EXCEPTION 'corpse container entry ceiling exceeded' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
$arm$;
        ELSIF n = 'game_item_nested_entry_proven' THEN
            insert_at := regexp_instr(body, E'\\mBEGIN\\M', 1, 1, 1, 'i');
            arm := $arm$
    IF TG_OP = 'INSERT' AND game_kill_nested_loot_mint_proven(NEW.item_instance_id) THEN
        RETURN NULL;
    END IF;
$arm$;
        ELSE
            insert_at := regexp_instr(body, E'\\mBEGIN\\M', 1, 1, 1, 'i');
            arm := $arm$
    IF game_kill_nested_loot_mint_proven(NEW.item_instance_id) THEN
        RETURN NULL;
    END IF;
$arm$;
        END IF;
        IF insert_at = 0 THEN
            RAISE EXCEPTION 'unexpected installed kill-loot proof body: %', n;
        END IF;
        EXECUTE overlay(body placing arm from insert_at for 0);
    END LOOP;
END
$migration$;

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_corpse_container_entry_proven',
        'game_item_mint_consistency_guard', 'game_item_nested_entry_proven',
        'game_kill_nested_loot_mint_proven'] LOOP
        IF n = 'game_kill_nested_loot_mint_proven' THEN
            EXECUTE format('ALTER FUNCTION %I(UUID) SET search_path = pg_catalog,%I,pg_temp', n, current_schema());
        ELSE
            EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog,%I,pg_temp', n, current_schema());
        END IF;
    END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_kill_nested_loot_mint_proven(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_kill_nested_loot_mint_proven(UUID) TO oteryn_game_runtime;
