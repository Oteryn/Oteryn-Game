-- ENGNPC: promote QUESTL2's bounded sixteen-effect receipt capacity.
-- Extend the installed guard rather than replacing the composed value-only
-- reward, WORLDCHG and historical NPC arms from migrations 1314/1333/1349.
-- All array shape, same-transaction proof and receipt-chain guards remain intact.
DO $migration$
DECLARE
    body TEXT;
    needle CONSTANT TEXT := 'cardinality(track_keys) <= 8';
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT body
      FROM pg_constraint
     WHERE conrelid = 'game_character_quest_receipts'::regclass
       AND conname = 'game_character_quest_receipt_effects_shape';
    IF (length(body) - length(replace(body, needle, ''))) / length(needle) <> 1 THEN
        RAISE EXCEPTION 'missing or ambiguous canonical quest receipt effect capacity';
    END IF;
    ALTER TABLE game_character_quest_receipts
        DROP CONSTRAINT game_character_quest_receipt_effects_shape;
    EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT '
        || 'game_character_quest_receipt_effects_shape '
        || replace(body, needle, 'cardinality(track_keys) <= 16');
END
$migration$;
