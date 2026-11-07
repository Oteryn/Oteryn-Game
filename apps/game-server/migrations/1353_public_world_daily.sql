-- PUBPROJ-C: derived World daily snapshots; source owners keep gameplay authority.
-- 0562_server_save_berlin_06.sql, not the superseded 0148 initial calendar,
-- is current authority. Every interval derives its end from its own calendar row.
ALTER TABLE game_public_v2_heads DROP CONSTRAINT game_public_v2_heads_family_check;
ALTER TABLE game_public_v2_heads ADD CONSTRAINT game_public_v2_heads_family_check
 CHECK(family IN('character_deaths','guild','house','world_daily'));

-- Captured by the existing typed transition writer when the firing is committed.
-- source_kind=3 means a source-calendar callback, not a fixed calendar slot.
ALTER TABLE game_world_raid_firings
 ADD COLUMN public_daily_policy SMALLINT NOT NULL DEFAULT 0 CHECK(public_daily_policy IN(0,1,2)),
 ADD COLUMN public_fixed_slot_ms BIGINT,
 ADD CONSTRAINT game_public_daily_firing_slot CHECK(
 (public_daily_policy=2 AND public_fixed_slot_ms IS NOT NULL AND public_fixed_slot_ms>=0 AND public_fixed_slot_ms<=fired_at_ms)
 OR (public_daily_policy<>2 AND public_fixed_slot_ms IS NULL));
GRANT INSERT(public_daily_policy,public_fixed_slot_ms) ON game_world_raid_firings TO oteryn_game_runtime;
CREATE FUNCTION game_public_daily_firing_identity() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF (NEW.public_daily_policy,NEW.public_fixed_slot_ms) IS DISTINCT FROM (OLD.public_daily_policy,OLD.public_fixed_slot_ms) THEN
  RAISE EXCEPTION 'committed public raid classification is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER public_daily_firing_identity BEFORE UPDATE ON game_world_raid_firings
 FOR EACH ROW EXECUTE FUNCTION game_public_daily_firing_identity();

CREATE TABLE game_public_world_daily_status (
 world_id UUID PRIMARY KEY,
 withdrawn BOOLEAN NOT NULL DEFAULT false,
 pending_ms BIGINT CHECK(pending_ms>=0)
);
REVOKE ALL ON game_public_world_daily_status FROM PUBLIC;
GRANT SELECT ON game_public_world_daily_status TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_public_world_daily_next_slot(start_at TIMESTAMPTZ,calendar_name TEXT,revision NUMERIC)
RETURNS BIGINT LANGUAGE plpgsql STABLE AS $$
DECLARE next_slot TIMESTAMPTZ;
BEGIN
 IF calendar_name IS DISTINCT FROM 'TIBIA_SERVER_SAVE_BERLIN_06' OR revision IS NULL OR revision<1 THEN RETURN NULL; END IF;
 next_slot:=(date_trunc('day',(start_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours')+interval '1 day 6 hours') AT TIME ZONE 'Europe/Berlin';
 IF next_slot<=start_at THEN RETURN NULL; END IF;
 RETURN floor(extract(epoch FROM next_slot))::bigint;
END $$;
CREATE FUNCTION game_public_world_daily_key(k TEXT,bound INTEGER) RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT k IS NOT NULL AND octet_length(k) BETWEEN 1 AND bound AND k COLLATE "C" ~ '^[A-Za-z0-9:._/-]+$'
$$;
CREATE FUNCTION game_public_world_daily_wait(w UUID,at_ms BIGINT) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 INSERT INTO game_public_world_daily_status VALUES(w,false,greatest(0,at_ms)) ON CONFLICT(world_id) DO UPDATE
 SET withdrawn=false,pending_ms=least(coalesce(game_public_world_daily_status.pending_ms,EXCLUDED.pending_ms),EXCLUDED.pending_ms);
 -- JSON null is unavailable source, never the explicit [] withdrawal.
 PERFORM game_public_v2_touch('world_daily','',w::text,'null'::jsonb);
END $$;
CREATE FUNCTION game_public_world_daily_capture(w UUID,explicit_withdrawal BOOLEAN DEFAULT false) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE current_row RECORD; prior RECORD; from_s BIGINT; until_s BIGINT; prior_from BIGINT; prior_until BIGINT;
 now_ms BIGINT;
 floor_s BIGINT; last_start BIGINT; history JSONB:='[]'; raids JSONB; daily JSONB; n INTEGER;
BEGIN
 -- Epoch before the World derivation lock matches operator-resync lock order.
 -- Source rows are changed before this trigger, but read AFTER serialization so
 -- concurrent Channel commits cannot overwrite a newer complete snapshot with
 -- a snapshot built before the other committed change became visible.
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:public-world-daily:'||w::text,0));
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 -- Read only committed source owners in the caller's source transaction. No network.
 SELECT e.*,b.creature_key,b.boss_key INTO current_row
 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 LEFT JOIN game_world_daily_boosts b USING(world_id,reset_epoch) WHERE a.world_id=w;
 IF NOT FOUND THEN
  IF explicit_withdrawal OR EXISTS(SELECT 1 FROM game_public_world_daily_status WHERE world_id=w AND withdrawn) THEN
   INSERT INTO game_public_world_daily_status VALUES(w,true,NULL) ON CONFLICT(world_id) DO UPDATE SET withdrawn=true,pending_ms=NULL;
   PERFORM game_public_v2_touch('world_daily','',w::text,'[]');
  ELSE PERFORM game_public_world_daily_wait(w,now_ms); END IF;
  RETURN;
 END IF;
 from_s:=floor(extract(epoch FROM current_row.activated_at))::bigint;
 until_s:=game_public_world_daily_next_slot(current_row.activated_at,current_row.calendar,current_row.calendar_revision);
 IF from_s<0 OR from_s>now_ms/1000 OR until_s IS NULL OR until_s<=from_s
 OR NOT game_public_world_daily_key(current_row.creature_key,255) OR NOT game_public_world_daily_key(current_row.boss_key,255) THEN
  PERFORM game_public_world_daily_wait(w,least(now_ms,greatest(0,from_s)*1000)); RETURN;
 END IF;
 IF now_ms/1000>=until_s THEN PERFORM game_public_world_daily_wait(w,until_s*1000); RETURN; END IF;
 -- A coverage floor is positive evidence, not an inferred 30-day event ledger.
 -- The latest32 bounded rows suffice: omitted/unknown older intervals are outside
 -- floor_s. Multiple resets never become overlapping published intervals.
 floor_s:=from_s;last_start:=from_s;
 FOR prior IN SELECT e.*,b.creature_key,b.boss_key FROM game_world_reset_epochs e
 LEFT JOIN game_world_daily_boosts b USING(world_id,reset_epoch)
 WHERE e.world_id=w AND e.reset_epoch<current_row.reset_epoch
 AND e.activated_at>=current_row.activated_at-interval '2592000 seconds'
 ORDER BY e.reset_epoch DESC LIMIT 32 LOOP
  prior_from:=floor(extract(epoch FROM prior.activated_at))::bigint;
  prior_until:=game_public_world_daily_next_slot(prior.activated_at,prior.calendar,prior.calendar_revision);
  IF prior_from<greatest(0,from_s-2592000) OR prior_from>=last_start OR prior_until IS NULL
   OR prior_until<=prior_from OR prior_until>last_start
   OR NOT game_public_world_daily_key(prior.creature_key,255) OR NOT game_public_world_daily_key(prior.boss_key,255) THEN EXIT; END IF;
  history:=history||jsonb_build_array(jsonb_build_object('reset_epoch',prior.reset_epoch::text,'effective_from',prior_from::text,
   'effective_until',prior_until::text,'creature_key',prior.creature_key,'boss_key',prior.boss_key));
  floor_s:=prior_from;last_start:=prior_from;
 END LOOP;
 -- Legacy unknown classification is never guessed from names/source_kind,
 -- consumed slots, next_check_at or metadata Applied return values.
 IF EXISTS(SELECT 1 FROM game_world_raid_firings f WHERE f.world_id=w AND f.reset_epoch=current_row.reset_epoch
 AND f.public_daily_policy=0) THEN PERFORM game_public_world_daily_wait(w,from_s*1000); RETURN; END IF;
 IF EXISTS(SELECT 1 FROM game_world_raid_firings f WHERE f.world_id=w AND f.reset_epoch=current_row.reset_epoch AND f.public_daily_policy=2
 AND (f.public_fixed_slot_ms%1000<>0 OR f.public_fixed_slot_ms/1000<from_s OR f.public_fixed_slot_ms/1000>=until_s
 OR f.public_fixed_slot_ms>now_ms OR NOT game_public_world_daily_key(f.raid_key,128))) THEN
  PERFORM game_public_world_daily_wait(w,from_s*1000); RETURN;
 END IF;
 SELECT count(*),coalesce(jsonb_agg(x.body ORDER BY x.slot,x.raid_key),'[]'::jsonb) INTO n,raids FROM (
 SELECT f.public_fixed_slot_ms AS slot,f.raid_key,jsonb_build_object('raid_key',f.raid_key,'scheduled_at',(f.public_fixed_slot_ms/1000)::text,
 'state',CASE f.state WHEN 1 THEN 'running' WHEN 2 THEN 'done' WHEN 3 THEN 'cancelled' END) AS body
 FROM game_world_raid_firings f WHERE f.world_id=w AND f.reset_epoch=current_row.reset_epoch AND f.public_daily_policy=2
 ORDER BY f.public_fixed_slot_ms,f.raid_key LIMIT 257) x;
 IF n>256 THEN PERFORM game_public_world_daily_wait(w,from_s*1000); RETURN; END IF;
 IF EXISTS(SELECT 1 FROM game_world_raid_firings f WHERE f.world_id=w AND f.reset_epoch=current_row.reset_epoch
 AND f.public_daily_policy=2 GROUP BY f.raid_key,f.public_fixed_slot_ms HAVING count(*)>1) THEN
  PERFORM game_public_world_daily_wait(w,from_s*1000); RETURN;
 END IF;
 daily:=jsonb_build_array(jsonb_build_object('reset_epoch',current_row.reset_epoch::text,'effective_from',from_s::text,'effective_until',until_s::text,
 'creature_key',current_row.creature_key,'boss_key',current_row.boss_key,'history_complete_from',floor_s::text,'history',history,'raids',raids));
 PERFORM game_public_v2_touch('world_daily','',w::text,daily);
 INSERT INTO game_public_world_daily_status VALUES(w,false,NULL) ON CONFLICT(world_id) DO UPDATE SET withdrawn=false,pending_ms=NULL;
END $$;
CREATE FUNCTION game_public_world_daily_source_trigger() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN
  PERFORM game_public_world_daily_capture(OLD.world_id,TG_TABLE_NAME='game_world_reset_active' AND TG_OP='DELETE');
 END IF;
 IF TG_OP<>'DELETE' AND (TG_OP='INSERT' OR NEW.world_id IS DISTINCT FROM OLD.world_id) THEN
  PERFORM game_public_world_daily_capture(NEW.world_id,false);
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER public_world_daily_boost AFTER INSERT OR UPDATE OR DELETE ON game_world_daily_boosts FOR EACH ROW EXECUTE FUNCTION game_public_world_daily_source_trigger();
CREATE TRIGGER public_world_daily_reset AFTER INSERT OR UPDATE OR DELETE ON game_world_reset_active FOR EACH ROW EXECUTE FUNCTION game_public_world_daily_source_trigger();
CREATE TRIGGER public_world_daily_calendar AFTER INSERT OR UPDATE ON game_world_reset_epochs FOR EACH ROW EXECUTE FUNCTION game_public_world_daily_source_trigger();
CREATE TRIGGER public_world_daily_raid AFTER INSERT OR UPDATE ON game_world_raid_firings FOR EACH ROW EXECUTE FUNCTION game_public_world_daily_source_trigger();
CREATE INDEX game_public_world_daily_reset_history ON game_world_reset_epochs(world_id,activated_at DESC,reset_epoch DESC);
CREATE INDEX game_public_world_daily_raid_interval ON game_world_raid_firings(world_id,reset_epoch,public_daily_policy,public_fixed_slot_ms,raid_key);

ALTER FUNCTION game_public_v2_source_pending(TEXT) RENAME TO game_public_v2_source_pending_before_world_daily;
CREATE FUNCTION game_public_v2_source_pending(f TEXT) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT min(p.stamp) FROM (
 SELECT game_public_v2_source_pending_before_world_daily(f) AS stamp
 UNION ALL SELECT pending_ms FROM game_public_world_daily_status WHERE f='world_daily'
 UNION ALL SELECT floor(extract(epoch FROM e.activated_at)*1000)::bigint
 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 LEFT JOIN game_public_world_daily_status s USING(world_id) WHERE f='world_daily' AND s.world_id IS NULL
 UNION ALL SELECT (h.payload->0->>'effective_until')::bigint*1000 FROM game_public_v2_heads h
 WHERE f='world_daily' AND h.family=f
 AND CASE WHEN jsonb_typeof(h.payload)='array' THEN jsonb_array_length(h.payload)=1 ELSE false END
 AND (h.payload->0->>'effective_until')::bigint<=floor(extract(epoch FROM statement_timestamp()))::bigint
 ) p
$$;
ALTER FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT) RENAME TO game_public_v2_ready_before_world_daily;
CREATE FUNCTION game_public_v2_ready(f TEXT,w TEXT,s TEXT) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f<>'world_daily' THEN game_public_v2_ready_before_world_daily(f,w,s) ELSE EXISTS(
 SELECT 1 FROM game_public_v2_heads h JOIN game_public_world_daily_status q ON q.world_id=CASE WHEN h.family='world_daily' THEN h.subject::uuid END
 WHERE h.family=f AND h.world_id=w AND h.subject=s AND q.pending_ms IS NULL
 AND CASE WHEN jsonb_typeof(h.payload) IS DISTINCT FROM 'array' THEN false
 WHEN jsonb_array_length(h.payload)=0 THEN q.withdrawn
 ELSE (h.payload->0->>'effective_until')::bigint>floor(extract(epoch FROM statement_timestamp()))::bigint END) END
$$;

ALTER FUNCTION game_public_projection_resync(TEXT,BOOLEAN) RENAME TO game_public_projection_resync_before_world_daily;
CREATE FUNCTION game_public_projection_resync(p_family TEXT,p_raise_epoch BOOLEAN) RETURNS BIGINT LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE e BIGINT; w UUID; head RECORD;
BEGIN
 IF p_family='world_daily' THEN
  IF p_raise_epoch THEN RAISE EXCEPTION 'epoch raise requires all families' USING ERRCODE='23514'; END IF;
  SELECT projection_epoch INTO STRICT e FROM game_public_projection_epoch WHERE epoch_scope=1 FOR UPDATE;
 ELSE e:=game_public_projection_resync_before_world_daily(p_family,p_raise_epoch); END IF;
 IF p_family IN('all','world_daily') THEN
  FOR w IN SELECT world_id FROM game_world_reset_active UNION SELECT world_id FROM game_public_world_daily_status
  UNION SELECT subject::uuid FROM game_public_v2_heads WHERE family='world_daily' ORDER BY 1 LOOP
   PERFORM game_public_world_daily_capture(w,false);
  END LOOP;
  FOR head IN SELECT family,world_id,subject,payload FROM game_public_v2_heads WHERE family='world_daily' ORDER BY subject LOOP
   PERFORM game_public_v2_force_touch(head.family,head.world_id,head.subject,head.payload);
  END LOOP;
 END IF;
 RETURN e;
END $$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=current_schema()
 AND (p.proname LIKE 'game_public_world_daily_%' OR p.proname IN('game_public_daily_firing_identity','game_public_projection_resync','game_public_v2_ready','game_public_v2_source_pending')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT),game_public_v2_source_pending(TEXT)
TO oteryn_game_runtime,oteryn_game_control;
DO $$ DECLARE w UUID; BEGIN
 FOR w IN SELECT world_id FROM game_world_reset_active ORDER BY world_id LOOP PERFORM game_public_world_daily_capture(w,false); END LOOP;
END $$;
