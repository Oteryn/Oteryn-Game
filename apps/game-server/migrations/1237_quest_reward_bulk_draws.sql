-- Server-owned per-input random draws; the immutable source plan retains each roll.
CREATE FUNCTION game_quest_reward_rolls(n integer) RETURNS integer[]
LANGUAGE plpgsql VOLATILE AS $$
DECLARE rolls integer[];
BEGIN
    IF n IS NULL OR n NOT BETWEEN 1 AND 50000 THEN RAISE EXCEPTION 'invalid bulk draw count'; END IF;
    SELECT array_agg((floor(random()*100)+1)::integer ORDER BY ordinal)
      INTO rolls FROM generate_series(1,n) ordinal;
    RETURN rolls;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_rolls(integer) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_reward_rolls(integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_rolls(integer) TO oteryn_game_runtime;

-- Extend the complete quest proof; all original cost, fee, MINT, deadline and
-- appearance proof arms continue to run after the bulk-specific checks.
DO $migration$
DECLARE body TEXT; insert_at INTEGER;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    insert_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
    IF insert_at=0 THEN RAISE EXCEPTION 'unexpected installed quest bulk proof'; END IF;
    EXECUTE overlay(body placing $arm$
    IF NEW.plan ? 'bulk' THEN
        DECLARE cfg JSONB:=NEW.plan->'bulk'; draw JSONB:=NEW.plan->'bulk_result';
            n INTEGER; bonus_count INTEGER; expected BIGINT; actual BIGINT;
            output_items BOOLEAN; source_world UUID;
        BEGIN
            IF jsonb_typeof(cfg) IS DISTINCT FROM 'object'
               OR jsonb_typeof(draw) IS DISTINCT FROM 'object'
               OR jsonb_typeof(draw->'rolls') IS DISTINCT FROM 'array'
            THEN RAISE EXCEPTION 'bulk reward lacks retained native draws' USING ERRCODE='23514'; END IF;
            n:=jsonb_array_length(draw->'rolls');
            IF n NOT BETWEEN 1 AND 50000
               OR n IS DISTINCT FROM (draw->>'input_count')::integer
               OR NOT COALESCE((cfg->>'base')::integer BETWEEN 1 AND 100,FALSE)
               OR NOT COALESCE((cfg->>'bonus')::integer BETWEEN 1 AND 65535,FALSE)
               OR NOT COALESCE((cfg->>'chance_percent')::integer BETWEEN 1 AND 100,FALSE)
               OR cfg->>'item' IS NOT DISTINCT FROM cfg->>'output'
               OR EXISTS(SELECT 1 FROM jsonb_array_elements(draw->'rolls') roll
                   WHERE jsonb_typeof(roll)<>'number' OR (roll::text)::integer NOT BETWEEN 1 AND 100)
               OR jsonb_array_length(NEW.plan->'take')<>1
               OR NEW.plan #>> '{take,0,item}' IS DISTINCT FROM cfg->>'item'
               OR (NEW.plan #>> '{take,0,count}')::integer IS DISTINCT FROM n
            THEN RAISE EXCEPTION 'bulk reward must consume its exact complete input count' USING ERRCODE='23514'; END IF;
            SELECT count(*) INTO bonus_count FROM jsonb_array_elements(draw->'rolls') roll
                WHERE (roll::text)::integer <= (cfg->>'chance_percent')::integer;
            expected:=n::bigint*(cfg->>'base')::integer+bonus_count::bigint*(cfg->>'bonus')::integer;
            SELECT sum((output->>'count')::bigint),bool_and(output->>'item'=cfg->>'output')
                INTO actual,output_items FROM jsonb_array_elements(jsonb_build_array(NEW.plan->'give') ||
                    COALESCE(NEW.plan #> '{give,backpack_outputs}','[]'::jsonb)) output;
            IF actual IS DISTINCT FROM expected OR output_items IS DISTINCT FROM TRUE
               OR (draw->>'output_count')::bigint IS DISTINCT FROM expected
            THEN RAISE EXCEPTION 'bulk reward must mint the exact retained draw outcome' USING ERRCODE='23514'; END IF;
            SELECT (c.before_item->>'world_id')::uuid INTO source_world
                FROM game_quest_callback_consumptions c
                WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                  AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                  AND c.created_xact_id=pg_current_xact_id() LIMIT 1;
            IF source_world IS NULL OR EXISTS(
                SELECT 1 FROM game_quest_callback_carried(NEW.character_id,source_world) c
                    JOIN game_item_instances i ON i.item_instance_id=c.item
                WHERE i.lifecycle=1 AND i.definition_production_key=cfg->>'item')
            THEN RAISE EXCEPTION 'bulk reward must hand in all carried matching inputs' USING ERRCODE='23514'; END IF;
        END;
    END IF;
$arm$ from insert_at for 0);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
