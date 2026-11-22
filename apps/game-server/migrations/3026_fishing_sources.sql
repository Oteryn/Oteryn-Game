-- Source fishing uses the existing fenced craft and terrain receipts.
CREATE TABLE game_fishing_shimmer_cooldowns (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    available_at TIMESTAMPTZ NOT NULL
);
REVOKE ALL ON game_fishing_shimmer_cooldowns FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_fishing_shimmer_cooldowns TO oteryn_game_runtime;
GRANT SELECT ON game_fishing_shimmer_cooldowns TO oteryn_game_control;

DO $migration$
DECLARE definition TEXT; body TEXT; anchor TEXT; fn TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
    WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';

    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE 'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ('
        || substring(definition FROM 7) || $check$
        OR (recipe='oteryn:gathering.crystal' AND consume_source
            AND before_item->>'definition_production_key'='oteryn:item.tibia.i3492'
            AND outputs='[]'::jsonb)
        OR (recipe='oteryn:fishing.corpse' AND NOT consume_source
            AND jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 0 AND 1))
    $check$;

    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    anchor := 'AND r.transformed_item_id=4609';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected mechanical fishing proof';
    END IF;
    body := replace(body,anchor,
        'AND r.transformed_item_id IN (4609,4610,4611,4612,4613,4614,4809,4810,4811,4812,4813,4814,21314)');
    anchor := 'AND tool.definition_production_key IN (''oteryn:item.tibia.i3293''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected gathering tool proof';
    END IF;
    body := replace(body,anchor,$arm$
        AND ((tool.definition_production_key='oteryn:item.tibia.i3483' AND r.transformed_item_id=12560
              AND r.created_xact_id=pg_current_xact_id())
             OR tool.definition_production_key IN ('oteryn:item.tibia.i3293'$arm$);
    anchor := '''oteryn:item.tibia.i9596''))';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected end of gathering tool proof';
    END IF;
    body := replace(body,anchor,'''oteryn:item.tibia.i9596'')))');
    anchor := $old$AND NEW.recipe<>'oteryn:skinning.crystal' THEN$old$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing corpse craft dispatch guard'; END IF;
    body := replace(body,anchor,$new$AND NEW.recipe NOT IN ('oteryn:skinning.crystal','oteryn:fishing.corpse') THEN$new$);
    EXECUTE overlay(body placing E'BEGIN\n'||$arm$
    IF NEW.recipe='oteryn:fishing.corpse' AND (NEW.consume_source OR NOT EXISTS (
        SELECT 1 FROM game_corpse_skinning_records s
        JOIN game_item_mint_receipts m ON m.item_instance_id=s.corpse_item_instance_id
        WHERE s.transaction_id=NEW.transaction_id AND s.created_xact_id=pg_current_xact_id()
          AND NEW.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i3483','oteryn:item.tibia.i9306')
          AND (s.before_item->>'world_id')::uuid=NEW.world_id
          AND s.before_item->>'definition_production_key'='oteryn:item.tibia.i9582'
          AND s.spent_key='oteryn:item.tibia.i9583'
          AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
          AND (m.corpse_top_damage_character_id IS NULL OR m.corpse_top_damage_character_id=NEW.character_id)))
    THEN RAISE EXCEPTION 'corpse fishing requires an exact owned elemental corpse' USING ERRCODE='23514'; END IF;
    IF NEW.recipe='oteryn:gathering.crystal' AND NEW.consume_source AND NEW.outputs='[]'::jsonb
      AND NOT EXISTS (
        SELECT 1 FROM game_terrain_tool_receipts r JOIN game_item_instances tool
          ON tool.item_instance_id=r.tool_item_instance_id
        WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
          AND r.created_xact_id=pg_current_xact_id()
          AND r.transformed_item_id IN (12558,12559,12560,13988)
          AND tool.definition_production_key='oteryn:item.tibia.i9306')
    THEN RAISE EXCEPTION 'empty fishing catch requires a fresh dirty or desert water receipt' USING ERRCODE='23514'; END IF;
    $arm$ FROM regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i') FOR 5);

    FOREACH fn IN ARRAY ARRAY['game_item_instance_guard()', 'game_item_instance_change_proven()',
                           'game_item_craft_complete(uuid)', 'game_corpse_skinning_complete()']
    LOOP
        body := pg_get_functiondef(fn::regprocedure);
        IF fn='game_item_craft_complete(uuid)' THEN
            anchor := $old$r.recipe='oteryn:skinning.crystal'$old$;
            IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing corpse craft completion guard'; END IF;
            EXECUTE replace(body,anchor,$new$r.recipe IN ('oteryn:skinning.crystal','oteryn:fishing.corpse')$new$);
        ELSIF fn='game_corpse_skinning_complete()' THEN
            anchor := $old$AND c.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')$old$;
            -- 2847 already parenthesizes the skinning tool admission and adds its timed check.
            anchor := substring(anchor FROM 5);
            IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing corpse tool completion guard'; END IF;
            body := replace(body,anchor,$new$((c.recipe='oteryn:skinning.crystal' AND
              c.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942'))
              OR (c.recipe='oteryn:fishing.corpse' AND c.before_item->>'definition_production_key'
                  IN ('oteryn:item.tibia.i3483','oteryn:item.tibia.i9306')))$new$);
            body := replace(body,$old$OR c.before_item->>'definition_production_key'='oteryn:item.tibia.i5942'$old$,
              $new$OR c.before_item->>'definition_production_key'='oteryn:item.tibia.i5942'
                   OR c.recipe='oteryn:fishing.corpse'$new$);
            EXECUTE replace(body,$old$c.recipe='oteryn:skinning.crystal'
$old$,
                $new$c.recipe IN ('oteryn:skinning.crystal','oteryn:fishing.corpse')
$new$);
        ELSE
            anchor := $old$c.recipe='oteryn:skinning.crystal'$old$;
            IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing corpse transition guard in %',fn; END IF;
            EXECUTE replace(body,anchor,$new$c.recipe IN ('oteryn:skinning.crystal','oteryn:fishing.corpse')$new$);
        END IF;
    END LOOP;
END $migration$;
