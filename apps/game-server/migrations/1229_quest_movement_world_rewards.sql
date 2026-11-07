-- Movement ingress uses the same exact native MINT participant as USE. Bind
-- each source receipt kind to its callback trigger; existing proof arms stay intact.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    original:=$old$r.cause_kind='use'$old$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed world reward cause proof';
    END IF;
    body:=replace(body,original,$new$r.cause_kind IN ('use','step')
            AND (r.cause_kind='step')=(NEW.plan #>> '{world_callback,trigger}'='step_in')$new$);
    original:=$old$AND NEW.plan #>> '{world_callback,trigger}' IN ('use','use_carried_on_target')$old$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed world reward trigger proof';
    END IF;
    body:=replace(body,original,$new$AND NEW.plan #>> '{world_callback,trigger}' IN ('use','step_in','use_carried_on_target')$new$);
    original:=$old$NEW.plan #>> '{world_callback,trigger}'='use'$old$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed plain world reward proof';
    END IF;
    EXECUTE replace(body,original,$new$NEW.plan #>> '{world_callback,trigger}' IN ('use','step_in')$new$);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
