-- A carried sculpture consumes its material and keeps the tool. Terrain
-- gathering retains its separate transform receipt and admission proof.
DO $migration$
DECLARE
    body TEXT;
    previous_arm TEXT := $arm$IF (NOT NEW.consume_source OR NEW.recipe='oteryn:gathering.crystal') AND NEW.recipe<>'oteryn:skinning.crystal' THEN$arm$;
    sculpture_arm TEXT := $arm$
    IF NEW.recipe='oteryn:sculpture.carried' THEN
        IF NEW.consume_source OR NEW.ingredient_item_instance_id IS NULL
            OR NEW.ingredient_count IS DISTINCT FROM 1
            OR COALESCE(NEW.before_item->>'definition_production_key','')
                NOT IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')
            OR COALESCE(NEW.ingredient_before_item->>'definition_production_key','') NOT IN (
                'oteryn:item.tibia.i7441','oteryn:item.tibia.i7442','oteryn:item.tibia.i7444',
                'oteryn:item.tibia.i7445','oteryn:item.tibia.i10426')
            OR jsonb_array_length(NEW.outputs)>1
            OR (jsonb_array_length(NEW.outputs)=1 AND (
                NEW.outputs->0->>'count' IS DISTINCT FROM '1'
                OR NOT COALESCE(CASE NEW.ingredient_before_item->>'definition_production_key'
                    WHEN 'oteryn:item.tibia.i7441' THEN NEW.outputs->0->>'key'='oteryn:item.tibia.i7442'
                    WHEN 'oteryn:item.tibia.i7442' THEN NEW.outputs->0->>'key'='oteryn:item.tibia.i7444'
                    WHEN 'oteryn:item.tibia.i7444' THEN NEW.outputs->0->>'key'='oteryn:item.tibia.i7445'
                    WHEN 'oteryn:item.tibia.i7445' THEN NEW.outputs->0->>'key'='oteryn:item.tibia.i7446'
                    WHEN 'oteryn:item.tibia.i10426' THEN NEW.outputs->0->>'key' IN (
                        'oteryn:item.tibia.i10427','oteryn:item.tibia.i10428','oteryn:item.tibia.i10429')
                    END, FALSE)))
        THEN RAISE EXCEPTION 'carried sculpting requires a qualified tool, material and yield' USING ERRCODE='23514';
        END IF;
    ELSIF (NOT NEW.consume_source OR NEW.recipe='oteryn:gathering.crystal') AND NEW.recipe<>'oteryn:skinning.crystal' THEN$arm$;
    outputs_check TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    IF (length(body)-length(replace(body,previous_arm,'')))/length(previous_arm)<>1 THEN
        RAISE EXCEPTION 'expected one existing gathering craft admission arm';
    END IF;
    EXECUTE replace(body,previous_arm,sculpture_arm);

    SELECT pg_get_constraintdef(oid) INTO STRICT outputs_check FROM pg_constraint
        WHERE conrelid='game_item_craft_records'::regclass
          AND conname='game_item_craft_records_outputs_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE format(
        'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
         CHECK ((%s) OR (recipe=''oteryn:sculpture.carried'' AND NOT consume_source AND outputs=''[]''::jsonb))',
         substring(outputs_check FROM 7));
END $migration$;

-- Keep the original direct ingredient admission for other recipes. A sculpture
-- can also debit an exact nested leaf or a single material in Channel Ground custody.
ALTER TABLE game_item_craft_records ADD COLUMN ingredient_location JSONB;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_ingredient_location_check
CHECK (ingredient_location IS NULL OR (
    recipe='oteryn:sculpture.carried' AND ingredient_item_instance_id IS NOT NULL
    AND jsonb_typeof(ingredient_location)='object'
    AND octet_length(ingredient_location::text) BETWEEN 2 AND 4096
));

DO $migration$
DECLARE body TEXT; anchor TEXT; arm TEXT; location_check TEXT; name TEXT; begin_at INT;
    -- The ammunition-only definitions used by game_quiver_entry.
    quiver_keys CONSTANT TEXT := $keys$
        'oteryn:item.tibia.i35562','oteryn:item.tibia.i35848','oteryn:item.tibia.i35849','oteryn:item.tibia.i35524',
        'oteryn:item.tibia.i36666','oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644'
    $keys$;
BEGIN
    SELECT substring(pg_get_constraintdef(oid) FROM 7) INTO STRICT location_check
      FROM pg_constraint WHERE conrelid='game_item_craft_records'::regclass
      AND conname='game_item_craft_event_source_location_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
    EXECUTE format($constraint$
        ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check
        CHECK ((%s) OR (
            NOT consume_source AND recipe IN ('oteryn:sculpture.carried','oteryn:gathering.crystal')
            AND before_item->>'definition_production_key' IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')
            AND jsonb_typeof(event_source_location)='object'
            AND octet_length(event_source_location::text) BETWEEN 2 AND 4096
        ))$constraint$, location_check);

    body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    anchor:='OR (NEW.ingredient_item_instance_id IS NOT NULL AND (';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing direct craft ingredient guard'; END IF;
    body:=replace(body,anchor,'OR (NEW.ingredient_location IS NULL AND NEW.ingredient_item_instance_id IS NOT NULL AND (');
    anchor:='SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id FOR UPDATE;';
    arm:=$arm$
    IF NEW.ingredient_location IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_item_instances x WHERE x.item_instance_id=NEW.ingredient_item_instance_id
        AND x.world_id=NEW.world_id AND x.lifecycle=1 AND x.quantity>0
        AND to_jsonb(x)=NEW.ingredient_before_item AND (
            EXISTS (
                SELECT 1 FROM game_item_nested_entries e
                JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true
                WHERE e.item_instance_id=x.item_instance_id AND e.world_id=x.world_id
                AND to_jsonb(e)=NEW.ingredient_location AND owner.owner_character=NEW.character_id
                AND owner.depth BETWEEN 1 AND 7
            ) OR (x.quantity=1 AND EXISTS (
                SELECT 1 FROM game_item_ground_locations g
                WHERE g.item_instance_id=x.item_instance_id AND g.world_id=x.world_id
                AND g.scope_kind=1 AND to_jsonb(g)=NEW.ingredient_location
                AND (game_item_loop_drop_ref(g.corpse_ref) OR game_item_action_ground_source(x.item_instance_id))
            ))
        )
    ) THEN RAISE EXCEPTION 'sculpture material custody changed' USING ERRCODE='23514'; END IF;
    IF NEW.outputs->0 ? 'ground' AND NEW.recipe<>'oteryn:reward-bag.event_6570' AND NOT (
        jsonb_array_length(NEW.outputs)=1 AND NOT NEW.consume_source AND (
            (NEW.recipe='oteryn:gathering.crystal'
                AND NEW.outputs->0->>'key' IN ('oteryn:item.tibia.i10427','oteryn:item.tibia.i10428','oteryn:item.tibia.i10429')
                AND EXISTS (
                    SELECT 1 FROM game_terrain_tool_receipts t
                    JOIN game_terrain_tool_transforms v USING(world_id,channel_id,placement,reset_epoch)
                    WHERE t.game_session_id=NEW.game_session_id AND t.command_id=NEW.command_id
                    AND t.character_id=NEW.character_id AND t.world_id=NEW.world_id
                    AND t.tool_item_instance_id=NEW.source_item_instance_id AND t.intent=NEW.intent
                    AND t.transformed_item_id=0
                    AND v.map_revision=NEW.outputs->0->'ground'->>'map_revision'
                    AND 'sha256:'||encode(v.content_generation,'hex')=NEW.outputs->0->'ground'->>'content_revision'
                    AND encode(uuid_send(t.channel_id),'hex')=NEW.outputs->0->'ground'->>'channel'
                ))
            OR (NEW.recipe='oteryn:sculpture.carried' AND (
                NEW.ingredient_before_item->>'definition_production_key'='oteryn:item.tibia.i10426'
                OR (NEW.ingredient_location ? 'spatial_position') IS TRUE
            ))
        )
    ) THEN RAISE EXCEPTION 'unqualified Ground sculpture output' USING ERRCODE='23514'; END IF;
$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft input lock'; END IF;
    body:=replace(body,anchor,anchor||arm);
    anchor:='FOR o IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP';
    arm:=$arm$
     IF o ? 'ground' AND (o IS DISTINCT FROM NEW.outputs->0
         OR jsonb_typeof(o->'ground') IS DISTINCT FROM 'object') THEN
       RAISE EXCEPTION 'unexpected Ground craft output' USING ERRCODE='23514';
     END IF;
$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output admission'; END IF;
    body:=replace(body,anchor,anchor||arm||$arm$
     IF o ? 'nested_parent' AND (
         o ? 'ground' OR jsonb_array_length(NEW.outputs)<>1 OR NEW.consume_source
         OR o->>'count' IS DISTINCT FROM '1'
         OR COALESCE(o->>'key','') NOT IN (
             'oteryn:item.tibia.i10427','oteryn:item.tibia.i10428','oteryn:item.tibia.i10429')
         OR NOT (
             (NEW.recipe='oteryn:sculpture.carried' AND
                 NEW.ingredient_before_item->>'definition_production_key'='oteryn:item.tibia.i10426') IS TRUE
             OR (NEW.recipe='oteryn:gathering.crystal' AND EXISTS (
                 SELECT 1 FROM game_terrain_tool_receipts t
                 WHERE t.game_session_id=NEW.game_session_id AND t.command_id=NEW.command_id
                 AND t.character_id=NEW.character_id AND t.world_id=NEW.world_id
                 AND t.tool_item_instance_id=NEW.source_item_instance_id AND t.intent=NEW.intent
                 AND t.transformed_item_id=0
             ))
         )
         OR NOT EXISTS (
             SELECT 1 FROM game_item_instances parent
             JOIN LATERAL game_item_loop_owner(parent.item_instance_id) owner ON true
             WHERE parent.item_instance_id=(o->>'nested_parent')::uuid
             AND parent.lifecycle=1 AND parent.world_id=NEW.world_id
             AND parent.definition_production_key NOT IN (SCULPTURE_QUIVER_KEYS)
             AND owner.owner_character=NEW.character_id AND owner.depth BETWEEN 1 AND 7
         )
         OR (o->>'ordinal')::numeric <= (
             SELECT coalesce(max(placement_ordinal),0) FROM game_item_nested_entries
             WHERE parent_item_instance_id=(o->>'nested_parent')::uuid
         )
     ) THEN RAISE EXCEPTION 'unqualified nested marble output' USING ERRCODE='23514'; END IF;
$arm$);
    -- Blue Present retains its own drop admission. Sculptures are admitted by
    -- the recipe and custody checks above, including the backpack ordinal for
    -- ordinary Ground sculpture outputs and the separate nested ordinal proof.
    anchor:=$arm$IF o ? 'ground' AND (
      NEW.recipe<>'oteryn:reward-bag.event_6570'$arm$;
    arm:=$arm$IF o ? 'ground' AND NEW.recipe NOT IN ('oteryn:sculpture.carried','oteryn:gathering.crystal') AND (
      NEW.recipe<>'oteryn:reward-bag.event_6570'$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing Blue Present Ground admission'; END IF;
    body:=replace(body,anchor,arm);
    anchor:=$arm$(NOT (o ? 'ground') AND (o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)$arm$;
    arm:=$arm$(NOT (o ? 'ground' AND NEW.recipe='oteryn:reward-bag.event_6570') AND NOT (o ? 'nested_parent') AND (o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing main backpack output ordinal guard'; END IF;
    body:=replace(body,anchor,arm);
    EXECUTE replace(body,'SCULPTURE_QUIVER_KEYS',quiver_keys);

    body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    -- The exact ingredient debit arm originally checked direct custody only on retirement.
    anchor:='(x.quantity>0 OR NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=x.item_instance_id))';
    arm:=$arm$(x.quantity>0 OR NOT (
        EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=x.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=x.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=x.item_instance_id)))$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft material retirement proof'; END IF;
    body:=replace(body,anchor,arm);
    anchor:='IF NOT EXISTS(SELECT 1 FROM game_item_instances n LEFT JOIN game_item_container_entries e USING(item_instance_id)';
    arm:=$arm$IF o ? 'ground' AND r.recipe IN ('oteryn:sculpture.carried','oteryn:gathering.crystal') THEN
      IF NOT EXISTS (
        SELECT 1 FROM game_item_instances n JOIN game_item_ground_locations g USING(item_instance_id,world_id)
        WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
        AND n.quantity=(o->>'count')::bigint AND n.definition_family='Item'
        AND n.definition_production_key=o->>'key' AND n.definition_revision_ref=o->>'revision'
        AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL
        AND g.scope_kind=1 AND encode(uuid_send(g.channel_id),'hex')=o->'ground'->>'channel'
        AND g.runtime_scope_ownership_generation::text=o->'ground'->>'generation'
        AND encode(g.spatial_position,'hex')=o->'ground'->>'position'
        AND g.map_revision=o->'ground'->>'map_revision' AND g.content_revision=o->'ground'->>'content_revision'
        AND encode(g.native_room_placement_context,'hex')=o->'ground'->>'frame'
        AND g.corpse_ref=convert_to('sculpture-craft-v1','UTF8')
      ) THEN RAISE EXCEPTION 'sculpture Ground output or custody mismatch' USING ERRCODE='23514'; END IF;
    ELSIF o ? 'nested_parent' THEN
      IF NOT EXISTS (
        SELECT 1 FROM game_item_instances n JOIN game_item_nested_entries e USING(item_instance_id,world_id)
        JOIN game_item_instances parent ON parent.item_instance_id=e.parent_item_instance_id
        JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true
        WHERE parent.definition_production_key NOT IN (SCULPTURE_QUIVER_KEYS)
        AND n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
        AND n.quantity=(o->>'count')::bigint AND n.definition_family='Item'
        AND n.definition_production_key=o->>'key' AND n.definition_revision_ref=o->>'revision'
        AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL
        AND e.parent_item_instance_id=(o->>'nested_parent')::uuid AND e.placed_transaction_id=r.transaction_id
        AND e.placement_ordinal=(o->>'ordinal')::numeric
        AND owner.owner_character=r.character_id AND owner.depth BETWEEN 1 AND 7
        AND (SELECT count(*) FROM game_item_nested_entries
             WHERE parent_item_instance_id=e.parent_item_instance_id)<=20
      ) THEN RAISE EXCEPTION 'marble output or nested custody mismatch' USING ERRCODE='23514'; END IF;
    ELSIF NOT EXISTS(SELECT 1 FROM game_item_instances n LEFT JOIN game_item_container_entries e USING(item_instance_id)$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output completion'; END IF;
    body:=replace(body,anchor,arm);
    EXECUTE replace(body,'SCULPTURE_QUIVER_KEYS',quiver_keys);

    FOREACH name IN ARRAY ARRAY['game_item_nested_entry_proven()','game_item_ground_removal_proven()',
                                 'game_item_ground_insertion_guard()'] LOOP
        body:=pg_get_functiondef(name::regprocedure);
        begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'missing sculpture custody guard: %',name; END IF;
        IF name='game_item_ground_insertion_guard()' THEN
            arm:=$arm$
            IF EXISTS (
                SELECT 1 FROM game_item_instances i JOIN game_item_craft_records r
                    ON r.transaction_id=i.minted_transaction_id
                WHERE i.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
                AND r.recipe IN ('oteryn:sculpture.carried','oteryn:gathering.crystal')
                AND r.outputs->0 ? 'ground' AND (r.outputs->0->>'item')::uuid=i.item_instance_id
            ) THEN PERFORM game_item_craft_complete(
                (SELECT minted_transaction_id FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id)
            ); RETURN NULL; END IF;
$arm$;
        ELSE
            arm:=$arm$
            IF TG_OP='DELETE' AND EXISTS (
                SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i
                    ON i.item_instance_id=r.ingredient_item_instance_id AND i.last_transaction_id=r.transaction_id
                WHERE i.item_instance_id=OLD.item_instance_id AND i.lifecycle=2 AND i.quantity=0
                AND r.ingredient_location=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()
            ) THEN RETURN NULL; END IF;
$arm$;
        END IF;
        IF name='game_item_nested_entry_proven()' THEN
            arm:=arm||$arm$
            IF TG_OP='INSERT' AND EXISTS (
                SELECT 1 FROM game_item_craft_records r
                WHERE r.transaction_id=NEW.placed_transaction_id AND r.created_xact_id=pg_current_xact_id()
                AND r.outputs->0 ? 'nested_parent' AND (r.outputs->0->>'item')::uuid=NEW.item_instance_id
            ) THEN PERFORM game_item_craft_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
$arm$;
        END IF;
        EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    END LOOP;

    -- The ordinary loop pickup and Ground projection already share this provenance query.
    body:=pg_get_functiondef('game_item_action_ground_source(uuid)'::regprocedure);
    anchor:='(o->>''item'')::uuid=p_item)';
    arm:=$arm$(o->>'item')::uuid=p_item)
        OR EXISTS (
            SELECT 1 FROM game_item_instances i JOIN game_item_craft_records r
                ON r.transaction_id=i.minted_transaction_id
            WHERE i.item_instance_id=p_item
            AND r.recipe IN ('oteryn:sculpture.carried','oteryn:gathering.crystal')
            AND r.outputs->0 ? 'ground'
            AND (r.outputs->0->>'item')::uuid=p_item
        )$arm$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing Ground pickup provenance'; END IF;
    EXECUTE replace(body,anchor,arm);
END $migration$;
