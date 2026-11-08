DO $probe$
DECLARE
    stamp BIGINT:=floor(extract(epoch FROM transaction_timestamp()));
    plan JSONB:='{"storage":[],"state_ops":[{"track":"oteryn:clock","from":{"kind":"elapsed_less_than","seconds":1800},"effect":{"kind":"add","value":0}}]}'::jsonb;
    variant JSONB; value BIGINT;
BEGIN
    FOREACH value IN ARRAY ARRAY[0::bigint,-1::bigint,stamp+1,stamp-1800] LOOP
        IF game_quest_reward_effects_valid(plan,ARRAY['oteryn:clock'],ARRAY[value],ARRAY[value]) THEN
            RAISE EXCEPTION 'elapsed_less_than admitted invalid stamp %',value;
        END IF;
    END LOOP;
    value:=stamp-1799;
    IF NOT game_quest_reward_effects_valid(plan,ARRAY['oteryn:clock'],ARRAY[value],ARRAY[value]) THEN
        RAISE EXCEPTION 'elapsed_less_than refused1799s';
    END IF;
    FOR variant IN SELECT jsonb_set(plan,'{state_ops,0,from,seconds}',number) FROM unnest(ARRAY['0'::jsonb,'-1'::jsonb,'1800.5'::jsonb,'31622401'::jsonb,'"1800"'::jsonb]) number LOOP
        IF game_quest_reward_effects_valid(variant,ARRAY['oteryn:clock'],ARRAY[value],ARRAY[value]) THEN
            RAISE EXCEPTION 'elapsed_less_than admitted nonnative bound';
        END IF;
    END LOOP;
    variant:=jsonb_set(plan,'{state_ops,0,from,client_time}','1'::jsonb);
    IF game_quest_reward_effects_valid(variant,ARRAY['oteryn:clock'],ARRAY[value],ARRAY[value]) THEN RAISE EXCEPTION 'unknown guard field admitted'; END IF;
    variant:=jsonb_set(plan,'{state_ops,0,effect,client_time}','1'::jsonb);
    IF game_quest_reward_effects_valid(variant,ARRAY['oteryn:clock'],ARRAY[value],ARRAY[value]) THEN RAISE EXCEPTION 'unknown effect field admitted'; END IF;
    variant:=jsonb_set(plan,'{state_ops,0,from}','{"kind":"any"}'::jsonb);
    variant:=jsonb_set(variant,'{state_ops,0,effect}','{"kind":"set_now"}'::jsonb);
    IF NOT game_quest_reward_effects_valid(variant,ARRAY['oteryn:clock'],ARRAY[0::bigint],ARRAY[stamp])
        OR game_quest_reward_effects_valid(variant,ARRAY['oteryn:clock'],ARRAY[0::bigint],ARRAY[stamp+1]) THEN
        RAISE EXCEPTION 'set_now must equal exact transaction clock';
    END IF;
    RAISE NOTICE 'POSTGRES_STATE_OPS_SQL_SHAPE_AND_DEADLINE_BOUNDARIES_PASS';
END $probe$;
