CREATE TABLE game_character_service_revisions (
 account_id UUID PRIMARY KEY REFERENCES game_character_account_guards,
 revision BIGINT NOT NULL CHECK(revision>0), revised_at BIGINT NOT NULL CHECK(revised_at>0)
);
CREATE TABLE game_character_service_outbox (
 account_id UUID NOT NULL REFERENCES game_character_service_revisions,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 projection_epoch BIGINT NOT NULL CHECK(projection_epoch>0), projection_revision BIGINT NOT NULL CHECK(projection_revision>0),
 observed_at BIGINT NOT NULL, snapshot JSONB NOT NULL, delivered_revision BIGINT NOT NULL DEFAULT 0,
 committed_at_ms BIGINT NOT NULL, PRIMARY KEY(account_id,character_id)
);
CREATE INDEX game_character_service_outbox_pending ON game_character_service_outbox(committed_at_ms,account_id,character_id) WHERE projection_revision>delivered_revision;
CREATE INDEX game_character_service_outbox_character ON game_character_service_outbox(character_id);
CREATE FUNCTION game_character_service_touch(c UUID,a UUID,removed BOOLEAN DEFAULT false) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE r game_character_roots;d game_character_service_deletions;e BIGINT;v BIGINT;n BIGINT:=game_character_service_now();s JSONB;BEGIN
 IF n IS NULL OR n<=0 THEN RAISE EXCEPTION 'trusted service clock unavailable';END IF;
 SELECT * INTO STRICT r FROM game_character_roots WHERE character_id=c;
 SELECT * INTO d FROM game_character_service_deletions WHERE character_id=c;
 SELECT projection_epoch INTO STRICT e FROM game_character_account_projection_epoch WHERE epoch_scope=1;
 INSERT INTO game_character_service_revisions VALUES(a,1,n) ON CONFLICT(account_id) DO UPDATE SET revision=game_character_service_revisions.revision+1,revised_at=excluded.revised_at
 WHERE game_character_service_revisions.revised_at<=n RETURNING revision INTO v;
 IF v IS NULL THEN RAISE EXCEPTION 'service clock moved backwards';END IF;
 s:=jsonb_build_object('account_id',a::text,'character_id',c::text,'projection_epoch',e::text,'projection_revision',v::text,'source_observed_at',n::text,'character_revision',r.character_revision::text,'world_id',r.world_id::text,'name',r.name,
 'sex',CASE WHEN removed THEN NULL ELSE r.sex END,'lifecycle',CASE WHEN removed THEN 'REMOVED_FROM_ACCOUNT' WHEN r.lifecycle=3 THEN 'DELETION_PENDING' WHEN r.lifecycle=4 THEN 'DELETED' ELSE 'ACTIVE' END,
 'deletion_operation_id',CASE WHEN NOT removed AND r.lifecycle IN(3,4) THEN d.operation_id::text END,
 'deletion_scheduled_at',CASE WHEN NOT removed AND r.lifecycle IN(3,4) THEN d.scheduled_at::text END,
 'deletion_effective_at',CASE WHEN NOT removed AND r.lifecycle IN(3,4) THEN d.effective_at::text END);
 INSERT INTO game_character_service_outbox VALUES(a,c,e,v,n,s,0,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)
 ON CONFLICT(account_id,character_id) DO UPDATE SET projection_epoch=e,projection_revision=v,observed_at=n,snapshot=s,committed_at_ms=CASE WHEN game_character_service_outbox.projection_revision>game_character_service_outbox.delivered_revision THEN least(game_character_service_outbox.committed_at_ms,excluded.committed_at_ms) ELSE excluded.committed_at_ms END;
END $$;
CREATE FUNCTION game_character_service_projection_trigger() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND NEW.account_id<>OLD.account_id THEN PERFORM game_character_service_touch(NEW.character_id,OLD.account_id,true);END IF;
 PERFORM game_character_service_touch(NEW.character_id,NEW.account_id);RETURN NULL;
END $$;
CREATE TRIGGER game_character_service_projection_insert AFTER INSERT ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_service_projection_trigger();
CREATE TRIGGER game_character_service_projection_update AFTER UPDATE ON game_character_roots FOR EACH ROW
 WHEN((NEW.account_id,NEW.name,NEW.sex,NEW.lifecycle,NEW.character_revision) IS DISTINCT FROM (OLD.account_id,OLD.name,OLD.sex,OLD.lifecycle,OLD.character_revision))
 EXECUTE FUNCTION game_character_service_projection_trigger();
CREATE FUNCTION game_character_service_deletion_facts_changed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN
 PERFORM game_character_service_touch(NEW.character_id,NEW.account_id);RETURN NULL;
END $$;
CREATE TRIGGER game_character_service_deletion_facts_changed AFTER UPDATE ON game_character_service_deletions
 FOR EACH ROW WHEN((NEW.scheduled_at,NEW.effective_at) IS DISTINCT FROM (OLD.scheduled_at,OLD.effective_at)) EXECUTE FUNCTION game_character_service_deletion_facts_changed();
-- Shared epoch raises/resyncs queue the independent feed without changing LCFA revisions.
CREATE FUNCTION game_character_service_resync() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE item RECORD;BEGIN
 FOR item IN SELECT character_id,account_id FROM game_character_roots ORDER BY account_id,character_id LOOP
  PERFORM game_character_service_touch(item.character_id,item.account_id);
 END LOOP;
 UPDATE game_character_service_outbox o SET projection_epoch=(SELECT projection_epoch FROM game_character_account_projection_epoch WHERE epoch_scope=1),delivered_revision=0,
 snapshot=jsonb_set(o.snapshot,'{projection_epoch}',to_jsonb((SELECT projection_epoch::text FROM game_character_account_projection_epoch WHERE epoch_scope=1)))
 WHERE o.snapshot->>'lifecycle'='REMOVED_FROM_ACCOUNT';
END $$;
CREATE FUNCTION game_character_service_epoch_changed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN
 PERFORM game_character_service_resync();RETURN NULL;END $$;
CREATE TRIGGER game_character_service_epoch_changed AFTER UPDATE ON game_character_account_projection_epoch FOR EACH ROW EXECUTE FUNCTION game_character_service_epoch_changed();
SELECT game_character_service_resync();
REVOKE ALL ON game_character_service_revisions,game_character_service_outbox FROM PUBLIC;
GRANT SELECT ON game_character_service_revisions TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,UPDATE ON game_character_service_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_character_service_outbox TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_character_service_touch(UUID,UUID,BOOLEAN),game_character_service_resync(),game_character_service_epoch_changed(),game_character_service_projection_trigger(),game_character_service_deletion_facts_changed() FROM PUBLIC;
