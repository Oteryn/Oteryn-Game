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
