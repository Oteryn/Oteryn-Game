-- L2: a costed repeat may award XP through the existing same-transaction obligation owner.
-- Preserve canonical input, payment, item MINT, participant and earned-value proofs.
CREATE FUNCTION game_quest_reward_has_experience(
    v_character UUID, v_cause UUID, v_ordinal NUMERIC, v_transition TEXT, v_plan JSONB
) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT CASE WHEN jsonb_typeof(v_plan->'experience')='number'
        AND v_plan->>'experience' ~ '^[1-9][0-9]{0,8}$'
    THEN (v_plan->>'experience')::bigint BETWEEN 1 AND 100000000
        AND EXISTS(SELECT 1 FROM game_character_quest_xp_obligations x
            WHERE x.character_id=v_character AND x.cause_id=v_cause
                AND x.cause_ordinal=v_ordinal AND x.transition_key=v_transition
                AND x.amount=(v_plan->>'experience')::bigint
                AND x.created_xact_id=pg_current_xact_id())
    ELSE FALSE END
$$;

DO $migration$
DECLARE body TEXT; anchor TEXT:=$old$OR jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object'$old$;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'quest XP repeat completion anchor changed'; END IF;
    body:=replace(body,anchor,$new$OR (jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object'
        AND NOT game_quest_reward_has_experience(NEW.character_id,NEW.cause_id,
            NEW.cause_ordinal,NEW.transition_key,NEW.plan))$new$);
    EXECUTE replace(body,'quest repeat must be a costed item exchange',
        'quest repeat must consume value for an item or proven experience');
END $migration$;
