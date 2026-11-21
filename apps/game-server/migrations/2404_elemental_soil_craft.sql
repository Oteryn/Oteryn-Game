-- Elemental Spheres: one carried soil unit and its exact terrain transition
-- share the existing craft audit, custody proof and transaction.
DO $migration$
DECLARE definition TEXT; body TEXT; at INTEGER;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
        WHERE conrelid='game_item_craft_records'::regclass
          AND conname='game_item_craft_records_outputs_check';
    IF definition NOT LIKE 'CHECK %' THEN RAISE EXCEPTION 'missing craft outputs predecessor'; END IF;
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE 'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ('
        || substring(definition FROM 7) || $check$
        OR COALESCE(consume_source AND recipe='oteryn:elemental_soil.crystal'
            AND outputs='[]'::jsonb AND before_item->>'definition_production_key'
                IN ('oteryn:item.tibia.i940','oteryn:item.tibia.i941','oteryn:item.tibia.i944'),FALSE))
    $check$;

    body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF at=0 THEN RAISE EXCEPTION 'missing craft admission predecessor'; END IF;
    body:=overlay(body placing E'BEGIN\n'||$arm$
    IF NEW.recipe='oteryn:elemental_soil.crystal' THEN
        IF NOT NEW.consume_source OR NEW.outputs<>'[]'::jsonb OR NOT EXISTS (
            SELECT 1 FROM game_terrain_tool_receipts r
            JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)
                =(r.world_id,r.channel_id,r.placement,r.reset_epoch)
            WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
                AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
                AND r.intent=NEW.intent AND r.tool_item_instance_id=NEW.source_item_instance_id
                AND r.created_xact_id=pg_current_xact_id()
                AND r.transformed_item_id=t.state_item_id
                AND t.decay_delays=ARRAY[120000]::bigint[]
                AND ((NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i940'
                        AND t.state_item_id=7746 AND t.decay_states=ARRAY[7742]::bigint[])
                    OR (NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i941'
                        AND t.state_item_id=7745 AND t.decay_states=ARRAY[7743]::bigint[])
                    OR (NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i944'
                        AND t.state_item_id=7744 AND t.decay_states=ARRAY[7741]::bigint[])))
        THEN RAISE EXCEPTION 'elemental soil requires a fresh matching terrain receipt' USING ERRCODE='23514'; END IF;
    END IF;
    $arm$ FROM at FOR 5);
    EXECUTE body;
END $migration$;
