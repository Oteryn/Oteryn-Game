-- Main1258 replaced the old cause-shape name while adding server_timer.
-- Preserve that complete predicate under the name the following unchanged
-- native source-think migration extends. No cause gains authority here.
DO $$
DECLARE prior TEXT; expected TEXT; prior_ast TEXT; expected_ast TEXT;
BEGIN
    IF EXISTS(SELECT 1 FROM pg_constraint
        WHERE conrelid='game_character_quest_receipts'::regclass
            AND conname='game_character_quest_receipt_cause_shape') THEN
        RAISE EXCEPTION 'expected only main quest timer ordinal constraint';
    END IF;
    CREATE TEMP TABLE srx3_quest_cause_shape_reference
        (LIKE game_character_quest_receipts) ON COMMIT DROP;
    ALTER TABLE srx3_quest_cause_shape_reference
        ADD CONSTRAINT srx3_quest_cause_shape_expected
        CHECK ((cause_kind IN ('creature_death','server_timer'))=(cause_ordinal=0));
    SELECT pg_get_constraintdef(oid),conbin::text INTO STRICT prior,prior_ast
        FROM pg_constraint WHERE conrelid='game_character_quest_receipts'::regclass
            AND conname='game_quest_timer_cause_ordinal' AND contype='c';
    SELECT pg_get_constraintdef(oid),conbin::text INTO STRICT expected,expected_ast
        FROM pg_constraint WHERE conrelid='srx3_quest_cause_shape_reference'::regclass
            AND conname='srx3_quest_cause_shape_expected' AND contype='c';
    -- Parse locations differ between the main and reference DDL; every semantic
    -- AST field (operators, types, columns, literals and ordering) stays exact.
    IF prior IS DISTINCT FROM expected OR
        regexp_replace(prior_ast,':location -?[0-9]+',':location -1','g') IS DISTINCT FROM
        regexp_replace(expected_ast,':location -?[0-9]+',':location -1','g') THEN
        RAISE EXCEPTION 'main quest timer ordinal predicate changed';
    END IF;
    ALTER TABLE game_character_quest_receipts RENAME CONSTRAINT
        game_quest_timer_cause_ordinal TO game_character_quest_receipt_cause_shape;
END $$;
