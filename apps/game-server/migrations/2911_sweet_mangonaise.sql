-- Keep the existing bounds for all other consume/transform actions.
ALTER TABLE game_item_consume_transform_records
    DROP CONSTRAINT game_item_consume_transform_records_outputs_check,
    ADD CONSTRAINT game_item_consume_transform_records_outputs_check CHECK (
        jsonb_typeof(outputs) = 'array' AND
        CASE
            WHEN recipe = 'oteryn:novelty-item.sweet-mangonaise'
                THEN jsonb_array_length(outputs) = 10
                    AND consume_source AND source_count = 1
                    AND before_item->>'definition_production_key' IS NOT DISTINCT FROM 'oteryn:item.tibia.i11588'
            WHEN recipe LIKE 'oteryn:event-container.%' THEN jsonb_array_length(outputs) BETWEEN 0 AND 62
            ELSE jsonb_array_length(outputs) BETWEEN 0 AND 1
        END
    );

CREATE INDEX game_item_consume_transform_sweet_mangonaise_cooldown
    ON game_item_consume_transform_records (character_id, occurred_at DESC)
    WHERE recipe = 'oteryn:novelty-item.sweet-mangonaise';

REVOKE ALL ON game_item_consume_transform_records FROM PUBLIC;
GRANT SELECT, INSERT ON game_item_consume_transform_records TO oteryn_game_runtime;
GRANT SELECT ON game_item_consume_transform_records TO oteryn_game_control;
