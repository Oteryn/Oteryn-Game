-- Keep the deferred proof in step with add-on-only quest writers. Legacy
-- plans continue to prove the base and every authored add-on.
DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_quest_reward_grant_keys(jsonb)'::regprocedure);
    anchor TEXT := 'FOR v_addon IN 0..2 LOOP';
BEGIN
    IF strpos(body, anchor) = 0 THEN
        RAISE EXCEPTION 'missing quest reward grant-key loop';
    END IF;
    EXECUTE replace(body, anchor, $patch$
    IF v_plan->'addon_only' = 'true'::jsonb AND v_mask = 0 THEN
        RAISE EXCEPTION 'add-on-only quest reward has a base target' USING ERRCODE='23514';
    END IF;
    FOR v_addon IN (CASE WHEN v_plan->'addon_only' = 'true'::jsonb THEN 1 ELSE 0 END)..2 LOOP
    $patch$);
END $migration$;
