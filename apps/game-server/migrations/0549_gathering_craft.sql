-- FIXM09: preserve the carried gathering tool; use the existing craft mint/audit proof.
-- A terrain receipt in this same command is mandatory for the new zero-debit shape.
ALTER TABLE game_item_craft_records ADD COLUMN consume_source BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
    CHECK (jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 0 AND 4
        AND (jsonb_array_length(outputs)>0 OR (NOT consume_source AND recipe='oteryn:gathering.crystal')));
-- Zero is an internal removed-placement tombstone, never a definition or appearance id.
ALTER TABLE game_terrain_tool_transforms DROP CONSTRAINT game_terrain_tool_transforms_state_item_id_check;
ALTER TABLE game_terrain_tool_transforms ADD CONSTRAINT game_terrain_tool_transforms_state_item_id_check
    CHECK (state_item_id BETWEEN 0 AND 4294967295);
ALTER TABLE game_terrain_tool_receipts DROP CONSTRAINT game_terrain_tool_receipts_transformed_item_id_check;
ALTER TABLE game_terrain_tool_receipts ADD CONSTRAINT game_terrain_tool_receipts_transformed_item_id_check
    CHECK (transformed_item_id BETWEEN 0 AND 4294967295);

DO $migration$
DECLARE body TEXT; before TEXT; after TEXT; arm TEXT; begin_at INT;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected craft admission body'; END IF;
    arm := $arm$
    IF NOT NEW.consume_source OR NEW.recipe='oteryn:gathering.crystal' THEN
        IF NEW.recipe<>'oteryn:gathering.crystal' OR NOT EXISTS (
            SELECT 1 FROM game_terrain_tool_receipts r
            JOIN game_item_instances tool ON tool.item_instance_id=r.tool_item_instance_id
            WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
                AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
                AND r.intent=NEW.intent AND tool.world_id=NEW.world_id AND tool.lifecycle=1
                AND ((NOT NEW.consume_source AND r.tool_item_instance_id=NEW.source_item_instance_id
                        AND tool.definition_production_key IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942'))
                    OR (NEW.consume_source AND tool.definition_production_key='oteryn:item.tibia.i3483'
                        AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i3492')))
        THEN RAISE EXCEPTION 'gathering requires a matching terrain and tool receipt' USING ERRCODE='23514'; END IF;
    END IF;
$arm$;
    body := overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    before := 'i.quantity=1 AND o=NEW.outputs->0';
    after := 'NEW.consume_source AND i.quantity=1 AND o=NEW.outputs->0';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft ordinal guard'; END IF;
    EXECUTE replace(body,before,after);

    body := pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    before := $old$OR i.quantity<>(r.before_item->>'quantity')::bigint-1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id$old$;
    after := $new$OR i.quantity<>(r.before_item->>'quantity')::bigint-(CASE WHEN r.consume_source THEN 1 ELSE 0 END)
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR (r.consume_source AND i.last_transaction_id IS DISTINCT FROM r.transaction_id)
    OR (NOT r.consume_source AND to_jsonb(i) IS DISTINCT FROM r.before_item)$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft debit proof'; END IF;
    body := replace(body,before,after);
    before := '(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1';
    after := '(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.consume_source THEN 1 ELSE 0 END)';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft write count'; END IF;
    EXECUTE replace(body,before,after);
END $migration$;
