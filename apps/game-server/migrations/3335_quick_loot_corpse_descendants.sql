-- Quick Loot uses the physical corpse root for rights, including for nested kill loot.
-- A bag can be taken separately once its remaining contents have all been removed.
CREATE FUNCTION game_quick_loot_corpse_root(p_item UUID, p_world UUID) RETURNS UUID
LANGUAGE sql STABLE STRICT AS $$
    WITH RECURSIVE ancestors(item, depth) AS (
        SELECT p_item, 1
        UNION ALL
        SELECT n.parent_item_instance_id, a.depth + 1
          FROM ancestors a
          JOIN game_item_nested_entries n ON n.item_instance_id = a.item AND n.world_id = p_world
          JOIN game_item_instances p ON p.item_instance_id = n.parent_item_instance_id
                                    AND p.world_id = n.world_id AND p.lifecycle = 1 AND p.quantity = 1
         WHERE a.depth < 8
    )
    SELECT e.parent_item_instance_id
      FROM ancestors a
      JOIN game_item_corpse_container_entries e ON e.item_instance_id = a.item AND e.world_id = p_world
     WHERE NOT EXISTS (SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id = p_item)
     LIMIT 1
$$;

DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    old_source CONSTANT TEXT := $old$SELECT 1 FROM game_item_corpse_container_entries e
            JOIN game_item_ground_locations cg ON cg.item_instance_id=e.parent_item_instance_id
            JOIN game_item_instances ci ON ci.item_instance_id=cg.item_instance_id
            JOIN game_item_mint_receipts cr ON cr.item_instance_id=ci.item_instance_id
            WHERE e.item_instance_id=NEW.item_instance_id AND ci.lifecycle=1$old$;
    new_source CONSTANT TEXT := $new$SELECT 1 FROM game_item_ground_locations cg
            JOIN game_item_instances ci ON ci.item_instance_id=cg.item_instance_id
            JOIN game_item_mint_receipts cr ON cr.item_instance_id=ci.item_instance_id
            WHERE cg.item_instance_id=game_quick_loot_corpse_root(NEW.item_instance_id,r.world_id) AND ci.lifecycle=1$new$;
BEGIN
    IF (length(body)-length(replace(body,old_source,'')))/length(old_source) <> 1 THEN
        RAISE EXCEPTION 'missing exact Quick Loot source guard';
    END IF;
    EXECUTE replace(body, old_source, new_source);
    EXECUTE format('ALTER FUNCTION game_quick_loot_corpse_root(uuid,uuid) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END
$migration$;

REVOKE ALL ON FUNCTION game_quick_loot_corpse_root(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quick_loot_corpse_root(UUID, UUID) TO oteryn_game_runtime;
