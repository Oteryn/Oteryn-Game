-- Reuse PC8 d90c1bfa29fac954fbdd08e014a6473d4a33de96's world USE
-- participant without editing the published NPC migration or its later extensions.
DO $migration$
DECLARE body TEXT; original TEXT:=$old$r.quest_key=NEW.plan->>'quest' AND r.cause_kind='command'$old$;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed quest value occurrence proof';
    END IF;
    EXECUTE replace(body,original,$new$r.quest_key=NEW.plan->>'quest'
        AND (r.cause_kind='command' OR (r.cause_kind='use'
            AND NEW.plan #>> '{world_callback,key}'=NEW.transition_key
            AND NEW.plan #>> '{world_callback,quest}'=NEW.plan->>'quest'
            AND NEW.plan #>> '{world_callback,trigger}'='use'
            AND NEW.plan #> '{world_callback,effects,item_reward}'=NEW.plan->'give'
            AND NEW.plan #> '{world_callback,storage}'=NEW.plan->'storage'
            AND NEW.plan #> '{world_callback,offerings}'='[]'::jsonb
            AND COALESCE(NEW.plan #> '{world_callback,item_on_target}','null'::jsonb)='null'::jsonb))$new$);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
