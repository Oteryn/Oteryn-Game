-- QSTORE extends the existing Quest owner without replacing evolved shape,
-- capacity, checked-counter, participant, receipt or session-fence guards.
DO $qstore$
DECLARE
    v_function TEXT := 'game_quest_reward_effects_valid(jsonb,text[],bigint[],bigint[])';
    v_old TEXT := 'WHEN ''elapsed_at_least'', ''elapsed_less_than'' THEN';
    v_new TEXT := $branch$
            WHEN 'bits_set', 'bits_clear' THEN
                IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect->'from') key WHERE key NOT IN ('kind','mask'))
                    OR jsonb_typeof(v_effect #> '{from,mask}') IS DISTINCT FROM 'number'
                    OR NOT (v_effect #>> '{from,mask}') ~ '^[1-9][0-9]*$'
                    THEN RETURN false; END IF;
                IF (v_effect #>> '{from,mask}')::bigint <= 0 THEN RETURN false; END IF;
                IF v_kind='bits_set' THEN
                    v_condition:=(v_before[v_index] & (v_effect #>> '{from,mask}')::bigint)=(v_effect #>> '{from,mask}')::bigint;
                ELSE v_condition:=(v_before[v_index] & (v_effect #>> '{from,mask}')::bigint)=0; END IF;
            WHEN 'elapsed_at_least', 'elapsed_less_than' THEN$branch$;
    v_body TEXT;
BEGIN
    v_body := pg_get_functiondef(v_function::regprocedure);
    IF (length(v_body)-length(replace(v_body,v_old,'')))/length(v_old) <> 1 THEN
        RAISE EXCEPTION 'missing or ambiguous quest storage condition branch';
    END IF;
    EXECUTE replace(v_body,v_old,v_new);
    EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',
                   v_function,current_schema());
END $qstore$;
