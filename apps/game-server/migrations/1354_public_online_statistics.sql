-- PUBPROJ-D / G-8, owner 2026-11-07: authoritative ONLINE intervals,
-- not scan observations. No admission/session identity appears on their wire.
-- Donor reference: Canary99902524 src/io/functions/iologindata_save_player.cpp:320,359-360;
-- Crystalff7ede59 same path:200-206,364-378. Owner2026-11-07 replaces cumulative
-- counters/negative-time clamping with private atomic intervals and unknown-end denial.
ALTER TABLE game_public_v2_heads DROP CONSTRAINT game_public_v2_heads_family_check;
ALTER TABLE game_public_v2_heads ADD CONSTRAINT game_public_v2_heads_family_check
 CHECK(family IN('character_deaths','guild','house','character_online_sessions'));
CREATE INDEX game_public_online_head_clock ON game_public_v2_heads(world_id,observed_ms DESC)
 WHERE family='character_online_sessions';

CREATE TABLE game_character_online_intervals (
 statistics_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7() CHECK(game_character_is_uuid_v7(statistics_id)),
 character_id UUID NOT NULL CHECK(game_character_is_uuid_v7(character_id)),
 -- Private ownership binding detects transfers; erased with the payload.
 owner_account_id UUID,
 world_id UUID,
 started_ms BIGINT,
 ended_ms BIGINT,
 expires_ms BIGINT,
 state SMALLINT NOT NULL CHECK(state IN(1,2,3)), -- open, closed, payload erased
 withheld BOOLEAN NOT NULL DEFAULT false,
 CHECK((state=3 AND owner_account_id IS NULL AND world_id IS NULL AND started_ms IS NULL
           AND ended_ms IS NULL AND expires_ms IS NULL)
    OR (state IN(1,2) AND owner_account_id IS NOT NULL AND world_id IS NOT NULL
         AND started_ms>0 AND expires_ms>started_ms
         AND ((state=1 AND ended_ms IS NULL) OR (state=2 AND ended_ms>=started_ms))))
);
-- No FK to the Character root: deletion must retain the exported subject fence.
CREATE UNIQUE INDEX game_online_one_open_interval ON game_character_online_intervals(character_id) WHERE state=1;
CREATE INDEX game_online_character_intervals ON game_character_online_intervals(character_id,statistics_id);
CREATE INDEX game_online_interval_expiry ON game_character_online_intervals(expires_ms,statistics_id) WHERE state<>3;
CREATE TABLE game_character_online_pending (
 character_id UUID PRIMARY KEY,
 observed_ms BIGINT NOT NULL,
 reason TEXT NOT NULL CHECK(reason IN('INITIAL_START_UNKNOWN','UNSAFE_CLOCK','OPEN_RETENTION')),
 world_id UUID,
 owner_account_id UUID
);
REVOKE ALL ON game_character_online_intervals,game_character_online_pending FROM PUBLIC;
GRANT SELECT ON game_character_online_intervals,game_character_online_pending TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_public_online_publish(p_statistics UUID,p_force BOOLEAN DEFAULT false) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_character_online_intervals%ROWTYPE; payload JSONB;
BEGIN
 SELECT * INTO STRICT i FROM game_character_online_intervals WHERE statistics_id=p_statistics;
 IF i.state=3 OR i.withheld THEN payload:='[]'::jsonb;
 ELSIF EXISTS(SELECT 1 FROM game_character_online_pending WHERE character_id=i.character_id AND reason='UNSAFE_CLOCK') THEN
  payload:='null'::jsonb;
 ELSE
  payload:=jsonb_build_array(jsonb_build_object('world_id',i.world_id::text,
    'started_at',(i.started_ms/1000)::text,
    'ended_at',CASE WHEN i.ended_ms IS NULL THEN '[]'::jsonb ELSE jsonb_build_array((i.ended_ms/1000)::text) END));
 END IF;
 IF p_force THEN
  PERFORM game_public_v2_force_touch('character_online_sessions',i.character_id::text,i.statistics_id::text,payload);
 ELSE
  PERFORM game_public_v2_touch('character_online_sessions',i.character_id::text,i.statistics_id::text,payload);
 END IF;
END $$;

CREATE FUNCTION game_public_online_refresh_character(c UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID; a UUID; i game_character_online_intervals%ROWTYPE;
 p game_character_online_pending%ROWTYPE; n BIGINT; last_observed BIGINT;
 s UUID; row_id UUID; transferred BOOLEAN; history_transferred BOOLEAN;
BEGIN
 -- Every source writer already owns its existing authority locks. This lock
 -- serializes derived interval ownership without granting gameplay authority.
 PERFORM game_public_v2_subject_lock('online',c::text);
 SELECT account_id INTO a FROM game_character_roots WHERE character_id=c;
 SELECT r.world_id INTO w FROM game_character_roots r
 JOIN game_durability_admission_character_guards g ON g.character_id=r.character_id
 JOIN game_durability_reconnect_sessions live ON live.game_session_id=g.holder_game_session_id
 WHERE r.character_id=c AND game_public_v2_character_visible(c)
 AND g.eligible AND g.world_id=r.world_id AND live.character_id=r.character_id AND live.world_id=r.world_id
 AND live.session_state=2 AND live.current_transport_ref IS NOT NULL AND live.control_loss_epoch IS NULL;
 SELECT * INTO i FROM game_character_online_intervals WHERE character_id=c AND state=1 FOR UPDATE;
 SELECT * INTO p FROM game_character_online_pending WHERE character_id=c FOR UPDATE;
 -- Read the deciding clock after derived-row locks, never before waiting on a
 -- predecessor. Retained envelope metadata also fences clock rollback after
 -- payload erasure without retaining erased interval start/end fields.
 n:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 SELECT observed_ms INTO last_observed FROM game_public_v2_heads
  WHERE family='character_online_sessions' AND world_id=c::text ORDER BY observed_ms DESC LIMIT 1;
 transferred:=i.statistics_id IS NOT NULL AND a IS NOT NULL AND i.owner_account_id IS DISTINCT FROM a;
 history_transferred:=a IS NOT NULL AND EXISTS(SELECT 1 FROM game_character_online_intervals
  WHERE character_id=c AND state<>3 AND NOT withheld AND owner_account_id IS DISTINCT FROM a);
 IF NOT game_public_v2_character_visible(c) OR history_transferred THEN
  -- Server-side cursor, indexed by CharacterId: every retained exported subject
  -- gets its own tombstone in the same privacy/ownership transaction.
  FOR row_id IN SELECT statistics_id FROM game_character_online_intervals
   WHERE character_id=c AND state<>3 AND NOT withheld
    AND (NOT game_public_v2_character_visible(c) OR owner_account_id IS DISTINCT FROM a)
   ORDER BY statistics_id FOR UPDATE LOOP
   UPDATE game_character_online_intervals SET withheld=true WHERE statistics_id=row_id;
   PERFORM game_public_online_publish(row_id);
  END LOOP;
 END IF;
 -- An unsafe timestamp cannot be corrected by guessing a later end. It stays
 -- unavailable until independently reconciled source evidence exists.
 IF p.reason='UNSAFE_CLOCK' THEN RETURN; END IF;
 IF p.character_id IS NOT NULL THEN
  IF w IS NULL THEN
   DELETE FROM game_character_online_pending WHERE character_id=c;
  ELSIF p.world_id IS DISTINCT FROM w OR p.owner_account_id IS DISTINCT FROM a THEN
   -- A proven World/ownership boundary begins new coverage, never reconstructs
   -- the old unknown interval for either owner.
   DELETE FROM game_character_online_pending WHERE character_id=c;
  ELSE RETURN;
  END IF;
 END IF;
 IF i.statistics_id IS NOT NULL AND (w IS NULL OR i.world_id IS DISTINCT FROM w OR transferred) THEN
  IF n<i.started_ms OR n/1000<=0 OR n>9223372034262775807 THEN
   INSERT INTO game_character_online_pending(character_id,observed_ms,reason,world_id,owner_account_id)
   VALUES(c,least(n,i.started_ms),'UNSAFE_CLOCK',i.world_id,i.owner_account_id)
   ON CONFLICT(character_id) DO NOTHING;
   PERFORM game_public_online_publish(i.statistics_id);
   RETURN;
  END IF;
  UPDATE game_character_online_intervals SET state=2,ended_ms=n,expires_ms=n+2592000000,
   withheld=withheld OR transferred OR NOT game_public_v2_character_visible(c)
   WHERE statistics_id=i.statistics_id;
  PERFORM game_public_online_publish(i.statistics_id);
 END IF;
 IF w IS NOT NULL AND (i.statistics_id IS NULL OR i.world_id IS DISTINCT FROM w OR transferred) THEN
  IF n/1000<=0 OR n>9223372034262775807 OR n<coalesce(last_observed,0) THEN
   INSERT INTO game_character_online_pending(character_id,observed_ms,reason,world_id,owner_account_id)
   VALUES(c,n,'UNSAFE_CLOCK',w,a) ON CONFLICT(character_id) DO NOTHING;
   RETURN;
  END IF;
  INSERT INTO game_character_online_intervals(character_id,owner_account_id,world_id,started_ms,expires_ms,state)
   VALUES(c,a,w,n,n+2592000000,1) RETURNING statistics_id INTO s;
  PERFORM game_public_online_publish(s);
 END IF;
END $$;

CREATE FUNCTION game_public_online_source_trigger() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID;
BEGIN
 IF TG_TABLE_NAME='game_account_closures' THEN
  -- Closure membership is an owner-bounded cohort; the cursor retains no
  -- payload in the Game process and takes CharacterIds in stable order.
  FOR c IN SELECT character_id FROM game_account_closure_members
   WHERE account_id=CASE WHEN TG_OP='DELETE' THEN OLD.account_id ELSE NEW.account_id END
   ORDER BY character_id LOOP PERFORM game_public_online_refresh_character(c); END LOOP;
 ELSE
  IF TG_OP<>'INSERT' THEN PERFORM game_public_online_refresh_character(OLD.character_id); END IF;
  IF TG_OP<>'DELETE' AND (TG_OP='INSERT' OR NEW.character_id IS DISTINCT FROM OLD.character_id) THEN
   PERFORM game_public_online_refresh_character(NEW.character_id);
  END IF;
 END IF;
 RETURN NULL;
END $$;
-- Deferred triggers observe the final predicate at the commit boundary. An
-- intra-transaction Channel handoff cannot fabricate an OFFLINE/ONLINE split.
CREATE CONSTRAINT TRIGGER public_online_sessions_source AFTER INSERT OR UPDATE OR DELETE ON game_durability_reconnect_sessions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_online_source_trigger();
CREATE CONSTRAINT TRIGGER public_online_guards_source AFTER INSERT OR UPDATE OR DELETE ON game_durability_admission_character_guards
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_online_source_trigger();
CREATE CONSTRAINT TRIGGER public_online_roots_source AFTER INSERT OR UPDATE OR DELETE ON game_character_roots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_online_source_trigger();
CREATE CONSTRAINT TRIGGER public_online_closure_members_source AFTER INSERT OR UPDATE OR DELETE ON game_account_closure_members
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_online_source_trigger();
CREATE CONSTRAINT TRIGGER public_online_closures_source AFTER INSERT OR UPDATE OR DELETE ON game_account_closures
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_online_source_trigger();

CREATE FUNCTION game_public_online_maintain(p_limit INTEGER) RETURNS INTEGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_character_online_intervals%ROWTYPE; changed INTEGER:=0;
 n BIGINT:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
BEGIN
 IF p_limit NOT BETWEEN 1 AND 32 THEN RAISE EXCEPTION 'online retention batch outside bound' USING ERRCODE='22023'; END IF;
 FOR i IN SELECT * FROM game_character_online_intervals WHERE state<>3 AND expires_ms<=n
  ORDER BY expires_ms,statistics_id LIMIT p_limit FOR UPDATE SKIP LOCKED LOOP
  IF i.state=1 THEN
   INSERT INTO game_character_online_pending(character_id,observed_ms,reason,world_id,owner_account_id)
   VALUES(i.character_id,i.expires_ms,'OPEN_RETENTION',i.world_id,i.owner_account_id)
   ON CONFLICT(character_id) DO NOTHING;
  END IF;
  UPDATE game_character_online_intervals SET state=3,withheld=true,owner_account_id=NULL,world_id=NULL,
   started_ms=NULL,ended_ms=NULL,expires_ms=NULL WHERE statistics_id=i.statistics_id;
  PERFORM game_public_online_publish(i.statistics_id);
  changed:=changed+1;
 END LOOP;
 RETURN changed;
END $$;

ALTER FUNCTION game_public_v2_source_pending(TEXT) RENAME TO game_public_v2_source_pending_before_online;
CREATE FUNCTION game_public_v2_source_pending(f TEXT) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f='character_online_sessions' THEN least(
  (SELECT min(observed_ms) FROM game_character_online_pending),
  (SELECT min(expires_ms) FROM game_character_online_intervals WHERE state<>3
    AND expires_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint))
 ELSE game_public_v2_source_pending_before_online(f) END
$$;
ALTER FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT) RENAME TO game_public_v2_ready_before_online;
CREATE FUNCTION game_public_v2_ready(f TEXT,w TEXT,s TEXT) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f='character_online_sessions' THEN EXISTS(
  SELECT 1 FROM game_character_online_intervals WHERE character_id::text=w AND statistics_id::text=s)
 ELSE game_public_v2_ready_before_online(f,w,s) END
$$;

ALTER FUNCTION game_public_projection_resync(TEXT,BOOLEAN) RENAME TO game_public_projection_resync_before_online;
CREATE FUNCTION game_public_projection_resync(p_family TEXT,p_raise_epoch BOOLEAN) RETURNS BIGINT
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE e BIGINT; s UUID;
BEGIN
 IF p_family IS NULL OR p_raise_epoch IS NULL THEN
  RAISE EXCEPTION 'resync requires family and epoch mode' USING ERRCODE='22004';
 END IF;
 IF p_family='character_online_sessions' THEN
  IF p_raise_epoch THEN RAISE EXCEPTION 'only all may raise epoch' USING ERRCODE='22023'; END IF;
  SELECT projection_epoch INTO STRICT e FROM game_public_projection_epoch WHERE epoch_scope=1 FOR UPDATE;
 END IF;
 IF p_family IN('all','character_online_sessions') THEN
  -- Full retained-set resync is stopped-node/operator-only and bounded by the
  -- existing 60s resync transaction deadline, not a new page-render query.
  WHILE game_public_online_maintain(32)>0 LOOP END LOOP;
  FOR s IN SELECT statistics_id FROM game_character_online_intervals ORDER BY character_id,statistics_id LOOP
   PERFORM game_public_online_publish(s,true);
  END LOOP;
 END IF;
 IF p_family<>'character_online_sessions' THEN
  e:=game_public_projection_resync_before_online(p_family,p_raise_epoch);
 END IF;
 RETURN e;
END $$;

-- Existing live sessions have no provable historical start. Keep their coverage
-- unavailable until an authoritative OFFLINE or World/ownership boundary.
INSERT INTO game_character_online_pending(character_id,observed_ms,reason,world_id,owner_account_id)
SELECT r.character_id,floor(extract(epoch FROM clock_timestamp())*1000)::bigint,'INITIAL_START_UNKNOWN',r.world_id,r.account_id
 FROM game_character_roots r JOIN game_durability_admission_character_guards g USING(character_id)
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id
 WHERE game_public_v2_character_visible(r.character_id) AND g.eligible AND g.world_id=r.world_id
 AND s.character_id=r.character_id AND s.world_id=r.world_id AND s.session_state=2
 AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL;

CREATE FUNCTION game_public_online_no_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'online statistics source/fences cannot be truncated' USING ERRCODE='23514'; END $$;
CREATE TRIGGER public_online_intervals_no_truncate BEFORE TRUNCATE ON game_character_online_intervals
 EXECUTE FUNCTION game_public_online_no_truncate();
CREATE TRIGGER public_online_pending_no_truncate BEFORE TRUNCATE ON game_character_online_pending
 EXECUTE FUNCTION game_public_online_no_truncate();
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
 WHERE n.nspname=current_schema() AND (p.proname LIKE 'game_public_online_%'
  OR p.proname IN('game_public_v2_source_pending','game_public_v2_ready','game_public_projection_resync')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_public_online_maintain(INTEGER),game_public_v2_source_pending(TEXT),game_public_v2_ready(TEXT,TEXT,TEXT)
 TO oteryn_game_runtime,oteryn_game_control;
