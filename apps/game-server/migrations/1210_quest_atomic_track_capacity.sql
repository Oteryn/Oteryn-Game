-- L2: retain QENGINE2's value-only arm and every receipt proof; raise only RL-02.
-- A Father's Burden still commits all eleven effects in one receipt and revision.
DO $migration$
DECLARE definition TEXT; widened TEXT; anchor TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
        WHERE conrelid='game_character_quest_receipts'::regclass
            AND conname='game_character_quest_receipt_effects_shape';
    widened:=replace(definition,'<= 8','<= 16');
    IF widened=definition THEN RAISE EXCEPTION 'quest receipt capacity anchor changed'; END IF;
    -- Renamed QBASE Father compatibility installs its historical nonempty arm.
    -- Restore the unchanged QENGINE2 value-only arm after that shared migration.
    anchor:='((cardinality(track_keys) > 0) OR completes)';
    IF position('oteryn:quest-reward/%' IN widened)=0 THEN
        IF position(anchor IN widened)=0 THEN RAISE EXCEPTION 'quest value-only compatibility anchor changed'; END IF;
        widened:=replace(widened,anchor,'((cardinality(track_keys) > 0) OR completes OR transition_key LIKE ''oteryn:quest-reward/%'')');
    END IF;
    ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
    EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape '||widened;
END $migration$;
