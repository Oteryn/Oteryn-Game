-- OPERATOR PRECONDITION: apply only with Game writers and publishers STOPPED.
-- This transactional migration retains source-trigger SHARE ROW EXCLUSIVE and
-- CHECK-replacement ACCESS EXCLUSIVE locks through validation and index build.
-- Qualify elapsed time on a representative restored store before deployment;
-- there is no online-migration or production-size timing claim.
-- P3/G-4 revision 3. Derived sources only; no serving or publisher activation.
-- A validated whole Game catalogue, never an inferred revision from one earned row.
CREATE TABLE game_public_v2_achievement_catalogue (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), revision TEXT NOT NULL,
 entries JSONB NOT NULL CHECK(jsonb_typeof(entries)='object')
);
-- Publication evidence is written by the appearance owner AFTER content/unlock validation
-- in the same fenced transaction. A raw selection row alone never proves DisplayedLook.
CREATE TABLE game_public_v2_outfit_proofs (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 account_id UUID NOT NULL, sex SMALLINT NOT NULL CHECK(sex IN(1,2)),
 occurrence_id UUID NOT NULL REFERENCES game_character_appearance_receipts(appearance_occurrence_id),
 content_revision TEXT NOT NULL, outfit_key TEXT NOT NULL,
 colours INTEGER[] NOT NULL CHECK(game_appearance_colours_valid(colours)),
 addons SMALLINT NOT NULL CHECK(addons BETWEEN 0 AND 255)
);
CREATE INDEX game_public_v2_outfit_proof_occurrence ON game_public_v2_outfit_proofs(occurrence_id);
CREATE TABLE game_public_v2_character_baselines (
 family TEXT PRIMARY KEY CHECK(family IN('character_skills','character_achievements','character_equipment','character_outfit')),
 complete BOOLEAN NOT NULL DEFAULT false
);
INSERT INTO game_public_v2_character_baselines(family) VALUES('character_skills'),('character_achievements'),('character_equipment'),('character_outfit');
REVOKE ALL ON game_public_v2_achievement_catalogue,game_public_v2_outfit_proofs,game_public_v2_character_baselines FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_public_v2_achievement_catalogue TO oteryn_game_runtime,oteryn_game_control;
REVOKE INSERT,UPDATE,DELETE ON game_public_v2_outfit_proofs FROM oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_public_v2_outfit_proofs TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_public_v2_character_baselines TO oteryn_game_runtime,oteryn_game_control;

-- Account-leading lookup for achievement fan-out (built only while stopped).
CREATE INDEX game_public_v2_character_account ON game_character_roots USING hash(account_id) WHERE lifecycle=1;
CREATE TABLE game_public_v2_achievement_cuts (
 account_id UUID PRIMARY KEY, last_xid xid8 NOT NULL
);
CREATE TABLE game_public_v2_catalogue_refresh (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), last_character UUID
);
REVOKE ALL ON game_public_v2_achievement_cuts,game_public_v2_catalogue_refresh FROM PUBLIC;

CREATE FUNCTION game_public_v2_write_outfit_proof(c UUID,o UUID,k TEXT,p INTEGER[],a SMALLINT)
 RETURNS BIGINT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE changed BIGINT;
BEGIN
 INSERT INTO game_public_v2_outfit_proofs(character_id,account_id,sex,occurrence_id,content_revision,outfit_key,colours,addons)
 SELECT r.character_id,r.account_id,r.sex,v.last_appearance_occurrence_id,v.content_revision,k,p,a
 FROM game_character_roots r JOIN game_character_appearance v USING(character_id)
 JOIN game_character_appearance_receipts q ON q.appearance_occurrence_id=v.last_appearance_occurrence_id AND q.character_id=r.character_id
 WHERE r.character_id=c AND v.last_appearance_occurrence_id=o AND r.sex IS NOT NULL
 AND v.outfit_key=k AND v.colours=p AND v.addons=a AND v.committed_character_revision=r.character_revision
 ON CONFLICT(character_id) DO UPDATE SET account_id=EXCLUDED.account_id,sex=EXCLUDED.sex,occurrence_id=EXCLUDED.occurrence_id,
 content_revision=EXCLUDED.content_revision,outfit_key=EXCLUDED.outfit_key,colours=EXCLUDED.colours,addons=EXCLUDED.addons;
 GET DIAGNOSTICS changed=ROW_COUNT;
 RETURN changed;
END $$;
CREATE FUNCTION game_public_v2_equipment_custody(c UUID)
 RETURNS TABLE(slot SMALLINT,item_instance_id UUID,world_id UUID) LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT s.slot,s.item_instance_id,s.world_id FROM game_character_equipment_slots s WHERE s.character_id=c
 UNION ALL SELECT 9::smallint,b.item_instance_id,b.world_id FROM game_item_container_slots b WHERE b.character_id=c
$$;
CREATE FUNCTION game_public_v2_character_capture(c UUID,f TEXT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_character_roots%ROWTYPE; p JSONB:='[]'::jsonb; cat RECORD;
BEGIN
 IF f NOT IN('character_skills','character_achievements','character_equipment','character_outfit') THEN
  RAISE EXCEPTION 'unknown character projection family' USING ERRCODE='22023';
 END IF;
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 -- All four capture families take the catalogue lock before any subject lock.
 -- A skills capture and a later achievement capture in one transaction cannot
 -- straddle a catalogue UPDATE holding its row and waiting for that subject.
 PERFORM 1 FROM game_public_v2_achievement_catalogue WHERE singleton FOR SHARE;
 PERFORM game_public_v2_subject_lock('character_views',c::text);
 SELECT * INTO r FROM game_character_roots WHERE character_id=c;
 IF r.character_id IS NOT NULL AND r.lifecycle=1 AND NOT game_account_closure_detached(c) THEN
  IF f='character_skills' THEN
   SELECT jsonb_build_array(jsonb_build_object('world_id',r.world_id::text,'character_revision',r.character_revision::text,
    'level',s.level::text,'total_experience',s.total_experience::text,'magic_level',coalesce(b.magic_level,0),
    'fist_level',coalesce(b.fist_level,10),'club_level',coalesce(b.club_level,10),'sword_level',coalesce(b.sword_level,10),
    'axe_level',coalesce(b.axe_level,10),'distance_level',coalesce(b.distance_level,10),
    'shielding_level',coalesce(b.shielding_level,10),'fishing_level',coalesce(b.fishing_level,10))) INTO p
   FROM game_character_progression_state s LEFT JOIN game_character_build_state b USING(character_id)
   WHERE s.character_id=c AND s.character_revision=r.character_revision
    AND (s.profile_revision,s.ruleset_revision,s.content_revision)=(r.profile_revision,r.ruleset_revision,r.content_revision)
    AND (b.character_id IS NULL OR b.committed_character_revision<=r.character_revision);
   p:=coalesce(p,'[]'::jsonb);
  ELSIF f='character_achievements' THEN
   SELECT * INTO cat FROM game_public_v2_achievement_catalogue WHERE singleton;
   p:='null'::jsonb; -- Missing catalogue is unknown, never a known empty earned list.
   IF FOUND THEN
    -- One statement/cut for count, compatibility and content, bounded at 1025.
    WITH earned AS MATERIALIZED (
     SELECT a.achievement_key,g.earned_at FROM game_account_achievements a
      LEFT JOIN game_account_achievement_grant_requests g USING(account_id,achievement_key,source_kind,source_event_id)
      WHERE a.account_id=r.account_id ORDER BY a.achievement_key COLLATE "C" LIMIT 1025
    ) SELECT CASE WHEN count(*)>1024 OR coalesce(bool_or(earned_at IS NULL OR NOT cat.entries ? achievement_key OR earned_at/1000<=0),false)
     THEN 'null'::jsonb ELSE jsonb_build_array(jsonb_build_object('world_id',r.world_id::text,
      'character_revision',r.character_revision::text,'catalogue_revision',cat.revision,
      'earned',coalesce(jsonb_agg(jsonb_build_object('key',achievement_key,'earned_at',(earned_at/1000)::text)
       ORDER BY achievement_key COLLATE "C"),'[]'::jsonb))) END INTO p FROM earned;
   END IF;
  ELSIF f='character_equipment' THEN
   IF EXISTS(SELECT 1 FROM game_character_equipment_state WHERE character_id=c)
    AND NOT EXISTS(SELECT 1 FROM game_public_v2_equipment_custody(c) s LEFT JOIN game_item_instances i USING(item_instance_id)
      WHERE (i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.definition_family<>'Item'
       OR i.world_id<>r.world_id OR s.world_id<>r.world_id
       OR i.definition_production_key !~ '^[a-z0-9._:-]{1,128}$'
       OR i.definition_revision_ref !~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$')) THEN
    SELECT jsonb_build_array(jsonb_build_object('world_id',r.world_id::text,'character_revision',r.character_revision::text,
     'slots',coalesce(jsonb_agg(jsonb_build_object('slot',
      CASE s.slot WHEN 1 THEN 'Head' WHEN 7 THEN 'Necklace' WHEN 9 THEN 'Container' WHEN 2 THEN 'Armor' WHEN 5 THEN 'RightHand' WHEN 6 THEN 'LeftHand' WHEN 3 THEN 'Legs' WHEN 4 THEN 'Feet' WHEN 8 THEN 'Ring' WHEN 10 THEN 'Ammo' END,
      'item_key',i.definition_production_key,'item_revision',i.definition_revision_ref) ORDER BY array_position(ARRAY[1,7,9,2,5,6,3,4,8,10],s.slot::integer))
       FILTER(WHERE s.slot IS NOT NULL),'[]'::jsonb))) INTO p
     FROM game_public_v2_equipment_custody(c) s JOIN game_item_instances i USING(item_instance_id);
   END IF;
  ELSE
   SELECT jsonb_build_array(jsonb_build_object('world_id',r.world_id::text,'character_revision',r.character_revision::text,
    'content_revision',v.content_revision,'outfit_key',v.outfit_key,'colours',to_jsonb(v.colours),'addons',v.addons,'mount','[]'::jsonb)) INTO p
   FROM game_public_v2_outfit_proofs v JOIN game_character_appearance a USING(character_id)
   WHERE v.character_id=c AND v.account_id=r.account_id AND v.sex=r.sex AND v.content_revision=r.content_revision
    AND (v.occurrence_id,v.content_revision,v.outfit_key,v.colours,v.addons)=
        (a.last_appearance_occurrence_id,a.content_revision,a.outfit_key,a.colours,a.addons)
    AND a.committed_character_revision<=r.character_revision
    AND v.outfit_key ~ '^[a-z0-9._:-]{1,128}$';
   p:=coalesce(p,'[]'::jsonb);
  END IF;
 END IF;
 -- Existing force_touch increments the stored counter; wall clock never deduplicates changes.
 PERFORM game_public_v2_force_touch(f,'',c::text,p);
END $$;

CREATE FUNCTION game_public_v2_character_source() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID; f TEXT; a UUID;
BEGIN
 c:=CASE WHEN TG_OP='DELETE' THEN OLD.character_id ELSE NEW.character_id END;
 IF TG_TABLE_NAME='game_character_roots' AND TG_OP='UPDATE' THEN
  IF (OLD.account_id,OLD.world_id,OLD.lifecycle,OLD.profile_revision,OLD.ruleset_revision,OLD.content_revision,OLD.sex)
   IS NOT DISTINCT FROM (NEW.account_id,NEW.world_id,NEW.lifecycle,NEW.profile_revision,NEW.ruleset_revision,NEW.content_revision,NEW.sex) THEN RETURN NULL; END IF;
 END IF;
 IF TG_TABLE_NAME='game_character_roots' THEN
  SELECT account_id INTO a FROM game_character_roots WHERE character_id=c;
  IF a IS NOT NULL THEN PERFORM game_public_v2_subject_lock('character_views','account:'||a::text); END IF;
 END IF;
 FOREACH f IN ARRAY TG_ARGV LOOP
  -- Revision-only gameplay writes do not re-scan unrelated source families.
  IF TG_TABLE_NAME='game_character_roots' AND TG_OP='UPDATE' THEN
   IF (OLD.account_id,OLD.world_id,OLD.lifecycle) IS NOT DISTINCT FROM
      (NEW.account_id,NEW.world_id,NEW.lifecycle) THEN
    IF f='character_achievements' OR f='character_equipment' THEN CONTINUE; END IF;
    IF f='character_skills' AND (OLD.profile_revision,OLD.ruleset_revision,OLD.content_revision)
       IS NOT DISTINCT FROM (NEW.profile_revision,NEW.ruleset_revision,NEW.content_revision) THEN CONTINUE; END IF;
    IF f='character_outfit' AND (OLD.sex,OLD.content_revision) IS NOT DISTINCT FROM (NEW.sex,NEW.content_revision) THEN CONTINUE; END IF;
   END IF;
  END IF;
  PERFORM game_public_v2_character_capture(c,f);
 END LOOP;
 RETURN NULL;
END $$;
-- Deferred capture sees the final reconciled source cut, never an intermediate root/XP update.
CREATE CONSTRAINT TRIGGER public_v2_skills_progression AFTER INSERT OR UPDATE OR DELETE ON game_character_progression_state
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_skills');
CREATE CONSTRAINT TRIGGER public_v2_skills_build AFTER INSERT OR UPDATE OR DELETE ON game_character_build_state
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_skills');
CREATE CONSTRAINT TRIGGER public_v2_character_root AFTER INSERT OR UPDATE OR DELETE ON game_character_roots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_skills','character_achievements','character_equipment','character_outfit');
CREATE CONSTRAINT TRIGGER public_v2_equipment_slots AFTER INSERT OR UPDATE OR DELETE ON game_character_equipment_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_equipment');
CREATE CONSTRAINT TRIGGER public_v2_equipment_state AFTER INSERT OR UPDATE OR DELETE ON game_character_equipment_state
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_equipment');
CREATE CONSTRAINT TRIGGER public_v2_appearance AFTER INSERT OR UPDATE OR DELETE ON game_character_appearance
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_outfit');
CREATE CONSTRAINT TRIGGER public_v2_appearance_proof AFTER INSERT OR UPDATE OR DELETE ON game_public_v2_outfit_proofs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_outfit');

CREATE FUNCTION game_public_v2_achievement_source() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c RECORD; a UUID;
BEGIN
 a:=CASE WHEN TG_OP='DELETE' THEN OLD.account_id ELSE NEW.account_id END;
 -- Deferred events all observe the final transaction cut. Fan out once per
 -- affected account, even when the gameplay commit grants many achievements.
 INSERT INTO game_public_v2_achievement_cuts(account_id,last_xid) VALUES(a,pg_current_xact_id())
 ON CONFLICT(account_id) DO UPDATE SET last_xid=EXCLUDED.last_xid
 WHERE game_public_v2_achievement_cuts.last_xid<>EXCLUDED.last_xid;
 IF NOT FOUND THEN RETURN NULL; END IF;
 PERFORM game_public_v2_subject_lock('character_views','account:'||a::text);
 FOR c IN SELECT character_id FROM game_character_roots
 WHERE account_id=a AND lifecycle=1
 ORDER BY character_id LOOP
  PERFORM game_public_v2_character_capture(c.character_id,'character_achievements');
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER public_v2_earned AFTER INSERT OR UPDATE OR DELETE ON game_account_achievements
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_achievement_source();
CREATE FUNCTION game_public_v2_catalogue_source() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 -- Refresh only invalidates coverage here. Runtime preparation recaptures a
 -- bounded page per ordinary durability pass; never scan all actors in a trigger.
 UPDATE game_public_v2_character_baselines SET complete=false WHERE family='character_achievements';
 INSERT INTO game_public_v2_catalogue_refresh(singleton,last_character) VALUES(true,NULL)
 ON CONFLICT(singleton) DO UPDATE SET last_character=NULL;
 RETURN NULL;
END $$;
CREATE TRIGGER public_v2_catalogue AFTER INSERT OR UPDATE ON game_public_v2_achievement_catalogue
 FOR EACH ROW EXECUTE FUNCTION game_public_v2_catalogue_source();

CREATE FUNCTION game_public_v2_catalogue_refresh_page() RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE page_size CONSTANT INTEGER:=16; after_character UUID; c RECORD; last_seen UUID; captured INTEGER:=0;
BEGIN
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM 1 FROM game_public_v2_achievement_catalogue WHERE singleton FOR SHARE;
 SELECT last_character INTO after_character FROM game_public_v2_catalogue_refresh WHERE singleton FOR UPDATE;
 IF NOT FOUND THEN RETURN true; END IF;
 FOR c IN SELECT DISTINCT subject FROM (
  (SELECT character_id::text AS subject FROM (SELECT character_id FROM game_character_roots
    WHERE character_id>coalesce(after_character,'00000000-0000-0000-0000-000000000000'::uuid) ORDER BY character_id LIMIT page_size) r)
  UNION ALL
  (SELECT subject FROM game_public_v2_heads WHERE family='character_achievements' AND world_id=''
    AND subject>coalesce(after_character::text,'') ORDER BY subject LIMIT page_size)
 ) subjects ORDER BY subject LIMIT page_size LOOP
  PERFORM game_public_v2_character_capture(c.subject::uuid,'character_achievements');
  last_seen:=c.subject::uuid; captured:=captured+1;
 END LOOP;
 IF captured<page_size THEN
  DELETE FROM game_public_v2_catalogue_refresh WHERE singleton;
  UPDATE game_public_v2_character_baselines SET complete=true WHERE family='character_achievements';
  RETURN true;
 END IF;
 UPDATE game_public_v2_catalogue_refresh SET last_character=last_seen WHERE singleton;
 RETURN false;
END $$;

CREATE CONSTRAINT TRIGGER public_v2_container AFTER INSERT OR UPDATE OR DELETE ON game_item_container_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_equipment');
CREATE FUNCTION game_public_v2_item_source() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c RECORD; item_id UUID;
BEGIN
 item_id:=CASE WHEN TG_OP='DELETE' THEN OLD.item_instance_id ELSE NEW.item_instance_id END;
 FOR c IN SELECT character_id FROM game_character_equipment_slots WHERE item_instance_id=item_id
  UNION SELECT character_id FROM game_item_container_slots WHERE item_instance_id=item_id LOOP
  PERFORM game_public_v2_character_capture(c.character_id,'character_equipment');
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER public_v2_item AFTER INSERT OR UPDATE OR DELETE ON game_item_instances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_item_source();
CREATE CONSTRAINT TRIGGER public_v2_closure_member AFTER INSERT OR UPDATE OR DELETE ON game_account_closure_members
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_public_v2_character_source('character_skills','character_achievements','character_equipment','character_outfit');

ALTER FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT) RENAME TO game_public_v2_ready_before_character_views;
CREATE FUNCTION game_public_v2_ready(f TEXT,w TEXT,s TEXT) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f IN('character_skills','character_achievements','character_equipment','character_outfit') THEN
  EXISTS(SELECT 1 FROM game_public_v2_character_baselines WHERE family=f AND complete)
  AND EXISTS(SELECT 1 FROM game_public_v2_heads WHERE family=f AND world_id=w AND subject=s AND payload<>'null'::jsonb)
 ELSE game_public_v2_ready_before_character_views(f,w,s) END
$$;
ALTER FUNCTION game_public_v2_source_pending(TEXT) RENAME TO game_public_v2_source_pending_before_character_views;
CREATE FUNCTION game_public_v2_source_pending(f TEXT) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN f IN('character_skills','character_achievements','character_equipment','character_outfit')
  AND EXISTS(SELECT 1 FROM game_public_v2_character_baselines WHERE family=f AND NOT complete) THEN 0::bigint
 ELSE game_public_v2_source_pending_before_character_views(f) END
$$;
ALTER FUNCTION game_public_projection_resync(TEXT,BOOLEAN) RENAME TO game_public_projection_resync_before_character_views;
CREATE FUNCTION game_public_projection_resync(f TEXT,raise_epoch BOOLEAN) RETURNS BIGINT LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE e BIGINT; c RECORD; family_name TEXT; last_character UUID; batch_count INTEGER;
BEGIN
 IF f IS NULL OR raise_epoch IS NULL THEN RAISE EXCEPTION 'resync requires family and epoch mode' USING ERRCODE='22004'; END IF;
 IF f IN('character_skills','character_achievements','character_equipment','character_outfit') THEN
  IF raise_epoch THEN RAISE EXCEPTION 'only all may raise epoch' USING ERRCODE='22023'; END IF;
  SELECT projection_epoch INTO STRICT e FROM game_public_projection_epoch WHERE epoch_scope=1 FOR UPDATE;
 ELSE e:=game_public_projection_resync_before_character_views(f,raise_epoch); END IF;
 IF f='all' OR f IN('character_skills','character_achievements','character_equipment','character_outfit') THEN
 LOOP
  batch_count:=0;
  FOR c IN SELECT character_id FROM (
    SELECT character_id FROM game_character_roots WHERE last_character IS NULL OR character_id>last_character
    UNION SELECT subject::uuid FROM game_public_v2_heads WHERE family IN('character_skills','character_achievements','character_equipment','character_outfit')
      AND (last_character IS NULL OR subject::uuid>last_character)
   ) subjects ORDER BY character_id LIMIT 128 LOOP
   FOREACH family_name IN ARRAY ARRAY['character_skills','character_achievements','character_equipment','character_outfit'] LOOP
    IF f='all' OR f=family_name THEN PERFORM game_public_v2_character_capture(c.character_id,family_name); END IF;
   END LOOP;
   last_character:=c.character_id; batch_count:=batch_count+1;
  END LOOP;
  EXIT WHEN batch_count<128;
 END LOOP;
 END IF;
 UPDATE game_public_v2_character_baselines SET complete=true WHERE f='all' OR family=f;
 RETURN e;
END $$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=current_schema()
 AND (p.proname IN('game_public_v2_write_outfit_proof','game_public_v2_equipment_custody','game_public_v2_item_source','game_public_v2_character_capture','game_public_v2_character_source','game_public_v2_achievement_source','game_public_v2_catalogue_source','game_public_v2_catalogue_refresh_page','game_public_v2_ready','game_public_v2_source_pending','game_public_projection_resync')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT),game_public_v2_source_pending(TEXT) TO oteryn_game_runtime,oteryn_game_control;

-- Expand the pre-existing closed checks last; validations retain the DROP locks.
-- The stopped-service prerequisite above covers the full migration transaction.
-- PostgreSQL requires ACCESS EXCLUSIVE to remove the old closed CHECK; keep that
-- unavoidable lock out of source setup/resync (the migrator is transactional).
ALTER TABLE game_public_v2_heads DROP CONSTRAINT game_public_v2_heads_family_check;
ALTER TABLE game_public_v2_heads ADD CONSTRAINT game_public_v2_heads_family_check CHECK(family IN(
 'character_deaths','guild','house','character_online_sessions','world_daily_kills',
 'character_skills','character_achievements','character_equipment','character_outfit')) NOT VALID;
ALTER TABLE game_public_v2_heads VALIDATE CONSTRAINT game_public_v2_heads_family_check;

ALTER TABLE game_public_v2_subject_fences DROP CONSTRAINT game_public_v2_subject_fences_lock_kind_check;
ALTER TABLE game_public_v2_subject_fences ADD CONSTRAINT game_public_v2_subject_fences_lock_kind_check
 CHECK(lock_kind IN('deaths','guild','house','online','kills','character_views')) NOT VALID;
ALTER TABLE game_public_v2_subject_fences VALIDATE CONSTRAINT game_public_v2_subject_fences_lock_kind_check;


GRANT EXECUTE ON FUNCTION game_public_v2_write_outfit_proof(UUID,UUID,TEXT,INTEGER[],SMALLINT) TO oteryn_game_runtime,oteryn_game_control;

GRANT EXECUTE ON FUNCTION game_public_v2_catalogue_refresh_page() TO oteryn_game_runtime,oteryn_game_control;
