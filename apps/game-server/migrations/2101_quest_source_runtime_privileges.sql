-- 1257/1258 omitted the restricted node login's privileges. The online timer
-- pass reads these even when no source leases exist. Recipes remain read-only;
-- leases, observations and their audits remain append-only under their guards.
REVOKE ALL ON game_shared_switch_recipes, game_shared_switch_leases,
    game_shared_switch_audit, game_quest_timer_rules,
    game_quest_timer_observations, game_quest_timer_audit FROM PUBLIC;

GRANT SELECT ON game_shared_switch_recipes, game_shared_switch_leases,
    game_shared_switch_audit, game_quest_timer_rules,
    game_quest_timer_observations, game_quest_timer_audit
    TO oteryn_game_runtime, oteryn_game_control;
GRANT INSERT ON game_shared_switch_leases, game_shared_switch_audit,
    game_quest_timer_observations, game_quest_timer_audit TO oteryn_game_runtime;

-- Match the preceding source migrations: pin all function lookups and remove
-- default PUBLIC execution, including trigger-only entry points. Invoker
-- triggers need their called helpers, but no direct trigger-function grant.
DO $$ DECLARE f REGPROCEDURE; BEGIN
    FOR f IN SELECT p.oid::regprocedure FROM pg_proc p
        JOIN pg_namespace n ON n.oid=p.pronamespace
        WHERE n.nspname=current_schema()
          AND (p.proname LIKE 'game_shared_switch_%'
               OR p.proname LIKE 'game_quest_timer_%') LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',
                       f,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
    END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION
    game_shared_switch_lock(UUID,TEXT,NUMERIC),
    game_shared_switch_count(UUID,TEXT,NUMERIC,TEXT,BIGINT),
    game_shared_switch_complete(UUID),
    game_quest_timer_source_current(UUID,TEXT,UUID,UUID,UUID,UUID,NUMERIC),
    game_quest_timer_envelope(UUID,TEXT),
    game_quest_timer_complete(UUID,TEXT)
    TO oteryn_game_runtime;

-- No UPDATE is needed on the 1250-1258 sources: their FOR UPDATE clauses
-- lock game_item_instances, whose existing runtime grant covers row locks.
-- 1600 only extends existing quest guards; it introduces no new objects.
