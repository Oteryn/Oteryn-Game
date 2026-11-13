-- Independent services namespace/acknowledgement under the accepted LCFA epoch.
CREATE TABLE game_character_service_revisions (
 account_id UUID PRIMARY KEY REFERENCES game_character_account_guards,
 revision BIGINT NOT NULL CHECK(revision>0), revised_at BIGINT NOT NULL CHECK(revised_at>0)
);
CREATE TABLE game_character_service_outbox (
 account_id UUID NOT NULL REFERENCES game_character_service_revisions,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 projection_epoch BIGINT NOT NULL CHECK(projection_epoch>0), projection_revision BIGINT NOT NULL CHECK(projection_revision>0),
 observed_at BIGINT NOT NULL,snapshot JSONB NOT NULL,delivered_revision BIGINT NOT NULL DEFAULT 0,
 committed_at_ms BIGINT NOT NULL,quarantined BOOLEAN NOT NULL DEFAULT false,
 PRIMARY KEY(account_id,character_id)
);
CREATE INDEX game_character_service_outbox_pending ON game_character_service_outbox(committed_at_ms,account_id,character_id) WHERE projection_revision>delivered_revision;
CREATE INDEX game_character_service_outbox_character ON game_character_service_outbox(character_id);
CREATE INDEX game_character_service_resync_roots ON game_character_roots(account_id,character_id);
CREATE TABLE game_character_service_feed (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton),capture_started BOOLEAN NOT NULL DEFAULT false,
 projection_epoch BIGINT NOT NULL,phase SMALLINT NOT NULL CHECK(phase BETWEEN 1 AND 3),
 after_account UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
 after_character UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
 resync_started_at_ms BIGINT NOT NULL,quarantined_epoch BIGINT NOT NULL DEFAULT 0
);
INSERT INTO game_character_service_feed(singleton,projection_epoch,phase,resync_started_at_ms)
 SELECT true,projection_epoch,1,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint FROM game_character_account_projection_epoch WHERE epoch_scope=1;
CREATE FUNCTION game_character_service_projection_now() RETURNS BIGINT LANGUAGE sql VOLATILE
 SET search_path=pg_catalog,public AS $$ SELECT greatest(1,floor(extract(epoch FROM clock_timestamp()))::bigint) $$;
REVOKE ALL ON FUNCTION game_character_service_projection_now() FROM PUBLIC;
-- Projection capture never gates gameplay on service-decision clock health.
-- Preserve monotonic observation time across backward wall-clock adjustments.
CREATE FUNCTION game_character_service_touch(c UUID,a UUID,removed BOOLEAN DEFAULT false,retained JSONB DEFAULT NULL) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE r game_character_roots;d game_character_service_deletions;e BIGINT;v BIGINT;n BIGINT:=game_character_service_projection_now();s JSONB;BEGIN
 SELECT * INTO STRICT r FROM game_character_roots WHERE character_id=c;
 IF NOT removed AND r.account_id<>a THEN RAISE EXCEPTION 'service projection ownership changed';END IF;
 SELECT * INTO d FROM game_character_service_deletions WHERE character_id=c;
 SELECT projection_epoch INTO STRICT e FROM game_character_account_projection_epoch WHERE epoch_scope=1;
 INSERT INTO game_character_service_revisions VALUES(a,1,n) ON CONFLICT(account_id) DO UPDATE SET revision=game_character_service_revisions.revision+1,revised_at=greatest(game_character_service_revisions.revised_at,excluded.revised_at) RETURNING revision,revised_at INTO v,n;
 s:=jsonb_build_object('account_id',a::text,'character_id',c::text,'projection_epoch',e::text,'projection_revision',v::text,'source_observed_at',n::text,
 'character_revision',coalesce(retained->>'character_revision',r.character_revision::text),
 'world_id',coalesce(retained->>'world_id',r.world_id::text),'name',coalesce(retained->>'name',r.name),
 'sex',CASE WHEN removed THEN NULL ELSE r.sex END,'lifecycle',CASE WHEN removed THEN 'REMOVED_FROM_ACCOUNT' WHEN r.lifecycle=3 THEN 'DELETION_PENDING' WHEN r.lifecycle=4 THEN 'DELETED' WHEN r.lifecycle=1 THEN 'ACTIVE' ELSE 'UNSUPPORTED_LIFECYCLE' END,
 'deletion_operation_id',CASE WHEN NOT removed AND r.lifecycle IN(3,4) THEN d.operation_id::text END,
 'deletion_scheduled_at',CASE WHEN NOT removed AND r.lifecycle IN(3,4) THEN d.scheduled_at::text END,
 'deletion_effective_at',CASE WHEN NOT removed AND r.lifecycle IN(3,4) THEN d.effective_at::text END);
 INSERT INTO game_character_service_outbox(account_id,character_id,projection_epoch,projection_revision,observed_at,snapshot,delivered_revision,committed_at_ms)
 VALUES(a,c,e,v,n,s,0,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)
 ON CONFLICT(account_id,character_id) DO UPDATE SET projection_epoch=e,projection_revision=v,observed_at=n,snapshot=s,
 quarantined=CASE WHEN game_character_service_outbox.projection_epoch<e THEN false ELSE game_character_service_outbox.quarantined END,
 committed_at_ms=CASE WHEN game_character_service_outbox.projection_revision>game_character_service_outbox.delivered_revision THEN least(game_character_service_outbox.committed_at_ms,excluded.committed_at_ms) ELSE excluded.committed_at_ms END;
END $$;
CREATE FUNCTION game_character_service_projection_trigger() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_character_service_changes WHERE character_id=NEW.character_id AND created_xact_id=pg_current_xact_id()) THEN
  IF NOT(SELECT capture_started FROM game_character_service_feed WHERE singleton) THEN UPDATE game_character_service_feed SET capture_started=true WHERE singleton;END IF;
 ELSE
  PERFORM 1 FROM game_character_service_feed WHERE singleton FOR SHARE;
  IF NOT(SELECT capture_started FROM game_character_service_feed WHERE singleton) THEN RETURN NULL;END IF;
 END IF;
 IF TG_OP='UPDATE' AND NEW.account_id<>OLD.account_id THEN
  PERFORM game_character_service_touch(NEW.character_id,OLD.account_id,true,jsonb_build_object('name',OLD.name,'world_id',OLD.world_id::text,'character_revision',NEW.character_revision::text));
 END IF;
 PERFORM game_character_service_touch(NEW.character_id,NEW.account_id);RETURN NULL;
END $$;
CREATE TRIGGER game_character_service_projection_insert AFTER INSERT ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_service_projection_trigger();
CREATE TRIGGER game_character_service_projection_update AFTER UPDATE ON game_character_roots FOR EACH ROW
 WHEN((NEW.account_id,NEW.name,NEW.sex,NEW.lifecycle,NEW.character_revision) IS DISTINCT FROM (OLD.account_id,OLD.name,OLD.sex,OLD.lifecycle,OLD.character_revision)) EXECUTE FUNCTION game_character_service_projection_trigger();
CREATE FUNCTION game_character_service_deletion_facts_changed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN
 PERFORM game_character_service_touch(NEW.character_id,NEW.account_id);RETURN NULL;END $$;
CREATE TRIGGER game_character_service_deletion_facts_changed AFTER UPDATE ON game_character_service_deletions FOR EACH ROW
 WHEN((NEW.scheduled_at,NEW.effective_at) IS DISTINCT FROM (OLD.scheduled_at,OLD.effective_at)) EXECUTE FUNCTION game_character_service_deletion_facts_changed();
CREATE FUNCTION game_character_service_epoch_changed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN
 UPDATE game_character_service_feed SET projection_epoch=NEW.projection_epoch,phase=1,
 after_account='00000000-0000-0000-0000-000000000000',after_character='00000000-0000-0000-0000-000000000000',
 resync_started_at_ms=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint WHERE singleton;
 RETURN NULL;END $$;
CREATE TRIGGER game_character_service_epoch_changed AFTER UPDATE ON game_character_account_projection_epoch FOR EACH ROW EXECUTE FUNCTION game_character_service_epoch_changed();
-- A single authority pass enumerates at most sixteen subjects. Completeness stays
-- below resync start until both roots and retained old-owner removals are queued.
CREATE FUNCTION game_character_service_resync_page() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE f game_character_service_feed;item RECORD;counted INTEGER:=0;last_account UUID;last_character UUID;current_owner UUID;BEGIN
 IF NOT pg_try_advisory_xact_lock(hashtextextended('oteryn:character-service-resync',0)) THEN RETURN;END IF;
 SELECT * INTO STRICT f FROM game_character_service_feed WHERE singleton;
 last_account:=f.after_account;last_character:=f.after_character;
 IF f.quarantined_epoch=f.projection_epoch THEN RETURN;END IF;
 IF NOT f.capture_started THEN RETURN;END IF;
 IF f.phase=3 THEN
  -- An ordinary transaction may have sampled the preceding epoch before an
  -- operator raise committed. Repair those late rows without introducing a
  -- new epoch/portfolio lock order; their pending bytes hold completeness back.
  FOR item IN SELECT account_id,character_id,snapshot FROM game_character_service_outbox WHERE projection_epoch<>f.projection_epoch ORDER BY account_id,character_id LIMIT 16 LOOP
   PERFORM 1 FROM game_character_account_guards WHERE account_id=item.account_id FOR UPDATE;
   SELECT account_id INTO current_owner FROM game_character_roots WHERE character_id=item.character_id FOR SHARE;
   PERFORM game_character_service_touch(item.character_id,item.account_id,current_owner<>item.account_id,CASE WHEN current_owner<>item.account_id THEN item.snapshot END);
  END LOOP;
  RETURN;
 END IF;
 IF f.phase=1 THEN
  FOR item IN SELECT character_id,account_id FROM game_character_roots WHERE (account_id,character_id)>(f.after_account,f.after_character) ORDER BY account_id,character_id LIMIT 16 LOOP
   -- Resync and ordinary mutations use the same portfolio-before-root order.
   PERFORM 1 FROM game_character_account_guards WHERE account_id=item.account_id FOR UPDATE;
   SELECT account_id INTO current_owner FROM game_character_roots WHERE character_id=item.character_id FOR SHARE;
   IF current_owner=item.account_id THEN PERFORM game_character_service_touch(item.character_id,item.account_id);END IF;
   last_account:=item.account_id;last_character:=item.character_id;
   counted:=counted+1;
  END LOOP;
 ELSE
  FOR item IN SELECT o.account_id,o.character_id,o.snapshot,r.account_id AS current_owner FROM game_character_service_outbox o JOIN game_character_roots r USING(character_id)
   WHERE (o.account_id,o.character_id)>(f.after_account,f.after_character)
   ORDER BY o.account_id,o.character_id LIMIT 16 LOOP
   PERFORM 1 FROM game_character_account_guards WHERE account_id=item.account_id FOR UPDATE;
   IF item.account_id<>item.current_owner OR item.snapshot->>'lifecycle'='REMOVED_FROM_ACCOUNT' THEN
   PERFORM game_character_service_touch(item.character_id,item.account_id,true,item.snapshot);
   UPDATE game_character_service_outbox SET quarantined=false,delivered_revision=0 WHERE account_id=item.account_id AND character_id=item.character_id AND projection_epoch=f.projection_epoch;
   END IF;
   last_account:=item.account_id;last_character:=item.character_id;
   counted:=counted+1;
  END LOOP;
 END IF;
 UPDATE game_character_service_feed SET after_account=last_account,after_character=last_character WHERE singleton AND projection_epoch=f.projection_epoch AND phase=f.phase;
 IF counted<16 THEN UPDATE game_character_service_feed SET phase=phase+1,after_account='00000000-0000-0000-0000-000000000000',after_character='00000000-0000-0000-0000-000000000000' WHERE singleton AND projection_epoch=f.projection_epoch AND phase=f.phase;END IF;
END $$;
REVOKE ALL ON game_character_service_revisions,game_character_service_outbox,game_character_service_feed FROM PUBLIC;
GRANT SELECT ON game_character_service_revisions,game_character_service_feed TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,UPDATE ON game_character_service_outbox,game_character_service_feed TO oteryn_game_runtime;
GRANT SELECT ON game_character_service_outbox TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_character_service_touch(UUID,UUID,BOOLEAN,JSONB),game_character_service_resync_page(),game_character_service_epoch_changed(),game_character_service_projection_trigger(),game_character_service_deletion_facts_changed() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_service_resync_page() TO oteryn_game_runtime;
