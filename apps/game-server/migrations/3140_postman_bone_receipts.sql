-- Kevin takes every carried bone of either source definition. Keep the
-- ordinary typed-input cap and the exact aggregate/subtype consumption checks.
DO $migration$
DECLARE
    body TEXT;
    old_cap CONSTANT TEXT := '(e->>''count'')::NUMERIC NOT BETWEEN 1 AND 100';
    postman_cap CONSTANT TEXT := $cap$(e->>'count')::NUMERIC NOT BETWEEN 1 AND
                    CASE WHEN NEW.transition_key='oteryn:quest-npc/postman/kevin/bones_complete'
                         AND NEW.plan->>'quest'='oteryn:quest.the_postman_missions_quest'
                         AND NEW.plan->>'npc'='oteryn:npc.kevin'
                         AND e->>'item' IN ('oteryn:item.tibia.i3115','oteryn:item.tibia.i3116')
                         AND e->>'subtype' IS NULL
                    THEN 50000 -- at most 500 locked instances of at most 100 units
                    ELSE 100 END$cap$;
BEGIN
    body := pg_get_functiondef('game_quest_npc_typed_item_complete()'::regprocedure);
    IF position(old_cap IN body)=0 THEN
        RAISE EXCEPTION 'expected the NPC typed-input count check';
    END IF;
    EXECUTE replace(body, old_cap, postman_cap);
END $migration$;
