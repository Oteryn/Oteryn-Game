-- WVAR exact retained source capacities: Insectoid Worker44, Spectre/Poacher40,
-- Hive Overseer/Kollos/Spidris/Deepling/Crawler36. Keep the existing bounded owner;
-- preserve every receipt, lifecycle, nested-source and single-location guard.
DO $migration$
DECLARE body TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_corpse_container_entry_proven()'::regprocedure);
    IF regexp_count(body,E'\\) > 30 THEN')<>2 THEN
        RAISE EXCEPTION 'unexpected installed corpse capacity proof';
    END IF;
    body:=replace(body,') > 30 THEN',') > 44 THEN');
    EXECUTE body;
END $migration$;
ALTER TABLE game_item_decay_retire_reservations
    DROP CONSTRAINT game_item_decay_retire_reservations_placement_ordinal_check,
    ADD CONSTRAINT game_item_decay_retire_reservations_placement_ordinal_check
        CHECK (placement_ordinal BETWEEN 1 AND 44);
ALTER TABLE game_item_decay_retire_receipts
    DROP CONSTRAINT game_item_decay_retire_receipts_placement_ordinal_check,
    ADD CONSTRAINT game_item_decay_retire_receipts_placement_ordinal_check
        CHECK (placement_ordinal BETWEEN 1 AND 44);
