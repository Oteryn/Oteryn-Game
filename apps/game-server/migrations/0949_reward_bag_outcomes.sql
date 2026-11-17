-- SV11: Christmas bundles and anniversary boxes use the existing atomic craft receipt.
-- Only captured event recipes may exceed the ordinary four-output limit.
-- Forward rollback: disable these recipes; retain their immutable receipts and audit rows.
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
            OR (consume_source AND recipe='oteryn:soulpit.entry')));
