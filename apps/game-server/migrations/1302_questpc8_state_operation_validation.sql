-- QUESTPC8 extends the shared validator with closed shapes and elapsed-less-than.
-- Shared participant and state-operation triggers remain installed unchanged.
CREATE OR REPLACE FUNCTION game_quest_reward_effects_valid(
    v_plan JSONB, v_keys TEXT[], v_before BIGINT[], v_after BIGINT[]
) RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
DECLARE
    v_effect JSONB; v_index INTEGER; v_expected NUMERIC; v_condition BOOLEAN;
    v_seconds NUMERIC; v_kind TEXT;
    v_now NUMERIC := floor(extract(epoch FROM transaction_timestamp()));
BEGIN
    IF jsonb_typeof(v_plan->'storage') IS DISTINCT FROM 'array'
        OR jsonb_typeof(coalesce(v_plan->'state_ops','[]'::jsonb)) IS DISTINCT FROM 'array'
        OR v_keys IS NULL OR v_before IS NULL OR v_after IS NULL THEN RETURN false; END IF;
    IF cardinality(v_keys) NOT BETWEEN 1 AND 8
        OR array_ndims(v_keys) IS DISTINCT FROM 1 OR array_lower(v_keys,1) IS DISTINCT FROM 1
        OR array_ndims(v_before) IS DISTINCT FROM 1 OR array_lower(v_before,1) IS DISTINCT FROM 1
        OR array_ndims(v_after) IS DISTINCT FROM 1 OR array_lower(v_after,1) IS DISTINCT FROM 1
        OR cardinality(v_before)<>cardinality(v_keys) OR cardinality(v_after)<>cardinality(v_keys)
        OR array_position(v_keys,NULL) IS NOT NULL OR array_position(v_before,NULL) IS NOT NULL
        OR array_position(v_after,NULL) IS NOT NULL
        OR cardinality(v_keys)<>jsonb_array_length(v_plan->'storage')
            +jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))
        OR cardinality(v_keys)<>(SELECT count(DISTINCT key) FROM unnest(v_keys) key)
        THEN RETURN false;
    END IF;
    IF EXISTS (
        SELECT 1 FROM (
            SELECT value FROM jsonb_array_elements(v_plan->'storage')
            UNION ALL SELECT value FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb))
        ) declared WHERE jsonb_typeof(declared.value) IS DISTINCT FROM 'object'
            OR jsonb_typeof(declared.value->'track') IS DISTINCT FROM 'string'
            OR octet_length(declared.value->>'track') NOT BETWEEN 1 AND 128
    ) THEN RETURN false; END IF;
    IF cardinality(v_keys)<>(SELECT count(DISTINCT effect.value->>'track') FROM (
        SELECT value FROM jsonb_array_elements(v_plan->'storage')
        UNION ALL SELECT value FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb))
    ) effect) THEN RETURN false; END IF;
    FOR v_effect IN SELECT value FROM jsonb_array_elements(v_plan->'storage') LOOP
        IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect) key
            WHERE key NOT IN ('track','minimum','maximum','set','preserve','read_only'))
            OR jsonb_typeof(v_effect->'minimum') IS DISTINCT FROM 'number'
            OR jsonb_typeof(v_effect->'maximum') IS DISTINCT FROM 'number'
            OR jsonb_typeof(v_effect->'set') IS DISTINCT FROM 'number'
            OR (v_effect ? 'preserve' AND jsonb_typeof(v_effect->'preserve') IS DISTINCT FROM 'boolean')
            OR (v_effect ? 'read_only' AND jsonb_typeof(v_effect->'read_only') IS DISTINCT FROM 'boolean')
            OR NOT ((v_effect->>'minimum') ~ '^-?(0|[1-9][0-9]*)$'
                AND (v_effect->>'maximum') ~ '^-?(0|[1-9][0-9]*)$'
                AND (v_effect->>'set') ~ '^-?(0|[1-9][0-9]*)$')
            THEN RETURN false; END IF;
        v_index:=array_position(v_keys,v_effect->>'track');
        IF v_index IS NULL OR (v_effect->>'minimum')::bigint>(v_effect->>'maximum')::bigint
            OR v_before[v_index] NOT BETWEEN (v_effect->>'minimum')::bigint AND (v_effect->>'maximum')::bigint
            THEN RETURN false; END IF;
        IF coalesce((v_effect->>'preserve')::boolean,false) THEN v_expected:=v_before[v_index];
        ELSE v_expected:=(v_effect->>'set')::bigint; END IF;
        IF v_expected IS DISTINCT FROM v_after[v_index]::numeric THEN RETURN false; END IF;
    END LOOP;
    FOR v_effect IN SELECT value FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb)) LOOP
        IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect) key WHERE key NOT IN ('track','from','effect'))
            OR jsonb_typeof(v_effect->'from') IS DISTINCT FROM 'object'
            OR jsonb_typeof(v_effect->'effect') IS DISTINCT FROM 'object'
            THEN RETURN false; END IF;
        v_index:=array_position(v_keys,v_effect->>'track');
        IF v_index IS NULL THEN RETURN false; END IF;
        v_kind:=v_effect #>> '{from,kind}';
        CASE v_kind
            WHEN 'any' THEN
                IF v_effect->'from'<>'{"kind":"any"}'::jsonb THEN RETURN false; END IF;
                v_condition:=true;
            WHEN 'between' THEN
                IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect->'from') key WHERE key NOT IN ('kind','minimum','maximum'))
                    OR jsonb_typeof(v_effect #> '{from,minimum}') IS DISTINCT FROM 'number'
                    OR jsonb_typeof(v_effect #> '{from,maximum}') IS DISTINCT FROM 'number'
                    OR NOT ((v_effect #>> '{from,minimum}') ~ '^-?(0|[1-9][0-9]*)$'
                        AND (v_effect #>> '{from,maximum}') ~ '^-?(0|[1-9][0-9]*)$')
                    THEN RETURN false; END IF;
                v_condition:=(v_effect #>> '{from,minimum}')::bigint <= (v_effect #>> '{from,maximum}')::bigint
                    AND v_before[v_index] BETWEEN (v_effect #>> '{from,minimum}')::bigint AND (v_effect #>> '{from,maximum}')::bigint;
            WHEN 'elapsed_at_least', 'elapsed_less_than' THEN
                IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect->'from') key WHERE key NOT IN ('kind','seconds'))
                    OR jsonb_typeof(v_effect #> '{from,seconds}') IS DISTINCT FROM 'number'
                    OR NOT (v_effect #>> '{from,seconds}') ~ '^(0|[1-9][0-9]*)$'
                    THEN RETURN false; END IF;
                v_seconds:=(v_effect #>> '{from,seconds}')::numeric;
                IF v_seconds NOT BETWEEN 1 AND 31622400 THEN RETURN false; END IF;
                IF v_kind='elapsed_less_than' THEN
                    v_condition:=v_before[v_index]>0 AND v_now>=v_before[v_index]::numeric
                        AND v_now-v_before[v_index]::numeric < v_seconds;
                ELSE v_condition:=v_now-v_before[v_index]::numeric >= v_seconds; END IF;
            ELSE RETURN false;
        END CASE;
        IF NOT coalesce(v_condition,false) THEN RETURN false; END IF;
        CASE v_effect #>> '{effect,kind}'
            WHEN 'add' THEN
                IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect->'effect') key WHERE key NOT IN ('kind','value'))
                    OR jsonb_typeof(v_effect #> '{effect,value}') IS DISTINCT FROM 'number'
                    OR NOT (v_effect #>> '{effect,value}') ~ '^-?(0|[1-9][0-9]*)$'
                    THEN RETURN false; END IF;
                v_expected:=v_before[v_index]::numeric+(v_effect #>> '{effect,value}')::bigint;
            WHEN 'set_now' THEN
                IF v_effect->'effect'<>'{"kind":"set_now"}'::jsonb THEN RETURN false; END IF;
                v_expected:=v_now;
            ELSE RETURN false;
        END CASE;
        IF v_expected NOT BETWEEN -9223372036854775808::numeric AND 9223372036854775807::numeric
            OR v_expected IS DISTINCT FROM v_after[v_index]::numeric THEN RETURN false; END IF;
    END LOOP;
    RETURN true;
EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN RETURN false;
END $$;

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_effects_valid(JSONB,TEXT[],BIGINT[],BIGINT[]) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_effects_valid(JSONB,TEXT[],BIGINT[],BIGINT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_effects_valid(JSONB,TEXT[],BIGINT[],BIGINT[]) TO oteryn_game_runtime,oteryn_game_control;
