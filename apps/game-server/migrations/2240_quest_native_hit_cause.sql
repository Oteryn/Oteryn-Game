-- Successful native damage has its own occurrence namespace, independent of USE/death.
-- 1422/1423 renamed and extended the timer predicate with the sealed source-think
-- arm. Preserve both existing predicates, including every source-think guard.
DO $migration$
DECLARE name TEXT; prior TEXT;
BEGIN
    FOREACH name IN ARRAY ARRAY[
        'game_character_quest_receipts_cause_kind_check',
        'game_character_quest_receipt_cause_shape'
    ] LOOP
        SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
            WHERE conrelid='game_character_quest_receipts'::regclass AND conname=name;
        IF left(prior,6)<>'CHECK ' OR strpos(prior,'source_think')=0
            OR strpos(prior,'native_hit')<>0 THEN
            RAISE EXCEPTION 'unexpected prior native-hit cause constraint: %',name;
        END IF;
        EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I',name);
        EXECUTE format('ALTER TABLE game_character_quest_receipts ADD CONSTRAINT %I CHECK (((cause_kind<>''native_hit'' AND %s) OR (cause_kind=''native_hit'' AND cause_ordinal=0)) IS TRUE)',name,substr(prior,7));
    END LOOP;
END $migration$;
