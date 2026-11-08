-- Reuse L2's bounded sixteen-track transition capacity after main's 1349
-- composition. Keep the exact Father exception, value-only reward arm,
-- Benevola handover and every array/provenance guard unchanged.
DO $migration$
DECLARE definition TEXT; anchor TEXT := 'cardinality(track_keys) <= 8';
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
        WHERE conrelid='game_character_quest_receipts'::regclass
            AND conname='game_character_quest_receipt_effects_shape';
    IF (length(definition)-length(replace(definition,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'missing or ambiguous quest receipt capacity anchor';
    END IF;
    ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
    EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape '
        || replace(definition,anchor,'cardinality(track_keys) <= 16');
END $migration$;
