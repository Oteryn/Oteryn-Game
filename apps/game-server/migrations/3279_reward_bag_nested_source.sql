-- The generated Red Christmas Bundle recipe already admits seven outputs.
-- Its nested source uses the existing exact-row custody, retirement and debit triggers.
DO $migration$
DECLARE previous TEXT;
BEGIN
    SELECT pg_get_expr(conbin, conrelid) INTO STRICT previous
    FROM pg_constraint
    WHERE conrelid = 'game_item_craft_records'::regclass
      AND conname = 'game_item_craft_event_source_location_check';

    ALTER TABLE game_item_craft_records
        DROP CONSTRAINT game_item_craft_event_source_location_check;
    EXECUTE format($constraint$
        ALTER TABLE game_item_craft_records
            ADD CONSTRAINT game_item_craft_event_source_location_check
            CHECK ((%s) OR (
                recipe = 'oteryn:reward-bag.event_6506'
                AND consume_source
                AND ingredient_item_instance_id IS NULL
                AND before_item->>'definition_production_key'
                    IS NOT DISTINCT FROM 'oteryn:item.tibia.i6506'
                AND jsonb_array_length(outputs) = 7
                AND jsonb_typeof(event_source_location) IS NOT DISTINCT FROM 'object'
                AND octet_length(event_source_location::text) BETWEEN 2 AND 4096
                AND event_source_location ?& ARRAY['item_instance_id', 'parent_item_instance_id',
                    'world_id', 'placement_ordinal', 'placed_transaction_id']
            ))
    $constraint$, previous);
END $migration$;
