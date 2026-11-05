-- W19: extend the existing immutable craft receipt to the exact multi-gift
-- Christmas and anniversary bundles; every legacy recipe keeps its <=4 bound.
-- Crystal 201189f christmas_bundle.lua:16-31 and anniversary_reward_boxes.lua:2-55.
-- Forward rollback: stop the event recipes; retain all receipts and minted items.
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
CHECK (jsonb_typeof(outputs) = 'array' AND
    CASE recipe
        WHEN 'oteryn:event-container.6506' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i6506'
            AND jsonb_array_length(outputs) = 7
        WHEN 'oteryn:event-container.6507' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i6507'
            AND jsonb_array_length(outputs) = 7
        WHEN 'oteryn:event-container.6508' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i6508'
            AND jsonb_array_length(outputs) = 7
        WHEN 'oteryn:event-container.39704' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39704'
            AND jsonb_array_length(outputs) = 8
        WHEN 'oteryn:event-container.39705' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39705'
            AND jsonb_array_length(outputs) = 13
        WHEN 'oteryn:event-container.39706' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39706'
            AND jsonb_array_length(outputs) = 10
        WHEN 'oteryn:event-container.39710' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39710'
            AND jsonb_array_length(outputs) = 14
        ELSE jsonb_array_length(outputs) BETWEEN 0 AND 4
            AND (jsonb_array_length(outputs) > 0
                OR (NOT consume_source AND recipe = 'oteryn:gathering.crystal')
                OR (consume_source AND recipe = 'oteryn:soulpit.entry'))
    END);

-- Source remove() consumes the whole anniversary stack, whereas the other
-- containers' remove(1) consumes one unit. Retain the original receipt, locks,
-- before-row proof, instance/custody guards and deferred complete check.
-- Crystal ItemFunctions::luaItemRemove:274-279; Game::internalRemoveItem:2605-2607.
DO $migration$
DECLARE body TEXT; before TEXT; after TEXT; arm TEXT; begin_at INT;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    before := 'NEW.consume_source AND i.quantity=1 AND o=NEW.outputs->0';
    after := $new$NEW.consume_source AND (i.quantity=1 OR NEW.recipe IN (
        'oteryn:event-container.39704','oteryn:event-container.39705',
        'oteryn:event-container.39706','oteryn:event-container.39710'))
        AND o=NEW.outputs->0$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected event craft ordinal guard'; END IF;
    EXECUTE replace(body,before,after);

    body := pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    before := $old$OR i.quantity<>(r.before_item->>'quantity')::bigint-(CASE WHEN r.consume_source THEN 1 ELSE 0 END)$old$;
    after := $new$OR i.quantity<>(r.before_item->>'quantity')::bigint-
        (CASE WHEN NOT r.consume_source THEN 0
            WHEN r.recipe IN ('oteryn:event-container.39704','oteryn:event-container.39705',
                'oteryn:event-container.39706','oteryn:event-container.39710')
            THEN (r.before_item->>'quantity')::bigint ELSE 1 END)$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected event craft debit proof'; END IF;
    EXECUTE replace(body,before,after);

    body := pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected event craft instance guard'; END IF;
    arm := $arm$
    IF TG_OP='UPDATE' AND NEW.quantity=0 AND NEW.lifecycle=2
        AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id'])
            IS NOT DISTINCT FROM (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
        AND EXISTS (SELECT 1 FROM game_item_craft_records r
            WHERE r.source_item_instance_id=OLD.item_instance_id
                AND r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD)
                AND r.created_xact_id=pg_current_xact_id() AND r.consume_source
                AND r.recipe IN ('oteryn:event-container.39704','oteryn:event-container.39705',
                    'oteryn:event-container.39706','oteryn:event-container.39710'))
    THEN RETURN NEW; END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- Nested carried sources use the same craft receipt. A NULL location retains
-- every existing source shape; the new exact row snapshot is limited to W19.
ALTER TABLE game_item_craft_records ADD COLUMN event_source_location JSONB;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check
CHECK (event_source_location IS NULL OR (
    consume_source AND recipe IN (
            'oteryn:event-container.6506',
            'oteryn:event-container.6507',
            'oteryn:event-container.6508',
            'oteryn:event-container.6570',
            'oteryn:event-container.6571',
            'oteryn:event-container.8853',
            'oteryn:event-container.12045',
            'oteryn:event-container.14751',
            'oteryn:event-container.14759',
            'oteryn:event-container.37457',
            'oteryn:event-container.37461',
            'oteryn:event-container.37462',
            'oteryn:event-container.37463',
            'oteryn:event-container.37465',
            'oteryn:event-container.37466',
            'oteryn:event-container.37467',
            'oteryn:event-container.37468',
            'oteryn:event-container.37469',
            'oteryn:event-container.37614',
            'oteryn:event-container.39704',
            'oteryn:event-container.39705',
            'oteryn:event-container.39706',
            'oteryn:event-container.39710')
    AND (before_item->>'definition_production_key')
        IS NOT DISTINCT FROM ('oteryn:item.tibia.i' || split_part(recipe,'.',2))
    AND jsonb_typeof(event_source_location) IS NOT DISTINCT FROM 'object'
    AND octet_length(event_source_location::text) BETWEEN 2 AND 4096
    AND event_source_location ?& ARRAY['item_instance_id','parent_item_instance_id',
        'world_id','placement_ordinal','placed_transaction_id']
));

DO $migration$
DECLARE body TEXT; before TEXT; after TEXT; arm TEXT; begin_at INT;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    before := $old$OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                    ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.source_ordinal)$old$;
    after := $new$OR NOT (
        (NEW.event_source_location IS NULL AND EXISTS (
            SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
            WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
                AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.source_ordinal))
        OR (NEW.event_source_location IS NOT NULL AND EXISTS (
            SELECT 1 FROM game_item_nested_entries e
            JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true
            JOIN game_item_container_slots b ON b.character_id=NEW.character_id
            WHERE e.item_instance_id=i.item_instance_id AND to_jsonb(e)=NEW.event_source_location
                AND e.world_id=NEW.world_id AND e.placement_ordinal=NEW.source_ordinal
                AND owner.owner_character=NEW.character_id AND owner.depth BETWEEN 1 AND 7
                AND b.item_instance_id=NEW.parent_item_instance_id AND b.world_id=NEW.world_id))
    )$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected nested event craft source guard'; END IF;
    body := replace(body,before,after);
    before := 'NEW.consume_source AND (i.quantity=1 OR NEW.recipe IN (';
    after := 'NEW.event_source_location IS NULL AND NEW.consume_source AND (i.quantity=1 OR NEW.recipe IN (';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected nested event craft ordinal guard'; END IF;
    EXECUTE replace(body,before,after);

    body := pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    before := $old$OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))$old$;
    after := $new$OR (i.quantity=0 AND (
        EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected nested event craft retirement proof'; END IF;
    body := replace(body,before,after);
    before := $old$OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=r.source_ordinal))$old$;
    after := $new$OR (i.quantity>0 AND NOT (
        (r.event_source_location IS NULL AND EXISTS(
            SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id
                AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id
                AND placement_ordinal=r.source_ordinal))
        OR (r.event_source_location IS NOT NULL AND EXISTS(
            SELECT 1 FROM game_item_nested_entries e
            JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true
            WHERE e.item_instance_id=i.item_instance_id AND to_jsonb(e)=r.event_source_location
                AND e.world_id=r.world_id AND e.placement_ordinal=r.source_ordinal
                AND owner.owner_character=r.character_id AND owner.depth BETWEEN 1 AND 7))
    ))$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected nested event craft retained-source proof'; END IF;
    EXECUTE replace(body,before,after);

    body := pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected nested event removal guard'; END IF;
    arm := $arm$
    IF TG_OP='DELETE' AND EXISTS (
        SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i
            ON i.item_instance_id=r.source_item_instance_id
        WHERE r.source_item_instance_id=OLD.item_instance_id
            AND r.event_source_location=to_jsonb(OLD) AND r.world_id=OLD.world_id
            AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
            AND r.created_xact_id=pg_current_xact_id())
    THEN RETURN NULL; END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;
