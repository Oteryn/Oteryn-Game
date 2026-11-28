-- Extend the spell debit's predecessor and surviving-stack proofs to the same
-- exact leaf in a carried bag. The quiver projectile proof keeps its own rules.
CREATE FUNCTION game_spell_bag_entry(p_rune boolean,p_character bytea,p_parent uuid,p_item uuid,p_ordinal numeric,p_world uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT game_quiver_entry(p_character,p_parent,p_item,p_ordinal,p_world) OR (p_rune AND EXISTS (
        WITH RECURSIVE ancestors(item,depth) AS (
            SELECT n.parent_item_instance_id,1 FROM game_item_nested_entries n
            WHERE n.item_instance_id=p_item AND n.parent_item_instance_id=p_parent
              AND n.placement_ordinal=p_ordinal AND n.world_id=p_world
            UNION ALL
            SELECT n.parent_item_instance_id,a.depth+1 FROM ancestors a
            JOIN game_item_nested_entries n ON n.item_instance_id=a.item
            WHERE n.world_id=p_world AND a.depth<7
        )
        SELECT 1 FROM ancestors a
        JOIN game_item_container_entries e ON e.item_instance_id=a.item
        JOIN game_item_container_slots bag ON bag.item_instance_id=e.parent_item_instance_id
        WHERE e.character_id=encode(p_character,'hex')::uuid AND bag.character_id=e.character_id
          AND e.world_id=p_world AND bag.world_id=p_world
    ))
$$;

DO $migration$
DECLARE
    guard regprocedure;
    body text;
BEGIN
    EXECUTE format('ALTER FUNCTION game_spell_bag_entry(boolean,bytea,uuid,uuid,numeric,uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    FOREACH guard IN ARRAY ARRAY['game_spell_item_capture_custody()'::regprocedure,
                                 'game_spell_item_line_proven()'::regprocedure] LOOP
        body := pg_get_functiondef(guard);
        IF position('game_quiver_entry(' in body)=0 THEN
            RAISE EXCEPTION 'missing carried spell custody proof in %', guard;
        END IF;
        EXECUTE replace(body,'game_quiver_entry(',format(
            'game_spell_bag_entry((r.swing_ordinal IS NULL AND starts_with(r.spell_production_key,''candidate:spell/rune/'') AND %1$s.quantity_after=%1$s.quantity_before-1),',
            CASE WHEN guard='game_spell_item_capture_custody()'::regprocedure THEN 'l' ELSE 'NEW' END
        ));
    END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_spell_bag_entry(boolean,bytea,uuid,uuid,numeric,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_spell_bag_entry(boolean,bytea,uuid,uuid,numeric,uuid) TO oteryn_game_runtime;
