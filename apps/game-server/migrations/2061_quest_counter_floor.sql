-- M07: checked source counters initialise max(old + delta, minimum).
-- Patch only the shared effect branch; retain all evolved capacity/shape guards.
DO $m07$
DECLARE
    v_function TEXT := 'game_quest_reward_effects_valid(jsonb,text[],bigint[],bigint[])';
    v_old TEXT := 'WHEN ''set_now'' THEN v_expected:=v_now;';
    v_new TEXT := $branch$
            WHEN 'add_at_least' THEN
                IF jsonb_typeof(v_effect #> '{effect,value}') IS DISTINCT FROM 'number'
                    OR jsonb_typeof(v_effect #> '{effect,minimum}') IS DISTINCT FROM 'number'
                    OR NOT coalesce((v_effect #>> '{effect,value}') ~ '^-?(0|[1-9][0-9]*)$'
                        AND (v_effect #>> '{effect,minimum}') ~ '^-?(0|[1-9][0-9]*)$', false)
                    THEN RETURN false; END IF;
                IF (v_effect #>> '{effect,value}')::numeric NOT BETWEEN -9223372036854775808 AND 9223372036854775807
                    OR (v_effect #>> '{effect,minimum}')::numeric NOT BETWEEN -9223372036854775808 AND 9223372036854775807
                    THEN RETURN false; END IF;
                v_expected := v_before[v_index]::numeric + (v_effect #>> '{effect,value}')::numeric;
                -- A floor must never hide native signed-addition overflow.
                IF v_expected NOT BETWEEN -9223372036854775808 AND 9223372036854775807
                    THEN RETURN false; END IF;
                v_expected := greatest(v_expected, (v_effect #>> '{effect,minimum}')::numeric);
            WHEN 'set_now' THEN v_expected:=v_now;
$branch$;
    v_body TEXT;
BEGIN
    v_body := pg_get_functiondef(v_function::regprocedure);
    IF (length(v_body)-length(replace(v_body,v_old,'')))/length(v_old) <> 1 THEN
        RAISE EXCEPTION 'missing or ambiguous source counter effect branch';
    END IF;
    EXECUTE replace(v_body,v_old,v_new);
    EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',
                   v_function,current_schema());
END $m07$;
