-- PUBPROJ G-4 V2; derived snapshots commit with their source changes.
CREATE TABLE game_public_v2_heads (
 family TEXT NOT NULL CHECK(family IN('character_deaths','guild','house')),
 subject TEXT NOT NULL,
 world_id TEXT NOT NULL DEFAULT '',
 revision BIGINT NOT NULL CHECK(revision>0),
 observed_ms BIGINT NOT NULL CHECK(observed_ms>=0),
 payload JSONB NOT NULL,
 PRIMARY KEY(family,world_id,subject)
);
CREATE TABLE game_public_v2_outbox (
 family TEXT NOT NULL, world_id TEXT NOT NULL, subject TEXT NOT NULL,
 epoch BIGINT NOT NULL CHECK(epoch>0), revision BIGINT NOT NULL CHECK(revision>0),
 created_ms BIGINT NOT NULL CHECK(created_ms>=0),
 PRIMARY KEY(family,world_id,subject,epoch,revision),
 FOREIGN KEY(family,world_id,subject) REFERENCES game_public_v2_heads(family,world_id,subject)
);
CREATE INDEX game_public_v2_outbox_age ON game_public_v2_outbox(family,created_ms,world_id,subject);
-- Exact subject tuple locks preserve source serialization without allocating
-- one shared advisory-lock entry per subject during large operator resyncs.
-- Kinds and canonical subject order match the previous source lock namespaces.
CREATE TABLE game_public_v2_subject_fences (
 lock_kind TEXT NOT NULL CHECK(lock_kind IN('deaths','guild','house','online','kills')),
 subject TEXT NOT NULL,
 PRIMARY KEY(lock_kind,subject)
);
REVOKE ALL ON game_public_v2_subject_fences FROM PUBLIC;
CREATE FUNCTION game_public_v2_subject_lock(p_kind TEXT,p_subject TEXT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 INSERT INTO game_public_v2_subject_fences(lock_kind,subject) VALUES(p_kind,p_subject)
 ON CONFLICT DO NOTHING;
 PERFORM 1 FROM game_public_v2_subject_fences
 WHERE lock_kind=p_kind AND subject=p_subject FOR UPDATE;
END $$;

-- Private immutable historical identity, captured by the fenced death transaction.
CREATE TABLE game_public_death_killers (
 death_occurrence_id UUID NOT NULL REFERENCES game_character_death_receipts(death_occurrence_id),
 ordinal SMALLINT NOT NULL CHECK(ordinal BETWEEN 0 AND 80),
 kind TEXT NOT NULL CHECK(kind IN('player','creature')),
 character_id UUID,
 creature_key TEXT,
 name TEXT NOT NULL,
 PRIMARY KEY(death_occurrence_id,ordinal),
 CHECK((kind='player' AND character_id IS NOT NULL AND creature_key IS NULL)
    OR (kind='creature' AND character_id IS NULL AND creature_key IS NOT NULL))
);
CREATE INDEX game_public_death_killers_player ON game_public_death_killers(character_id) WHERE character_id IS NOT NULL;
CREATE TABLE game_public_house_catalogue (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 name TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN('private_house','shop','guildhall')),
 town TEXT NOT NULL,
 size_sqm BIGINT NOT NULL CHECK(size_sqm>0),
 beds BIGINT NOT NULL CHECK(beds>=0),
 rent_gold BIGINT NOT NULL CHECK(rent_gold>0),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 PRIMARY KEY(world_id,house_key)
);
REVOKE ALL ON game_public_v2_heads,game_public_v2_outbox,game_public_death_killers,game_public_house_catalogue FROM PUBLIC;
GRANT SELECT ON game_public_v2_heads,game_public_death_killers,game_public_house_catalogue TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,DELETE ON game_public_v2_outbox TO oteryn_game_runtime,oteryn_game_control;
GRANT INSERT ON game_public_death_killers TO oteryn_game_runtime;

CREATE FUNCTION game_public_v2_force_touch(f TEXT,w TEXT,s TEXT,p JSONB) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r BIGINT; t BIGINT:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
BEGIN
 -- A shared epoch lock prevents an old-epoch write from escaping resync.
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 INSERT INTO game_public_v2_heads(family,world_id,subject,revision,observed_ms,payload)
 VALUES(f,w,s,1,t,p) ON CONFLICT(family,world_id,subject) DO UPDATE
 SET revision=game_public_v2_heads.revision+1,observed_ms=t,payload=p RETURNING revision INTO r;
 INSERT INTO game_public_v2_outbox SELECT f,w,s,projection_epoch,r,t
 FROM game_public_projection_epoch WHERE epoch_scope=1;
END $$;
-- Source capture can coalesce, but operator resync must always enqueue.
CREATE FUNCTION game_public_v2_touch(f TEXT,w TEXT,s TEXT,p JSONB) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
 PERFORM game_public_v2_force_touch(f,w,s,p);
END $$;
CREATE FUNCTION game_public_v2_character_visible(c UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_character_roots r
 LEFT JOIN game_character_build_state b USING(character_id)
 LEFT JOIN game_character_progression_state p USING(character_id)
 WHERE r.character_id=c AND r.lifecycle=1 AND NOT game_account_closure_detached(c)
 AND coalesce(b.vocation,'none') IN('druid','elder_druid','knight','elite_knight','monk','exalted_monk','paladin','royal_paladin','sorcerer','master_sorcerer','none')
 AND coalesce(p.level,1)>=1)
$$;
CREATE FUNCTION game_public_v2_deaths(c UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p JSONB; recheck_at BIGINT; cut_s BIGINT;
BEGIN
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM game_public_v2_subject_lock('deaths',c::text);
 cut_s:=floor(extract(epoch FROM clock_timestamp()))::bigint;
 -- Missing/unpublishable latest20 historical classification remains unknown
 -- until the actual window expires. Its indexed deadline requests a future
 -- source recapture; it never changes or guesses the immutable receipt facts.
 IF game_public_v2_character_visible(c) THEN
  SELECT min(recent.committed_at/1000+2592001) INTO recheck_at
  FROM (SELECT death_occurrence_id,committed_at FROM game_character_death_receipts
   WHERE character_id=c AND committed_at/1000 >= cut_s-2592000
   ORDER BY committed_at/1000 DESC,death_occurrence_id DESC LIMIT 20) recent
  LEFT JOIN game_public_death_classifications classification USING(death_occurrence_id)
  WHERE classification.death_occurrence_id IS NULL OR NOT classification.publishable;
  IF recheck_at IS NOT NULL THEN
   INSERT INTO game_public_death_rechecks VALUES(c,recheck_at) ON CONFLICT(character_id)
   DO UPDATE SET due_at_seconds=EXCLUDED.due_at_seconds;
   PERFORM game_public_v2_touch('character_deaths','',c::text,'null'::jsonb);
   RETURN;
  END IF;
 END IF;
 -- Hidden subjects suppress safely even while their historical cause is unknown.
 DELETE FROM game_public_death_rechecks WHERE character_id=c;
 SELECT coalesce(jsonb_agg(x.body ORDER BY x.occurred_at DESC,x.death_occurrence_id DESC),'[]'::jsonb) INTO p
 FROM (SELECT d.committed_at/1000 AS occurred_at,d.death_occurrence_id,
 jsonb_build_object('death_occurrence_id',d.death_occurrence_id::text,
 'occurred_at',(d.committed_at/1000)::text,'level',d.level_before::text,
 'pvp',classification.pvp,
 'killers',coalesce((SELECT jsonb_agg(CASE k.kind WHEN 'player' THEN
 jsonb_build_object('kind','player','character_id',k.character_id::text,'name',k.name)
 ELSE jsonb_build_object('kind','creature','creature_key',k.creature_key,'name',k.name) END ORDER BY k.ordinal)
 FROM game_public_death_killers k WHERE k.death_occurrence_id=d.death_occurrence_id
 AND (k.kind='creature' OR game_public_v2_character_visible(k.character_id))),'[]'::jsonb)) AS body
 FROM game_character_death_receipts d JOIN game_public_death_classifications classification USING(death_occurrence_id) WHERE d.character_id=c AND d.committed_at/1000 >= cut_s-2592000 AND game_public_v2_character_visible(c)
 ORDER BY d.committed_at/1000 DESC,d.death_occurrence_id DESC LIMIT 20) x;
 PERFORM game_public_v2_touch('character_deaths','',c::text,p);
END $$;
CREATE FUNCTION game_public_v2_guild(g UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p JSONB;
BEGIN
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM game_public_v2_subject_lock('guild',g::text);
 SELECT jsonb_build_array(jsonb_build_object('world_id',x.world_id::text,'name',x.name,
 'state',CASE x.state WHEN 1 THEN 'FORMING' WHEN 2 THEN 'ACTIVE' WHEN 3 THEN 'DISBANDING' END,
 'founded_at',(x.created_at/1000)::text,
 'ranks',coalesce((SELECT jsonb_agg(jsonb_build_object('level',r.level,'name',r.name) ORDER BY r.level)
 FROM game_guild_ranks r WHERE r.guild_id=g),'[]'::jsonb),
 'members',coalesce((SELECT jsonb_agg(jsonb_build_object('character_id',m.character_id::text,'name',c.name,
 'rank_level',m.rank_level,'joined_at',(m.joined_at/1000)::text) ORDER BY m.rank_level,m.character_id)
 FROM game_guild_members m JOIN game_character_roots c USING(character_id)
 WHERE m.guild_id=g AND game_public_v2_character_visible(m.character_id) AND c.world_id=x.world_id),'[]'::jsonb))) INTO p
 FROM game_guilds x WHERE x.guild_id=g AND x.state IN(1,2,3);
 PERFORM game_public_v2_touch('guild','',g::text,coalesce(p,'[]'::jsonb));
END $$;
CREATE FUNCTION game_public_v2_guild_trigger() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN PERFORM game_public_v2_guild(OLD.guild_id); END IF;
 IF TG_OP<>'DELETE' AND (TG_OP='INSERT' OR NEW.guild_id IS DISTINCT FROM OLD.guild_id) THEN
 PERFORM game_public_v2_guild(NEW.guild_id); END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER public_v2_guild_source AFTER INSERT OR UPDATE OR DELETE ON game_guilds FOR EACH ROW EXECUTE FUNCTION game_public_v2_guild_trigger();
CREATE TRIGGER public_v2_guild_ranks AFTER INSERT OR UPDATE OR DELETE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION game_public_v2_guild_trigger();
CREATE TRIGGER public_v2_guild_members AFTER INSERT OR UPDATE OR DELETE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION game_public_v2_guild_trigger();

-- Private derived heads cannot be reset, truncated or rewritten by publisher roles.
CREATE FUNCTION game_public_v2_head_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.family=OLD.family AND NEW.world_id=OLD.world_id
 AND NEW.subject=OLD.subject AND NEW.revision=OLD.revision+1 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'public V2 heads advance by one' USING ERRCODE='23514';
END $$;
CREATE TRIGGER public_v2_head_guard BEFORE UPDATE OR DELETE ON game_public_v2_heads
 FOR EACH ROW EXECUTE FUNCTION game_public_v2_head_guard();
CREATE TRIGGER public_v2_heads_no_truncate BEFORE TRUNCATE ON game_public_v2_heads
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER public_v2_outbox_no_truncate BEFORE TRUNCATE ON game_public_v2_outbox
 EXECUTE FUNCTION game_character_reject_truncate();
ALTER FUNCTION game_public_projection_resync(TEXT,BOOLEAN) RENAME TO game_public_projection_resync_before_v2;
CREATE FUNCTION game_public_projection_resync(p_family TEXT,p_raise_epoch BOOLEAN) RETURNS BIGINT
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE e BIGINT; h RECORD; latest_payload JSONB;
BEGIN
 IF p_family IS NULL OR p_raise_epoch IS NULL THEN
  RAISE EXCEPTION 'resync requires family and epoch mode' USING ERRCODE='22004';
 END IF;
 IF p_family IN('character_deaths','guild','house') THEN
  IF p_raise_epoch THEN RAISE EXCEPTION 'only all may raise epoch' USING ERRCODE='22023'; END IF;
  SELECT projection_epoch INTO STRICT e FROM game_public_projection_epoch WHERE epoch_scope=1 FOR UPDATE;
 ELSE
  e:=game_public_projection_resync_before_v2(p_family,p_raise_epoch);
 END IF;
 FOR h IN SELECT * FROM game_public_v2_heads WHERE p_family='all' OR family=p_family
 ORDER BY family,world_id,subject LOOP
  IF h.family='character_deaths' THEN
   PERFORM game_public_v2_deaths(h.subject::uuid);
  END IF;
  SELECT payload INTO STRICT latest_payload FROM game_public_v2_heads
   WHERE family=h.family AND world_id=h.world_id AND subject=h.subject;
  PERFORM game_public_v2_force_touch(h.family,h.world_id,h.subject,latest_payload);
 END LOOP;
 RETURN e;
END $$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
 WHERE n.nspname=current_schema() AND p.proname LIKE 'game_public_v2_%' LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_public_projection_resync(text,boolean) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_public_projection_resync(TEXT,BOOLEAN) FROM PUBLIC;

-- Reuse the existing public profile mutation seam so all privacy withdrawals
-- (including closure paths) schedule dependent V2 snapshots in that transaction.
ALTER FUNCTION game_public_profile_touch(UUID,TEXT,UUID) RENAME TO game_public_profile_touch_before_v2;
CREATE FUNCTION game_public_profile_touch(p_character UUID,p_vacated TEXT,p_world UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID;
BEGIN
 PERFORM game_public_profile_touch_before_v2(p_character,p_vacated,p_world);
 -- A withdrawn killer's identity must disappear from every affected victim.
 FOR c IN SELECT p_character AS character_id
 UNION SELECT d.character_id FROM game_public_death_killers k
 JOIN game_character_death_receipts d USING(death_occurrence_id)
 WHERE k.character_id=p_character ORDER BY character_id LOOP
  PERFORM game_public_v2_deaths(c);
 END LOOP;
 PERFORM game_public_v2_character_dependencies(p_character);
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_public_profile_touch(uuid,text,uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_public_profile_touch(UUID,TEXT,UUID) FROM PUBLIC;
