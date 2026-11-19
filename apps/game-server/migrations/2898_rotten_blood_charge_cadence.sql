-- Rotten Blood Quest/Spoiler (curid 102953, oldid 1107075), Rotten Charges.
-- Align the durable receipt predicates with the native 20-second clock.
DO $migration$
DECLARE
    constraint_name TEXT;
    prior TEXT;
    old_cadence CONSTANT TEXT :=
        'WHEN ''oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update''::text THEN 30';
    new_cadence CONSTANT TEXT :=
        'WHEN ''oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update''::text THEN 20';
BEGIN
    FOREACH constraint_name IN ARRAY ARRAY[
        'game_character_quest_receipt_cause_shape',
        'game_character_quest_receipts_cause_kind_check'
    ] LOOP
        SELECT pg_get_constraintdef(oid) INTO STRICT prior
        FROM pg_constraint
        WHERE conrelid = 'game_character_quest_receipts'::regclass
          AND conname = constraint_name;
        IF left(prior, 6) <> 'CHECK '
            OR length(prior) - length(replace(prior, old_cadence, '')) <> 2 * length(old_cadence)
        THEN
            RAISE EXCEPTION 'unexpected Rotten Blood charge receipt constraint: %', constraint_name;
        END IF;
        EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I', constraint_name);
        EXECUTE format(
            'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT %I %s',
            constraint_name,
            replace(prior, old_cadence, new_cadence)
        );
    END LOOP;
END $migration$;
