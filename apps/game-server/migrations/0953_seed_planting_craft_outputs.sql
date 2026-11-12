-- W19: 0949 replaced the outputs constraint and lost 0883's seed-only arm.
-- A map planting consumes its seed and changes terrain without minting an Item.
-- Keep the terrain receipt/source admission trigger and all 0949 limits intact.
-- Forward rollback: disable seed USE; retain immutable receipts and audit rows.
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
    CHECK (jsonb_typeof(outputs)='array'
        AND (jsonb_array_length(outputs) BETWEEN 0 AND 4
            OR (consume_source AND recipe IN (
                'oteryn:reward-bag.event_6506', 'oteryn:reward-bag.event_6507',
                'oteryn:reward-bag.event_6508', 'oteryn:reward-bag.event_39704',
                'oteryn:reward-bag.event_39705', 'oteryn:reward-bag.event_39706',
                'oteryn:reward-bag.event_39710')
                AND jsonb_array_length(outputs) BETWEEN 5 AND 14))
        AND (jsonb_array_length(outputs)>0
            OR (NOT consume_source AND recipe='oteryn:gathering.crystal')
            OR (consume_source AND recipe='oteryn:soulpit.entry')
            OR (consume_source AND recipe='oteryn:planting.crystal'
                AND before_item->>'definition_production_key'='oteryn:item.tibia.i13844')));
