-- Extend QENGINE2's complete canonical reward guard, retaining every arm.
-- Only a literal world USE with the exact embedded callback may use this source.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    original:=$old$AND r.cause_kind='command'$old$;
    replacement:=$new$AND (r.cause_kind='command' OR (r.cause_kind='use'
        AND NEW.plan #>> '{world_callback,key}'=NEW.transition_key
        AND NEW.plan #>> '{world_callback,quest}'=NEW.plan->>'quest'
        AND NEW.plan #>> '{world_callback,trigger}'='use'))$new$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected canonical quest reward source guard';
    END IF;
    EXECUTE replace(body,original,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_complete() FROM PUBLIC;
