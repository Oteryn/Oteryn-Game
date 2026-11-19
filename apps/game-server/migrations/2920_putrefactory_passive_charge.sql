-- Crystal 201189f8, creaturescripts_putrefactory-x.lua:41-51,140-153.
-- Keep all existing source-think receipt shapes and admit the bounded passive tick.
DO $migration$
DECLARE
    constraint_name text;
    prior text;
    arm constant text := $predicate$(
        cause_kind = 'source_think' AND cause_ordinal = 0
        AND game_character_is_uuid_v7(cause_id)
        AND quest_key = 'oteryn:quest/rotten-blood'
        AND transition_key = 'oteryn:quest-transition/rotten-blood/putrefactory/passive-tick'
        AND completes = false AND octet_length(request_binding) = 33
        AND get_byte(request_binding, 0) = 1
        AND track_keys = ARRAY[
            'oteryn:quest-progress/crystalserver/quest/u13_20/rotten_blood/putrefactory_task',
            'oteryn:quest-progress/rotten-blood/putrefactory/last-passive-tick'
        ]
        AND cardinality(values_before) = 2 AND cardinality(values_after) = 2
        AND values_before[1] BETWEEN 0 AND 99
        AND values_after[1] = values_before[1]::numeric + 1
        AND values_before[2] >= 0
        AND values_after[2]::numeric >= values_before[2]::numeric + 20
    )$predicate$;
BEGIN
    FOREACH constraint_name IN ARRAY ARRAY[
        'game_character_quest_receipt_cause_shape',
        'game_character_quest_receipts_cause_kind_check'
    ] LOOP
        SELECT pg_get_constraintdef(oid) INTO STRICT prior
        FROM pg_constraint
        WHERE conrelid = 'game_character_quest_receipts'::regclass
          AND conname = constraint_name;
        IF left(prior, 6) <> 'CHECK ' OR strpos(prior, 'source_think') = 0 THEN
            RAISE EXCEPTION 'unexpected source-think receipt constraint: %', constraint_name;
        END IF;
        EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I', constraint_name);
        EXECUTE format(
            'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT %I CHECK ((%s OR %s) IS TRUE)',
            constraint_name, substr(prior, 7), arm
        );
    END LOOP;
END $migration$;

-- Crystal 201189f8, actions_cake_bite.lua:28-34. The fenced Cake USE owner
-- reads the cooldown from the existing eater row after eligibility locks it.
GRANT SELECT (world_id, event_year, character_id, last_bite_at)
ON game_cake_eaters TO oteryn_game_runtime;
