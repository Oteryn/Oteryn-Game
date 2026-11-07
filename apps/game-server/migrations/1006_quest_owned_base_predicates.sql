-- A storage-free base reward needs its own exact, neutral one-shot predicate.
-- Retain every existing same-R, MINT, input and immutable grant proof arm.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
    original:=$old$AND (t->>'sex' IS NULL OR t->>'sex'=coalesce(g->>'sex',v_source.plan->>'creation_sex'))) THEN$old$;
    replacement:=$new$AND (t->>'sex' IS NULL OR t->>'sex'=coalesce(g->>'sex',v_source.plan->>'creation_sex')))
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(v_targets) t
            CROSS JOIN jsonb_array_elements_text(coalesce(v_source.plan->'forbids_outfits','[]'::jsonb)) g
            WHERE t->>'outfit'=g AND (t->>'addons')::integer=0 AND t->>'sex' IS NULL) THEN$new$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected value-only quest predicate body';
    END IF;
    EXECUTE replace(body,original,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_reward_value_only_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_value_only_complete() FROM PUBLIC;
