-- C10: native fresh admission uses 2=Active, 1=Reconnectable. Legacy
-- reconnect journals use 1=Active. House handoffs retain the fresh lineage.
-- Extend the existing sleep guard without replacing its custody/ACL checks.
DO $migration$
DECLARE
    body TEXT;
    before TEXT := 'AND s.session_state=1 AND s.current_transport_ref=p_transport';
    after TEXT := $guard$AND s.session_state=CASE
              WHEN COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) IS NOT NULL
              THEN 2 ELSE 1 END
          AND s.current_transport_ref=p_transport$guard$;
BEGIN
    body := pg_get_functiondef('game_house_bed_sleep(uuid,uuid,uuid,text,numeric,numeric,numeric,bytea,numeric,integer,integer,integer,text,integer,uuid,numeric)'::regprocedure);
    IF length(body) - length(replace(body, before, '')) <> length(before) THEN
        RAISE EXCEPTION 'expected exactly one bed sleep active-session guard';
    END IF;
    -- Retain the function identity, SECURITY DEFINER and pinned search_path.
    EXECUTE replace(body, before, after);
END $migration$;

REVOKE ALL ON FUNCTION game_house_bed_sleep(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,TEXT,INTEGER,UUID,NUMERIC)
    FROM PUBLIC, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_sleep(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,TEXT,INTEGER,UUID,NUMERIC)
    TO oteryn_game_runtime;
