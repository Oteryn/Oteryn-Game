-- I11: qualify the exact shallow backpack tree for the ordinary cookie-tray recipe.
-- This helper checks the path to the named main backpack, including every live parent.
CREATE FUNCTION game_item_craft_nested_custody(
    p_item UUID, p_parent UUID, p_character UUID, p_world UUID
) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    WITH RECURSIVE parents(item_id, depth) AS (
        SELECT e.parent_item_instance_id, 1
        FROM game_item_nested_entries e
        WHERE e.item_instance_id=p_item AND e.world_id=p_world
        UNION ALL
        SELECT e.parent_item_instance_id, p.depth+1
        FROM parents p
        JOIN game_item_instances i ON i.item_instance_id=p.item_id
        JOIN game_item_nested_entries e ON e.item_instance_id=p.item_id
        WHERE p.depth<7 AND i.lifecycle=1 AND i.world_id=p_world AND e.world_id=p_world
    )
    SELECT EXISTS(
        SELECT 1 FROM parents p
        JOIN game_item_instances i ON i.item_instance_id=p.item_id
        JOIN game_item_container_entries e ON e.item_instance_id=p.item_id
        JOIN game_item_container_slots b
          ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
        WHERE i.lifecycle=1 AND i.world_id=p_world AND e.world_id=p_world
          AND b.world_id=p_world AND b.character_id=p_character AND b.item_instance_id=p_parent
    );
$$;
REVOKE ALL ON FUNCTION game_item_craft_nested_custody(UUID,UUID,UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_craft_nested_custody(UUID,UUID,UUID,UUID) TO oteryn_game_runtime;

ALTER TABLE game_item_craft_records ADD COLUMN ingredient_nested_location JSONB;

DO $migration$
DECLARE
    body TEXT;
    before TEXT;
    after TEXT;
    previous TEXT;
BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_item_craft_nested_custody(uuid,uuid,uuid,uuid) SET search_path=pg_catalog,%I,pg_temp',
        current_schema()
    );
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous
    FROM pg_constraint
    WHERE conrelid='game_item_craft_records'::regclass
      AND conname='game_item_craft_event_source_location_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
    EXECUTE format($check$
        ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check
        CHECK ((%s) OR (
            recipe='oteryn:item-conversion.fill-cookie-tray'
            AND jsonb_typeof(event_source_location) IS NOT DISTINCT FROM 'object'
            AND octet_length(event_source_location::text) BETWEEN 2 AND 4096
            AND event_source_location ?& ARRAY['item_instance_id','parent_item_instance_id',
                'world_id','placement_ordinal','placed_transaction_id']
        ))
    $check$, previous);

    body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    before:=$old$    OR (NEW.ingredient_item_instance_id IS NOT NULL AND (
      NOT EXISTS(SELECT 1 FROM game_item_instances x JOIN game_item_container_entries e USING(item_instance_id)
        JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
       WHERE x.item_instance_id=NEW.ingredient_item_instance_id AND to_jsonb(x)=NEW.ingredient_before_item
         AND x.lifecycle=1 AND x.quantity>0 AND x.world_id=NEW.world_id AND e.character_id=NEW.character_id
         AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id)))$old$;
    after:=$new$    OR (NEW.ingredient_item_instance_id IS NOT NULL AND NOT (
      (NEW.ingredient_nested_location IS NULL AND EXISTS(
        SELECT 1 FROM game_item_instances x JOIN game_item_container_entries e USING(item_instance_id)
        JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
       WHERE x.item_instance_id=NEW.ingredient_item_instance_id AND to_jsonb(x)=NEW.ingredient_before_item
         AND x.lifecycle=1 AND x.quantity>0 AND x.world_id=NEW.world_id AND e.character_id=NEW.character_id
         AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id))
      OR (NEW.ingredient_nested_location IS NOT NULL AND EXISTS(
        SELECT 1 FROM game_item_instances x JOIN game_item_nested_entries e USING(item_instance_id)
        WHERE x.item_instance_id=NEW.ingredient_item_instance_id AND to_jsonb(x)=NEW.ingredient_before_item
          AND x.lifecycle=1 AND x.quantity=1 AND x.world_id=NEW.world_id AND e.world_id=NEW.world_id
          AND to_jsonb(e)=NEW.ingredient_nested_location
          AND game_item_craft_nested_custody(x.item_instance_id,NEW.parent_item_instance_id,NEW.character_id,NEW.world_id)))))
    OR (NEW.recipe='oteryn:item-conversion.fill-cookie-tray' AND NEW.event_source_location IS NOT NULL
        AND NOT game_item_craft_nested_custody(i.item_instance_id,NEW.parent_item_instance_id,NEW.character_id,NEW.world_id))$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected I11 ingredient admission guard'; END IF;
    body:=replace(body,before,after);
    before:=$old$     OR ((o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)$old$;
    after:=$new$     OR (NEW.ingredient_nested_location IS NOT NULL AND
         (o->>'ordinal')::numeric IS DISTINCT FROM (NEW.ingredient_nested_location->>'placement_ordinal')::numeric)
     OR (NEW.ingredient_nested_location IS NULL AND
         (o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected I11 output admission guard'; END IF;
    EXECUTE replace(body,before,after);

    body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    before:=$old$      AND (x.quantity>0 OR NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=x.item_instance_id))))$old$;
    after:=$new$      AND (x.quantity>0 OR (
          NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=x.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=x.item_instance_id)))))$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected I11 ingredient completion guard'; END IF;
    body:=replace(body,before,after);
    before:=$old$    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)$old$;
    after:=$new$    OR (i.quantity>0 AND r.recipe='oteryn:item-conversion.fill-cookie-tray'
        AND r.event_source_location IS NOT NULL
        AND NOT game_item_craft_nested_custody(i.item_instance_id,r.parent_item_instance_id,r.character_id,r.world_id))
    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected I11 source completion guard'; END IF;
    body:=replace(body,before,after);
    before:=$old$  IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)$old$;
    after:=$new$   IF r.ingredient_nested_location IS NOT NULL THEN
    IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_nested_entries e USING(item_instance_id)
      WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
        AND n.quantity=(o->>'count')::bigint AND n.definition_family='Item' AND n.definition_production_key=o->>'key'
        AND n.definition_revision_ref=o->>'revision' AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL
        AND e.world_id=r.world_id AND e.parent_item_instance_id=(r.ingredient_nested_location->>'parent_item_instance_id')::uuid
        AND e.placement_ordinal=(o->>'ordinal')::numeric
        AND e.placement_ordinal=(r.ingredient_nested_location->>'placement_ordinal')::numeric
        AND e.placed_transaction_id=r.transaction_id
        AND game_item_craft_nested_custody(n.item_instance_id,r.parent_item_instance_id,r.character_id,r.world_id))
    THEN RAISE EXCEPTION 'craft nested output or custody mismatch' USING ERRCODE='23514'; END IF;
    CONTINUE;
   END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected I11 output completion guard'; END IF;
    EXECUTE replace(body,before,after);

    body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    IF regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i')=0 THEN
        RAISE EXCEPTION 'unexpected nested entry proof';
    END IF;
    EXECUTE regexp_replace(body,E'\\mBEGIN\\M',$arm$BEGIN
    IF TG_OP='DELETE' AND EXISTS(
        SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i
            ON i.item_instance_id=r.ingredient_item_instance_id
        WHERE r.ingredient_item_instance_id=OLD.item_instance_id
          AND r.ingredient_nested_location=to_jsonb(OLD) AND r.world_id=OLD.world_id
          AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
          AND r.created_xact_id=pg_current_xact_id()
    ) THEN RETURN NULL; END IF;
    IF TG_OP='INSERT' AND EXISTS(
        SELECT 1 FROM game_item_craft_records r
        WHERE r.transaction_id=NEW.placed_transaction_id AND r.ingredient_nested_location IS NOT NULL
          AND r.outputs->0->>'item'=NEW.item_instance_id::text
          AND r.created_xact_id=pg_current_xact_id()
    ) THEN PERFORM game_item_craft_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
$arm$);
END
$migration$;

-- Either nested operand is admitted only for the fixed cookie-tray conversion.
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_nested_cookie_tray_check CHECK (
    (ingredient_nested_location IS NULL AND
      (recipe<>'oteryn:item-conversion.fill-cookie-tray' OR event_source_location IS NULL))
    OR (
        recipe='oteryn:item-conversion.fill-cookie-tray' AND consume_source
        AND enchanting IS NULL AND soul_core_proof IS NULL
        AND before_item->>'definition_production_key' IS NOT DISTINCT FROM 'oteryn:item.tibia.i6276'
        AND before_item->>'definition_revision_ref' IS NOT DISTINCT FROM 'definition-r1'
        AND ingredient_item_instance_id IS NOT NULL
        AND ingredient_before_item->>'definition_family' IS NOT DISTINCT FROM 'Item'
        AND ingredient_before_item->>'definition_production_key' IS NOT DISTINCT FROM 'oteryn:item.tibia.i3464'
        AND ingredient_before_item->>'definition_revision_ref' IS NOT DISTINCT FROM 'definition-r1'
        AND ingredient_count=1
        AND jsonb_array_length(outputs)=1
        AND outputs->0->>'key' IS NOT DISTINCT FROM 'oteryn:item.tibia.i8020'
        AND outputs->0->>'revision' IS NOT DISTINCT FROM 'definition-r1'
        AND outputs->0->>'count' IS NOT DISTINCT FROM '1'
        AND (ingredient_nested_location IS NULL OR (
            jsonb_typeof(ingredient_nested_location)='object'
            AND octet_length(ingredient_nested_location::text) BETWEEN 2 AND 4096
            AND ingredient_nested_location ?& ARRAY['item_instance_id','parent_item_instance_id',
                'world_id','placement_ordinal','placed_transaction_id']
        ))
    )
);
