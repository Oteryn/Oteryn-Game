-- The immutable audited burn is the permanent Wheel grant. Its existing
-- command receipt, custody changes and audit event commit together.
DO $$
DECLARE
    constraint_name TEXT;
    previous_rule TEXT;
BEGIN
    FOREACH constraint_name IN ARRAY ARRAY[
        'game_item_use_records_cause_kind_check', 'game_item_use_cause_shape'
    ] LOOP
        SELECT pg_get_expr(c.conbin, c.conrelid) INTO STRICT previous_rule
          FROM pg_constraint c
         WHERE c.conrelid = 'game_item_use_records'::regclass
           AND c.conname = constraint_name AND c.contype = 'c';
        EXECUTE format(
            'ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (cause_kind = 3339 AND shape = 1))',
            constraint_name, constraint_name, previous_rule
        );
    END LOOP;
END $$;

ALTER TABLE game_item_use_records
    ADD CONSTRAINT game_promotion_scroll_source CHECK (
        cause_kind <> 3339 OR (
            shape = 1 AND activity_instance_id IS NULL AND source_house_key IS NULL
            AND source_definition_family = 'Item'
            AND source_definition_production_key IN (
                'oteryn:item.tibia.i43946', 'oteryn:item.tibia.i43947', 'oteryn:item.tibia.i43948',
                'oteryn:item.tibia.i43949', 'oteryn:item.tibia.i43950'
            )
        )
    );

CREATE UNIQUE INDEX game_promotion_scroll_once
    ON game_item_use_records (character_id, source_definition_production_key)
    WHERE cause_kind = 3339;
