-- Awash debits one plain coal through the existing craft custody and audit owner.
DO $migration$
DECLARE definition TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT definition
      FROM pg_constraint
      WHERE conrelid='game_item_craft_records'::regclass
        AND conname='game_item_craft_records_outputs_check';
    IF definition NOT LIKE 'CHECK %' THEN
        RAISE EXCEPTION 'missing craft outputs predecessor';
    END IF;
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE 'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ('
        || substring(definition FROM 7) || $check$
        OR COALESCE(consume_source AND recipe='oteryn:world-change.awash.coal'
            AND outputs='[]'::jsonb
            AND before_item->>'definition_production_key'='oteryn:item.tibia.i12600',FALSE))
    $check$;
END $migration$;

REVOKE ALL ON game_item_craft_records FROM PUBLIC;
GRANT SELECT, INSERT ON game_item_craft_records TO oteryn_game_runtime;
GRANT SELECT ON game_item_craft_records TO oteryn_game_control;
