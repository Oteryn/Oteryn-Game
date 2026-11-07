-- Compose selected native USE_WITH consumption and world MINT without changing
-- QENGINE2 migrations. Existing NPC and plain world USE proof arms remain intact.
DO $migration$
DECLARE body TEXT; original TEXT:=$old$AND NEW.plan #>> '{world_callback,trigger}'='use'$old$;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed world reward proof';
    END IF;
    EXECUTE replace(body,original,$new$AND NEW.plan #>> '{world_callback,trigger}' IN ('use','use_carried_on_target')$new$);
    original:=$old$AND COALESCE(NEW.plan #> '{world_callback,item_on_target}','null'::jsonb)='null'::jsonb$old$;
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed world reward input proof';
    END IF;
    EXECUTE replace(body,original,$new$AND ((NEW.plan #>> '{world_callback,trigger}'='use'
                AND COALESCE(NEW.plan #> '{world_callback,item_on_target}','null'::jsonb)='null'::jsonb)
            OR (NEW.plan #>> '{world_callback,trigger}'='use_carried_on_target'
                AND NEW.plan->'take'=jsonb_build_array(jsonb_build_object(
                    'item',NEW.plan #>> '{world_callback,item_on_target,item}','count',1))
                AND NEW.plan #>> '{world_callback,item_on_target,count}'='1'
                AND COALESCE(NEW.plan #> '{world_callback,item_on_target,replacement}','null'::jsonb)='null'::jsonb
                AND COALESCE(NEW.plan #> '{world_callback,item_on_target,target_replacement}','null'::jsonb)='null'::jsonb
                AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
                    WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                        AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                        AND c.created_xact_id=pg_current_xact_id()
                        AND convert_from(c.proof,'UTF8')::jsonb->'callback'=NEW.plan->'world_callback'
                        AND convert_from(c.proof,'UTF8')::jsonb #> '{context,carried,source}'=
                            (SELECT jsonb_agg(get_byte(uuid_send(c.item_instance_id),n) ORDER BY n)
                                FROM generate_series(0,15) AS bytes(n))
                        AND jsonb_typeof(convert_from(c.proof,'UTF8')::jsonb #> '{context,carried,target}')='object'
                        AND jsonb_array_length(convert_from(c.proof,'UTF8')::jsonb #> '{context,carried,intent}') BETWEEN 1 AND 32)))$new$);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
