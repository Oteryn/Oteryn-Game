-- questpc3: four exact repeatable Rookgaard exchanges use the existing craft
-- receipt, source/custody guards, audit and deferred debit+mint completion proof.
-- Forward rollback: disable the NPC adapters; retain immutable receipts.
ALTER TABLE game_item_craft_records ADD COLUMN npc_exchange JSONB;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_npc_exchange_check CHECK (
    (npc_exchange IS NULL AND recipe NOT LIKE 'oteryn:npc-quest-exchange.%')
    OR (npc_exchange IS NOT NULL AND (
        jsonb_typeof(npc_exchange) = 'object'
        AND npc_exchange ?& ARRAY['npc_key','npc_identity','npc_generation',
            'catalogue_revision','conversation_revision']
        AND jsonb_typeof(npc_exchange->'npc_identity') = 'array'
        AND jsonb_array_length(npc_exchange->'npc_identity') = 16
        AND (npc_exchange->>'npc_generation')::numeric BETWEEN 1 AND 18446744073709551615
        AND (npc_exchange->>'catalogue_revision')::numeric BETWEEN 1 AND 18446744073709551615
        AND (npc_exchange->>'conversation_revision')::numeric BETWEEN 1 AND 18446744073709551615
        AND recipe = 'oteryn:npc-quest-exchange.' || substring(npc_exchange->>'npc_key' FROM 12)
        AND consume_source AND ingredient_item_instance_id IS NULL
        AND before_item->>'definition_family' = 'Item'
        AND before_item->>'definition_revision_ref' = 'definition-r1'
        AND jsonb_array_length(outputs) = 1
        AND outputs->0->>'count' = '1'
        AND outputs->0->>'revision' = 'definition-r1'
        AND (npc_exchange->>'npc_key', before_item->>'definition_production_key', outputs->0->>'key') IN (
            ('oteryn:npc.amber','oteryn:item.tibia.i2821','oteryn:item.tibia.i3294'),
            ('oteryn:npc.billy','oteryn:item.tibia.i3466','oteryn:item.tibia.i7876'),
            ('oteryn:npc.lee_delle','oteryn:item.tibia.i2984','oteryn:item.tibia.i3362'),
            ('oteryn:npc.willie','oteryn:item.tibia.i3587','oteryn:item.tibia.i3426'))
    ) IS TRUE)
);

-- Reuse the existing nested-source proof, including exact location snapshot,
-- bounded carried ancestry and its deferred retirement/custody checks.
DO $migration$
DECLARE expression TEXT; body TEXT; before TEXT; after TEXT;
BEGIN
    SELECT pg_get_expr(conbin, conrelid) INTO STRICT expression FROM pg_constraint
        WHERE conrelid='game_item_craft_records'::regclass
            AND conname='game_item_craft_event_source_location_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
    EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check CHECK ((%s) OR (npc_exchange IS NOT NULL AND jsonb_typeof(event_source_location) IS NOT DISTINCT FROM ''object'' AND octet_length(event_source_location::text) BETWEEN 2 AND 4096 AND event_source_location ?& ARRAY[''item_instance_id'',''parent_item_instance_id'',''world_id'',''placement_ordinal'',''placed_transaction_id'']))',expression);

    -- The notebook earned from Short Sword's real chest has text. Keep every
    -- other plain-input restriction and allow only Amber's typed notebook debit.
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    before := 'OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)';
    after := $new$OR (EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
        AND NOT (NEW.npc_exchange IS NOT NULL AND NEW.recipe='oteryn:npc-quest-exchange.amber'
            AND i.definition_production_key='oteryn:item.tibia.i2821'))$new$;
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft text input guard'; END IF;
    EXECUTE replace(body,before,after);
END $migration$;
