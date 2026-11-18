-- A scaled reward composes the existing paid-exchange proof. Resolve its
-- immutable amount from the same-transaction receipt's locked level snapshot;
-- all original payment, source, revision and XP-obligation guards still run.
DO $migration$
DECLARE body TEXT; declaration TEXT; anchor TEXT; scaled TEXT;
BEGIN
    body := pg_get_functiondef('game_quest_carried_repeatable_complete(game_quest_reward_sources)'::regprocedure);
    IF strpos(body,'scaled_xp NUMERIC;')>0 THEN RETURN; END IF;
    declaration := 'expected_xp_amount BIGINT; consumed BIGINT;';
    anchor := '    IF p.plan->''give''=''null''::jsonb THEN';
    IF (length(body)-length(replace(body,declaration,'')))/length(declaration)<>1
        OR (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'scaled XP requires the exact paid-exchange proof';
    END IF;
    scaled := $scaled$
    IF p.plan ? 'experience_per_level' THEN
        IF coalesce(p.plan->'experience','null'::jsonb)<>'null'::jsonb
            OR jsonb_typeof(p.plan->'experience_per_level') IS DISTINCT FROM 'number'
            OR p.plan->>'experience_per_level' !~ '^[1-9][0-9]*$'
            OR (p.plan->>'experience_per_level')::numeric NOT BETWEEN 1 AND 9223372036854775807
        THEN RETURN FALSE; END IF;
        SELECT (p.plan->>'experience_per_level')::numeric * receipt.level_before INTO scaled_xp
          FROM game_character_quest_receipts receipt
         WHERE receipt.character_id=p.character_id AND receipt.cause_id=p.cause_id
           AND receipt.cause_ordinal=p.cause_ordinal AND receipt.transition_key=p.transition_key;
        IF scaled_xp IS NULL OR scaled_xp NOT BETWEEN 1 AND 100000000
        THEN RETURN FALSE; END IF;
        expected_xp_amount := scaled_xp::bigint;
    END IF;
$scaled$;
    body := replace(body,declaration,declaration || ' scaled_xp NUMERIC;');
    EXECUTE replace(body,anchor,scaled || anchor);
END $migration$;
