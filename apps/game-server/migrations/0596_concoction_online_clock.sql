-- W22b: Wealth Duplex counts its hour only while the Character is online. Crystal 201189f8
-- config.lua.dist:310-312 (tibiadromeConcoctionTickType = "online") and
-- data/libs/systems/concoctions.lua:126-142 (the tick re-arms only while the player exists),
-- :144-152 (login resumes the remaining time), :179-199 (activation; the 24-hour cooldown stays
-- on the wall clock, os.time at :181).
-- A session is online from its admission (training_created_at_us, 0180, DB clock) until it turns
-- terminal (session_state 3, stamped here) or the Character's next session starts, whichever is
-- first. The effect row of 0582 stays as written; expires_at is now its earliest possible end.
-- Rollback: a successor restores the wall-clock read; the stamp table stays as history.
CREATE TABLE game_character_session_ends (
 game_session_id UUID PRIMARY KEY REFERENCES game_durability_reconnect_sessions(game_session_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 ended_at_us BIGINT NOT NULL CHECK(ended_at_us>=0)
);
CREATE INDEX game_character_session_ends_character ON game_character_session_ends(character_id);
CREATE TRIGGER game_character_session_ends_immutable
 BEFORE UPDATE OR DELETE ON game_character_session_ends
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_session_ends_no_truncate
 BEFORE TRUNCATE ON game_character_session_ends
 EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_character_session_ended() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 -- Only a rooted Character can hold a concoction effect; a session row without a root (the
 -- durability harness admits bare session identities) leaves no stamp instead of failing its
 -- terminal transition.
 INSERT INTO public.game_character_session_ends(game_session_id,character_id,ended_at_us)
 SELECT NEW.game_session_id,NEW.character_id,floor(extract(epoch FROM clock_timestamp())*1000000)::bigint
  WHERE EXISTS (SELECT 1 FROM public.game_character_roots r WHERE r.character_id=NEW.character_id)
 ON CONFLICT (game_session_id) DO NOTHING;
 RETURN NULL;
END $$;
CREATE TRIGGER game_character_session_ended AFTER UPDATE OF session_state
 ON game_durability_reconnect_sessions FOR EACH ROW
 WHEN (OLD.session_state <> 3 AND NEW.session_state = 3)
 EXECUTE FUNCTION game_character_session_ended();

-- Online microseconds of `character` inside [since, now].
CREATE FUNCTION game_character_online_us(p_character UUID, p_since TIMESTAMPTZ, p_now TIMESTAMPTZ)
RETURNS BIGINT
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
 WITH bounds AS (
  SELECT floor(extract(epoch FROM p_since)*1000000)::bigint AS since_us,
         floor(extract(epoch FROM p_now)*1000000)::bigint AS now_us
 ), spans AS (
  SELECT s.training_created_at_us AS start_us,
         LEAST(COALESCE(e.ended_at_us, b.now_us),
               COALESCE(LEAD(s.training_created_at_us) OVER (ORDER BY s.training_session_sequence),
                        b.now_us),
               b.now_us) AS end_us
    FROM public.game_durability_reconnect_sessions s
    CROSS JOIN bounds b
    LEFT JOIN public.game_character_session_ends e USING(game_session_id)
   WHERE s.character_id=p_character AND s.training_created_at_us IS NOT NULL
 )
 SELECT COALESCE(SUM(GREATEST(0, spans.end_us-GREATEST(spans.start_us, b.since_us))), 0)::bigint
   FROM spans CROSS JOIN bounds b
$$;

REVOKE ALL ON game_character_session_ends FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_session_ended() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_online_us(UUID, TIMESTAMPTZ, TIMESTAMPTZ) FROM PUBLIC;
GRANT SELECT ON game_character_session_ends TO oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_character_online_us(UUID, TIMESTAMPTZ, TIMESTAMPTZ)
 TO oteryn_game_runtime, oteryn_game_control;
