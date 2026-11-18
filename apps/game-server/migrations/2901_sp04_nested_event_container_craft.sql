-- Generated SP04 event bags use the same exact nested custody proof as W19.
-- Retain the existing recipes and restrict the new receipt shape to captured bags.
DO $migration$
DECLARE previous TEXT;
BEGIN
    SELECT pg_get_expr(conbin, conrelid) INTO STRICT previous
      FROM pg_constraint
     WHERE conrelid = 'game_item_craft_records'::regclass
       AND conname = 'game_item_craft_event_source_location_check';

    ALTER TABLE game_item_craft_records
        DROP CONSTRAINT game_item_craft_event_source_location_check;
    EXECUTE format($ddl$
        ALTER TABLE game_item_craft_records
            ADD CONSTRAINT game_item_craft_event_source_location_check
            CHECK ((%s) OR (
                consume_source
                AND split_part(recipe, 'oteryn:reward-bag.event_', 2) IN (
                    '6506', '6507', '6508', '6570', '6571', '8853', '12045',
                    '14751', '14759', '21203', '37457', '37461', '37462',
                    '37463', '37465', '37466', '37467', '37468', '37469',
                    '37614', '39704', '39705', '39706', '39710')
                AND recipe = 'oteryn:reward-bag.event_'
                    || split_part(recipe, 'oteryn:reward-bag.event_', 2)
                AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM
                    ('oteryn:item.tibia.i' || split_part(recipe, 'oteryn:reward-bag.event_', 2))
                AND jsonb_typeof(event_source_location) IS NOT DISTINCT FROM 'object'
                AND octet_length(event_source_location::text) BETWEEN 2 AND 4096
                AND event_source_location ?& ARRAY['item_instance_id', 'parent_item_instance_id',
                    'world_id', 'placement_ordinal', 'placed_transaction_id']
            ))
    $ddl$, previous);
END $migration$;
