-- Reused from questpc3 9936a44dd:1226_quest_carried_exchange_inputs.sql.
-- Explicit opt-in preserves the original no-delivery carried-tool contract.
-- Extend the unchanged QENGINE2 selected-instance proof to bounded exchanges.
-- All input counts and the native MINT remain proved by quest_reward_complete.
DO $migration$
DECLARE body TEXT; original TEXT:=$old$    IF jsonb_array_length(NEW.plan->'take')<>1
        OR NEW.plan #>> '{take,0,item}' IS DISTINCT FROM NEW.plan->>'carried_item'
        OR NEW.plan #>> '{take,0,count}' IS DISTINCT FROM '1'
        OR NEW.plan->'give' IS DISTINCT FROM 'null'::jsonb
        OR NEW.created_xact_id<>pg_current_xact_id() THEN$old$;
BEGIN
    body:=pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed carried quest input proof';
    END IF;
    EXECUTE replace(body,original,$new$    IF jsonb_typeof(NEW.plan->'take') IS DISTINCT FROM 'array'
        OR jsonb_array_length(NEW.plan->'take') NOT BETWEEN 1 AND 8
        OR (SELECT count(*) FROM jsonb_array_elements(NEW.plan->'take') e
            WHERE e->>'item'=NEW.plan->>'carried_item' AND e->>'count'='1')<>1
        OR (SELECT count(DISTINCT e->>'item') FROM jsonb_array_elements(NEW.plan->'take') e)
            <>jsonb_array_length(NEW.plan->'take')
        OR EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
            WHERE CASE WHEN e->>'count' ~ '^[0-9]{1,3}$'
                THEN (e->>'count')::integer NOT BETWEEN 1 AND 100 ELSE true END)
        OR (CASE WHEN NEW.plan->'carried_exchange'='true'::jsonb THEN
            jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object'
        ELSE jsonb_array_length(NEW.plan->'take')<>1
            OR NEW.plan->'give' IS DISTINCT FROM 'null'::jsonb END)
        OR NEW.created_xact_id<>pg_current_xact_id() THEN$new$);
    EXECUTE format('ALTER FUNCTION game_quest_carried_use_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
