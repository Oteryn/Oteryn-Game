-- PUBPROJ-D: World/key/UTC-day aggregates and immutable physical occurrence dedup.
-- Source rules are owner-accepted Platform statistics revision2, 2026-11-07.
ALTER TABLE game_public_v2_heads DROP CONSTRAINT game_public_v2_heads_family_check;
ALTER TABLE game_public_v2_heads ADD CONSTRAINT game_public_v2_heads_family_check
 CHECK(family IN('character_deaths','guild','house','character_online_sessions','world_daily_kills'));
CREATE TABLE game_statistics_kill_worlds (
 world_id UUID PRIMARY KEY,
 coverage_from_ms BIGINT NOT NULL CHECK(coverage_from_ms>=0)
);
-- Existing Worlds have no invented pre-installation history. A genuinely new
-- World's first assignment below proves there were no earlier Game actors.
INSERT INTO game_statistics_kill_worlds SELECT DISTINCT world_id,
 floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_runtime_scope_assignments;
CREATE TABLE game_statistics_kill_sources (
 scope_key BYTEA NOT NULL, generation NUMERIC(20,0) NOT NULL CHECK(generation>0),
 world_id UUID NOT NULL, started_ms BIGINT NOT NULL CHECK(started_ms>=0),
 cut_ms BIGINT NOT NULL CHECK(cut_ms>=started_ms), ended_ms BIGINT,
 gap_ms BIGINT, clean BOOLEAN NOT NULL DEFAULT false,
 PRIMARY KEY(scope_key,generation),
 CHECK(ended_ms IS NULL OR ended_ms>=started_ms),
 CHECK(gap_ms IS NULL OR gap_ms>=started_ms)
);
CREATE INDEX game_statistics_kill_source_world ON game_statistics_kill_sources(world_id,started_ms);
CREATE TABLE game_statistics_creature_deaths (
 death_key BYTEA PRIMARY KEY CHECK(octet_length(death_key) IN(52,53)),
 world_id UUID NOT NULL, day_start BIGINT NOT NULL CHECK(day_start>0 AND day_start%86400=0),
 occurred_ms BIGINT NOT NULL CHECK(occurred_ms>=day_start*1000 AND occurred_ms<(day_start+86400)*1000),
 creature_key TEXT NOT NULL CHECK(octet_length(creature_key) BETWEEN 1 AND 255 AND creature_key NOT IN('player','unknown')),
 boss BOOLEAN NOT NULL
);
CREATE INDEX game_statistics_death_retention ON game_statistics_creature_deaths(day_start,death_key);
CREATE TABLE game_statistics_creature_daily_kills (
 world_id UUID NOT NULL, day_start BIGINT NOT NULL CHECK(day_start>0 AND day_start%86400=0),
 creature_key TEXT NOT NULL CHECK(octet_length(creature_key) BETWEEN 1 AND 255 AND creature_key NOT IN('player','unknown')),
 boss BOOLEAN NOT NULL, killed NUMERIC(20,0) NOT NULL CHECK(killed BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,day_start,creature_key,boss)
);
-- Minimal retained subject fences survive payload retention and prevent resurrection.
CREATE TABLE game_statistics_kill_days (
 world_id UUID NOT NULL, day_start BIGINT NOT NULL CHECK(day_start>0 AND day_start%86400=0),
 erased BOOLEAN NOT NULL DEFAULT false,
 PRIMARY KEY(world_id,day_start)
);
REVOKE ALL ON game_statistics_kill_worlds,game_statistics_kill_sources,game_statistics_creature_deaths,game_statistics_creature_daily_kills,game_statistics_kill_days FROM PUBLIC;
GRANT SELECT ON game_statistics_kill_worlds,game_statistics_kill_sources,game_statistics_creature_deaths,game_statistics_creature_daily_kills,game_statistics_kill_days TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_statistics_kill_assignment() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.scope_kind IN(1,3) AND OLD.state=1
  AND (NEW.state<>1 OR NEW.ownership_generation<>OLD.ownership_generation) THEN
  UPDATE game_statistics_kill_sources SET ended_ms=coalesce(ended_ms,greatest(started_ms,cut_ms,(NEW.decided_at+1)*1000)),
   gap_ms=CASE WHEN clean THEN gap_ms ELSE least(gap_ms,cut_ms) END
   WHERE scope_key=OLD.scope_key AND generation=OLD.ownership_generation;
 END IF;
 IF NEW.scope_kind IN(1,3) AND NEW.state=1 THEN
  INSERT INTO game_statistics_kill_worlds VALUES(NEW.world_id,(NEW.decided_at/86400)*86400000) ON CONFLICT DO NOTHING;
  INSERT INTO game_statistics_kill_sources(scope_key,generation,world_id,started_ms,cut_ms)
  VALUES(NEW.scope_key,NEW.ownership_generation,NEW.world_id,NEW.decided_at*1000,NEW.decided_at*1000)
  ON CONFLICT DO NOTHING;
 END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_statistics_kill_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'physical occurrence facts are immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER statistics_kill_occurrence_immutable BEFORE UPDATE ON game_statistics_creature_deaths
 FOR EACH ROW EXECUTE FUNCTION game_statistics_kill_immutable();
CREATE TRIGGER statistics_kill_sources_no_truncate BEFORE TRUNCATE ON game_statistics_kill_sources EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER statistics_kill_dimensions_no_truncate BEFORE TRUNCATE ON game_statistics_creature_daily_kills EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER statistics_kill_occurrences_no_truncate BEFORE TRUNCATE ON game_statistics_creature_deaths EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER statistics_kill_days_no_truncate BEFORE TRUNCATE ON game_statistics_kill_days EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER statistics_kill_worlds_no_truncate BEFORE TRUNCATE ON game_statistics_kill_worlds EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER statistics_kill_assignment AFTER INSERT OR UPDATE ON game_runtime_scope_assignments
 FOR EACH ROW EXECUTE FUNCTION game_statistics_kill_assignment();
-- A node ending is authoritative even before a replacement assignment arrives.
-- Legacy end timestamps are whole seconds; keep a conservative exclusive upper
-- bound for the dirty window, never invent a precise closure or zero interval.
CREATE FUNCTION game_statistics_kill_node_ending() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 UPDATE game_statistics_kill_sources s SET gap_ms=least(s.gap_ms,s.cut_ms),
  ended_ms=coalesce(s.ended_ms,greatest(s.started_ms,s.cut_ms,(NEW.ended_at+1)*1000))
 FROM game_runtime_scope_assignments a WHERE a.scope_key=s.scope_key AND a.ownership_generation=s.generation
  AND a.holder_node_id=NEW.node_id AND NOT s.clean;
 RETURN NULL;
END $$;
CREATE TRIGGER statistics_kill_node_ending AFTER INSERT ON game_node_registration_endings
 FOR EACH ROW EXECUTE FUNCTION game_statistics_kill_node_ending();
-- No historical zero/complete claims: capture starts at installation, not assignment time.
INSERT INTO game_statistics_kill_sources(scope_key,generation,world_id,started_ms,cut_ms)
 SELECT scope_key,ownership_generation,world_id,
 floor(extract(epoch FROM clock_timestamp())*1000)::bigint,
 floor(extract(epoch FROM clock_timestamp())*1000)::bigint
 FROM game_runtime_scope_assignments WHERE state=1 AND scope_kind IN(1,3);

CREATE FUNCTION game_statistics_kill_day_complete(w UUID,d BIGINT) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT (d+86400)*1000<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
 AND EXISTS(SELECT 1 FROM game_statistics_kill_worlds WHERE world_id=w AND coverage_from_ms<=d*1000)
 AND NOT EXISTS(SELECT 1 FROM game_statistics_kill_sources s WHERE s.world_id=w
  AND s.started_ms<(d+86400)*1000 AND coalesce(s.ended_ms,9223372036854775807)>d*1000
  AND (s.gap_ms IS NOT NULL AND s.gap_ms<(d+86400)*1000
    OR s.cut_ms<least(coalesce(s.ended_ms,9223372036854775807),(d+86400)*1000)))
$$;
CREATE FUNCTION game_statistics_kill_day_touch(w UUID,d BIGINT,force BOOLEAN DEFAULT false) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p JSONB; m NUMERIC; b NUMERIC;
BEGIN
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM game_public_v2_subject_lock('kills',w::text||':'||d::text);
 INSERT INTO game_statistics_kill_days(world_id,day_start) VALUES(w,d) ON CONFLICT DO NOTHING;
 IF (SELECT erased FROM game_statistics_kill_days WHERE world_id=w AND day_start=d)
 OR NOT EXISTS(SELECT 1 FROM game_statistics_kill_worlds WHERE world_id=w AND coverage_from_ms<=d*1000) THEN
  p:='[]'::jsonb;
 ELSE
  SELECT coalesce(sum(killed) FILTER(WHERE NOT boss),0),coalesce(sum(killed) FILTER(WHERE boss),0) INTO m,b
   FROM game_statistics_creature_daily_kills WHERE world_id=w AND day_start=d;
  IF m>18446744073709551615 OR b>18446744073709551615 THEN p:='null'::jsonb;
  ELSE p:=jsonb_build_array(jsonb_build_object('day_end',(d+86400)::text,'monsters_killed',m::text,
    'bosses_killed',b::text,'complete',game_statistics_kill_day_complete(w,d))); END IF;
 END IF;
 IF force OR NOT EXISTS(SELECT 1 FROM game_public_v2_heads WHERE family='world_daily_kills' AND world_id=w::text AND subject=d::text AND payload=p) THEN
  IF force THEN PERFORM game_public_v2_force_touch('world_daily_kills',w::text,d::text,p);
  ELSE PERFORM game_public_v2_touch('world_daily_kills',w::text,d::text,p); END IF;
 END IF;
END $$;

CREATE FUNCTION game_statistics_kill_batch(k BYTEA,g NUMERIC,n UUID,nr NUMERIC,secret BYTEA,
 events JSONB,cut BIGINT,source_gap BIGINT,final_cut BOOLEAN) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE s game_statistics_kill_sources%ROWTYPE; a RECORD; e JSONB; prior game_statistics_creature_deaths%ROWTYPE;
 death BYTEA; source_key BYTEA; source_gen NUMERIC; offset_gen INT; i INT; d BIGINT; t BIGINT;
 key_text TEXT; is_boss BOOLEAN; w UUID; now_ms BIGINT:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
BEGIN
 IF jsonb_typeof(events)<>'array' OR jsonb_array_length(events)>256 OR cut<0 OR cut>now_ms+1000 THEN
  RAISE EXCEPTION 'invalid physical kill batch' USING ERRCODE='23514'; END IF;
 IF NOT game_node_prove_current_incarnation(n,nr,secret) OR NOT game_runtime_scope_assignment_history_valid() THEN
  RAISE EXCEPTION 'physical kill batch node authority' USING ERRCODE='23514'; END IF;
 SELECT x.world_id INTO STRICT w FROM game_runtime_scope_assignments x
  JOIN game_durability_admission_runtime_guards r ON r.scope_key=set_byte(k,0,CASE get_byte(k,0) WHEN 3 THEN 2 ELSE 1 END)
  WHERE x.scope_key=k AND x.state=1 AND x.ownership_generation=g AND x.holder_node_id=n
   AND x.holder_registration_revision=nr AND (r.ready OR final_cut) AND r.ownership_generation=g FOR SHARE OF x,r;
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 FOR a IN SELECT DISTINCT ((value->>'occurred_ms')::bigint/86400000)*86400 AS day FROM jsonb_array_elements(events)
  UNION SELECT (cut/86400000)*86400 UNION SELECT (cut/86400000)*86400-86400 ORDER BY day LOOP
  PERFORM game_public_v2_subject_lock('kills',w::text||':'||a.day::text);
 END LOOP;
 SELECT * INTO STRICT s FROM game_statistics_kill_sources WHERE scope_key=k AND generation=g FOR UPDATE;
 -- A lost final COMMIT acknowledgement may retry at a later polling clock.
 -- Current possession/assignment/generation were independently proved above;
 -- clean sources can acknowledge only immutable facts already retained, without
 -- changing the original coverage endpoint, aggregate, head or outbox.
 IF s.clean THEN
  IF NOT final_cut OR cut<s.cut_ms OR source_gap IS DISTINCT FROM s.gap_ms THEN
   RAISE EXCEPTION 'closed physical kill source retry conflict' USING ERRCODE='23514'; END IF;
  FOR e IN SELECT value FROM jsonb_array_elements(events) LOOP
   IF (SELECT count(*) FROM jsonb_object_keys(e))<>4 OR NOT(e?'death_key' AND e?'creature_key' AND e?'boss' AND e?'occurred_ms')
    OR jsonb_typeof(e->'boss')<>'boolean' THEN RAISE EXCEPTION 'physical kill exact fields' USING ERRCODE='23514'; END IF;
   death:=decode(e->>'death_key','hex');key_text:=e->>'creature_key';is_boss:=(e->>'boss')::boolean;t:=(e->>'occurred_ms')::bigint;
   SELECT * INTO prior FROM game_statistics_creature_deaths WHERE death_key=death;
   IF NOT FOUND THEN
    RAISE EXCEPTION 'closed physical kill source new occurrence' USING ERRCODE='23514'; END IF;
   IF prior.world_id IS DISTINCT FROM w OR prior.creature_key IS DISTINCT FROM key_text
    OR prior.boss IS DISTINCT FROM is_boss OR prior.occurred_ms IS DISTINCT FROM t THEN
    RAISE EXCEPTION 'closed physical kill source fact conflict' USING ERRCODE='23514'; END IF;
  END LOOP;
  RETURN;
 END IF;
 IF cut<s.cut_ms OR cut<s.started_ms THEN
  RAISE EXCEPTION 'nonmonotonic physical kill source cut' USING ERRCODE='23514'; END IF;
 FOR e IN SELECT value FROM jsonb_array_elements(events) LOOP
  IF (SELECT count(*) FROM jsonb_object_keys(e))<>4 OR NOT(e?'death_key' AND e?'creature_key' AND e?'boss' AND e?'occurred_ms')
    OR jsonb_typeof(e->'boss')<>'boolean' THEN RAISE EXCEPTION 'physical kill exact fields' USING ERRCODE='23514'; END IF;
  death:=decode(e->>'death_key','hex');key_text:=e->>'creature_key';is_boss:=(e->>'boss')::boolean;t:=(e->>'occurred_ms')::bigint;
  d:=(t/86400000)*86400;
  IF octet_length(death) NOT IN(52,53) OR key_text IN('player','unknown') OR octet_length(key_text) NOT BETWEEN 1 AND 255
   OR key_text !~ '^[A-Za-z0-9:._/-]+$' OR t<0 OR t>now_ms+1000 OR d<=0 THEN
   RAISE EXCEPTION 'invalid native death dimensions' USING ERRCODE='23514'; END IF;
  SELECT * INTO prior FROM game_statistics_creature_deaths WHERE death_key=death;
  IF FOUND THEN
   IF prior.creature_key<>key_text OR prior.boss<>is_boss OR prior.occurred_ms<>t THEN
    RAISE EXCEPTION 'physical death identity conflict' USING ERRCODE='23514'; END IF;
   CONTINUE; -- An exact retained historical replay adds no authority or count.
  END IF;
  IF octet_length(death)=52 THEN source_key:='\x01'::bytea||substring(death FROM 1 FOR 32);offset_gen:=32;
  ELSIF get_byte(death,0)=2 THEN source_key:='\x03'::bytea||substring(death FROM 2 FOR 32);offset_gen:=33;
  ELSE RAISE EXCEPTION 'physical occurrence namespace' USING ERRCODE='23514'; END IF;
  source_gen:=0;FOR i IN 0..7 LOOP source_gen:=source_gen*256+get_byte(death,offset_gen+i);END LOOP;
  IF source_key<>k OR source_gen<>g OR t<s.started_ms OR (d+86400+34560000)*1000<=now_ms
   OR EXISTS(SELECT 1 FROM game_statistics_kill_days WHERE world_id=w AND day_start=d AND erased) THEN
   RAISE EXCEPTION 'physical source or retention fence' USING ERRCODE='23514'; END IF;
  INSERT INTO game_statistics_creature_deaths VALUES(death,w,d,t,key_text,is_boss);
  INSERT INTO game_statistics_creature_daily_kills VALUES(w,d,key_text,is_boss,1)
   ON CONFLICT(world_id,day_start,creature_key,boss) DO UPDATE SET killed=game_statistics_creature_daily_kills.killed+1;
 END LOOP;
 UPDATE game_statistics_kill_sources SET cut_ms=cut,gap_ms=least(gap_ms,source_gap),
  clean=final_cut,ended_ms=CASE WHEN final_cut THEN cut ELSE ended_ms END WHERE scope_key=k AND generation=g;
 -- At most one current day and one boundary day per bounded source pass; actual
 -- late corrections are also rebuilt from the batch's finite distinct days.
 FOR a IN SELECT DISTINCT ((value->>'occurred_ms')::bigint/86400000)*86400 AS day FROM jsonb_array_elements(events)
  UNION SELECT (cut/86400000)*86400
  UNION SELECT (cut/86400000)*86400-86400 LOOP
  IF a.day>0 AND (a.day+86400+34560000)*1000>now_ms THEN PERFORM game_statistics_kill_day_touch(w,a.day); END IF;
 END LOOP;
END $$;

CREATE FUNCTION game_statistics_kill_maintenance() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE h RECORD; cutoff BIGINT:=floor(extract(epoch FROM clock_timestamp()))::bigint-34560000;
BEGIN
 FOR h IN SELECT world_id,day_start FROM game_statistics_kill_days WHERE NOT erased AND day_start+86400<=cutoff
  ORDER BY world_id,day_start LIMIT 256 LOOP
  PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
  PERFORM game_public_v2_subject_lock('kills',h.world_id::text||':'||h.day_start::text);
  UPDATE game_statistics_kill_days SET erased=true WHERE world_id=h.world_id AND day_start=h.day_start;
  DELETE FROM game_statistics_creature_daily_kills WHERE world_id=h.world_id AND day_start=h.day_start;
  PERFORM game_statistics_kill_day_touch(h.world_id,h.day_start);
 END LOOP;
 DELETE FROM game_statistics_creature_deaths WHERE death_key IN
  (SELECT death_key FROM game_statistics_creature_deaths WHERE day_start+86400<=cutoff ORDER BY day_start,death_key LIMIT 1024);
END $$;

ALTER FUNCTION game_public_v2_source_pending(TEXT) RENAME TO game_public_v2_source_pending_before_kills;
CREATE FUNCTION game_public_v2_source_pending(f TEXT) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f='world_daily_kills' THEN least((SELECT min(gap_ms) FROM game_statistics_kill_sources
  WHERE gap_ms IS NOT NULL AND coalesce(ended_ms,9223372036854775807)>floor(extract(epoch FROM statement_timestamp())*1000)::bigint-34560000000),
  (SELECT min(cut_ms+1) FROM game_statistics_kill_sources WHERE ended_ms IS NULL AND NOT clean))
 ELSE game_public_v2_source_pending_before_kills(f) END
$$;
ALTER FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT) RENAME TO game_public_v2_ready_before_kills;
CREATE FUNCTION game_public_v2_ready(f TEXT,w TEXT,s TEXT) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f='world_daily_kills' THEN true ELSE game_public_v2_ready_before_kills(f,w,s) END
$$;
ALTER FUNCTION game_public_projection_resync(TEXT,BOOLEAN) RENAME TO game_public_projection_resync_before_kills;
CREATE FUNCTION game_public_projection_resync(f TEXT,raise_epoch BOOLEAN) RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE e BIGINT; h RECORD;
BEGIN
 IF f IS NULL OR raise_epoch IS NULL THEN
  RAISE EXCEPTION 'resync requires family and epoch mode' USING ERRCODE='22004';
 END IF;
 IF f='world_daily_kills' THEN
  IF raise_epoch THEN RAISE EXCEPTION 'only all may raise epoch' USING ERRCODE='22023'; END IF;
  SELECT projection_epoch INTO STRICT e FROM game_public_projection_epoch WHERE epoch_scope=1 FOR UPDATE;
 ELSE e:=game_public_projection_resync_before_kills(f,raise_epoch); END IF;
 IF f IN('all','world_daily_kills') THEN
  PERFORM game_statistics_kill_maintenance();
  FOR h IN SELECT world_id,day_start FROM game_statistics_kill_days ORDER BY world_id,day_start LOOP
   PERFORM game_statistics_kill_day_touch(h.world_id,h.day_start,true);
  END LOOP;
 END IF;
 RETURN e;
END $$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=current_schema()
 AND (p.proname LIKE 'game_statistics_kill_%' OR p.proname IN('game_public_v2_source_pending','game_public_v2_ready','game_public_projection_resync')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_statistics_kill_batch(BYTEA,NUMERIC,UUID,NUMERIC,BYTEA,JSONB,BIGINT,BIGINT,BOOLEAN),
 game_statistics_kill_maintenance(),game_public_v2_source_pending(TEXT),game_public_v2_ready(TEXT,TEXT,TEXT) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_public_v2_source_pending(TEXT),game_public_v2_ready(TEXT,TEXT,TEXT) TO oteryn_game_control;
