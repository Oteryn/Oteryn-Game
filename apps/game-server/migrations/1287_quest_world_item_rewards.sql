-- A world reward must carry its exact same-transaction USE receipt and callback plan.
DO $migration$ DECLARE body TEXT; original TEXT:= $old$AND r.quest_key=NEW.plan->>'quest' AND r.cause_kind='command'$old$;
BEGIN
 SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
 IF strpos(body,original)=0 THEN RAISE EXCEPTION 'world reward requires original exact quest receipt proof'; END IF;
 EXECUTE replace(body,original,$new$AND r.quest_key=NEW.plan->>'quest'
                AND (r.cause_kind='command' OR (r.cause_kind='use'
                    AND NEW.plan #>> '{world_callback,key}'=NEW.transition_key
                    AND NEW.plan #>> '{world_callback,quest}'=NEW.plan->>'quest'
                    AND NEW.plan #>> '{world_callback,trigger}'='use'))$new$);
END $migration$;
