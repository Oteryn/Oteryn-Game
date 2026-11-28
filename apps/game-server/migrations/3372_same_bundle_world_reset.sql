-- Same-bundle server saves preserve House custody and the exact content issuance.
-- A new House layout still requires the separate House-custody migration path.
DO $$
DECLARE
 f regprocedure;
 body text;
 guard text;
 same_bundle text := $guard$
    p_map ~ '^sha256:[0-9a-f]{64}$' AND EXISTS (
      SELECT 1 FROM game_world_reset_epochs prior
      WHERE prior.world_id=p_world AND prior.reset_epoch=p_epoch
        AND (prior.map_revision,prior.target_digest,prior.server_digest,prior.client_digest,prior.frame_digest)
            =(p_map,p_target,p_server,p_client,p_frame)
    )
 $guard$;
BEGIN
 FOREACH f IN ARRAY ARRAY[
   'game_world_reset_publish(uuid,numeric,bytea,text,bytea,bytea,bytea,jsonb,numeric)'::regprocedure,
   'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)'::regprocedure
 ] LOOP
   body := pg_get_functiondef(f);
   guard := $guard$EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=p_world)
  OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=p_world)$guard$;
   IF position(guard IN body)=0 THEN
      RAISE EXCEPTION '3372 expected the installed World reset House guard in %', f;
   END IF;
   EXECUTE replace(body, guard, '((' || guard || ') AND NOT (' || same_bundle || '))');
 END LOOP;
END $$;

-- The content issuance keeps its native artifact digest. The Hare scheduler
-- and effect owner instead bind the immutable map bundle.
CREATE FUNCTION game_world_reset_raid_bundle_digest(p_map text,p_legacy bytea)
RETURNS bytea LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog AS $$
 SELECT CASE WHEN p_map ~ '^sha256:[0-9a-f]{64}$' THEN decode(substr(p_map,8),'hex') ELSE p_legacy END
$$;
REVOKE ALL ON FUNCTION game_world_reset_raid_bundle_digest(text,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_raid_bundle_digest(text,bytea) TO oteryn_game_runtime;

-- Runtime may lock the exact published job and inspect its phase. The binding
-- remains independently checked in the drain, retirement and commit owners.
CREATE FUNCTION game_world_reset_job_phase_locked(p_world uuid,p_epoch numeric,p_binding bytea,p_target bytea)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 RETURN (
   SELECT phase FROM game_world_reset_jobs
   WHERE (world_id,reset_epoch,retirement_binding,target_digest)=(p_world,p_epoch,p_binding,p_target)
   FOR UPDATE
 );
END $$;

-- Runtime can commit only an already published same-bundle save. The existing
-- activation owner still checks the frozen cohort, current holder and Ground.
CREATE FUNCTION game_world_reset_activate_same_bundle(p_world uuid,p_epoch numeric,p_binding bytea,
 p_target bytea,p_map text,p_server bytea,p_client bytea,p_frame bytea,p_node uuid,p_registration numeric,p_secret bytea)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_world_reset_lock_admission();
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE THEN
   RETURN NULL;
 END IF;
 IF p_map !~ '^sha256:[0-9a-f]{64}$' OR NOT EXISTS (
   SELECT 1 FROM game_world_reset_epochs e
   WHERE (e.world_id,e.reset_epoch,e.map_revision,e.target_digest,e.server_digest,e.client_digest,e.frame_digest)
        =(p_world,p_epoch,p_map,p_target,p_server,p_client,p_frame)
 ) THEN
   RAISE EXCEPTION 'same-bundle World reset predecessor mismatch' USING ERRCODE='23514';
 END IF;
 RETURN game_world_reset_activate(p_world,p_epoch,p_binding,p_target,p_map,p_server,p_client,p_frame,p_node,p_registration);
END $$;

DO $$
DECLARE f regprocedure;
BEGIN
 FOREACH f IN ARRAY ARRAY[
   'game_world_reset_job_phase_locked(uuid,numeric,bytea,bytea)'::regprocedure,
   'game_world_reset_activate_same_bundle(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric,bytea)'::regprocedure
 ] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp', f,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
   EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;

-- A runtime caller cannot commit a pointer advance without the World event
-- decisions used to build the next bundle view.
CREATE FUNCTION game_world_reset_same_bundle_events_proven()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.reset_epoch > OLD.reset_epoch AND EXISTS (
   SELECT 1 FROM game_world_reset_epochs previous
   JOIN game_world_reset_epochs next USING(world_id)
   WHERE previous.world_id=NEW.world_id
     AND previous.reset_epoch=OLD.reset_epoch AND next.reset_epoch=NEW.reset_epoch
     AND next.map_revision ~ '^sha256:[0-9a-f]{64}$'
     AND (previous.map_revision,previous.target_digest,previous.server_digest,previous.client_digest,previous.frame_digest)
         =(next.map_revision,next.target_digest,next.server_digest,next.client_digest,next.frame_digest)
 ) AND NOT EXISTS (
   SELECT 1 FROM game_world_change_days day
   JOIN game_world_change_occurrences occurrence USING(world_id,reset_epoch)
   WHERE day.world_id=NEW.world_id AND day.reset_epoch=NEW.reset_epoch
 ) THEN
   RAISE EXCEPTION 'same-bundle reset must commit its World event decisions' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_same_bundle_events_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_world_reset_same_bundle_events_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_world_reset_same_bundle_events_proven
 AFTER UPDATE ON game_world_reset_active DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_same_bundle_events_proven();
