-- A Berlin save occurrence is bound to the epoch that is active at publication.
-- Channel ownership generations fence writes, but are not part of the cohort key.
CREATE TABLE game_server_save_occurrences (
 occurrence_id uuid PRIMARY KEY DEFAULT uuidv7(),
 world_id uuid NOT NULL,
 reset_epoch numeric(20,0) NOT NULL,
 scheduled_at timestamptz NOT NULL,
 freeze_until timestamptz NOT NULL CHECK (freeze_until > scheduled_at),
 map_revision text NOT NULL,
 server_digest bytea NOT NULL CHECK (octet_length(server_digest)=32),
 frame_digest bytea NOT NULL CHECK (octet_length(frame_digest)=32),
 cleanup_enabled boolean NOT NULL DEFAULT false,
 clean_protection_zones boolean NOT NULL DEFAULT false,
 UNIQUE (world_id, scheduled_at),
 FOREIGN KEY (world_id,reset_epoch) REFERENCES game_world_reset_epochs(world_id,reset_epoch)
);
CREATE INDEX game_server_save_epoch ON game_server_save_occurrences(world_id,reset_epoch,scheduled_at DESC);
CREATE TRIGGER game_server_save_occurrence_immutable BEFORE UPDATE OR DELETE
 ON game_server_save_occurrences FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE TABLE game_server_save_channels (
 occurrence_id uuid NOT NULL REFERENCES game_server_save_occurrences,
 channel_id uuid NOT NULL,
 state text NOT NULL CHECK (state IN ('frozen','complete','missed')),
 PRIMARY KEY (occurrence_id,channel_id)
);
CREATE TABLE game_server_save_ground (
 occurrence_id uuid NOT NULL,
 channel_id uuid NOT NULL,
 item_id uuid NOT NULL,
 evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence)='object'),
 disposition text NOT NULL CHECK (disposition IN ('pending','preserved','unknown','skipped','retired')),
 reason text,
 transaction_id uuid UNIQUE REFERENCES game_spell_item_receipts,
 FOREIGN KEY (occurrence_id,channel_id) REFERENCES game_server_save_channels,
 CHECK ((disposition='retired')=(transaction_id IS NOT NULL)),
 CHECK ((disposition IN ('preserved','unknown','skipped'))=(reason IS NOT NULL)),
 PRIMARY KEY (occurrence_id,channel_id,item_id)
);
CREATE INDEX game_server_save_ground_pending ON game_server_save_ground(occurrence_id,channel_id,item_id)
 WHERE disposition='pending';

REVOKE ALL ON game_server_save_occurrences,game_server_save_channels,game_server_save_ground FROM PUBLIC;
GRANT SELECT ON game_server_save_occurrences TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_server_save_channels,game_server_save_ground TO oteryn_game_runtime;
GRANT SELECT ON game_server_save_occurrences,game_server_save_channels,game_server_save_ground TO oteryn_game_control;

-- The save owner uses database time and a one-minute admission window.
-- The last Berlin slot is recovered after restart even if its window expired.
CREATE FUNCTION game_server_save_publish_due(p_world uuid,p_epoch numeric,p_cleanup boolean,p_protection boolean)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 observed timestamptz;
 scheduled timestamptz;
BEGIN
 IF NOT game_world_reset_map_epoch_current(p_world,p_epoch) THEN RETURN; END IF;
 observed:=clock_timestamp();
 scheduled:=(date_trunc('day',(observed AT TIME ZONE 'Europe/Berlin')-interval '6 hours')
             +interval '6 hours') AT TIME ZONE 'Europe/Berlin';
 INSERT INTO game_server_save_occurrences
   (world_id,reset_epoch,scheduled_at,freeze_until,map_revision,server_digest,frame_digest,cleanup_enabled,clean_protection_zones)
 SELECT e.world_id,e.reset_epoch,scheduled,scheduled+interval '1 minute',e.map_revision,e.server_digest,e.frame_digest,p_cleanup,p_protection
 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING (world_id,reset_epoch)
 WHERE a.world_id=p_world AND a.reset_epoch=p_epoch AND e.calendar='TIBIA_SERVER_SAVE_BERLIN_06'
   AND e.activated_at<=scheduled
 ON CONFLICT (world_id,scheduled_at) DO NOTHING;
END $$;

-- Pin the current Channel issuance and the positive House exclusion against writers.
CREATE FUNCTION game_server_save_pin(p_world uuid,p_channel uuid,p_epoch numeric,p_map text,p_server bytea,p_frame bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT game_world_reset_map_epoch_current(p_world,p_epoch) THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:'||p_world::text||':'||p_channel::text,0));
 LOCK TABLE game_house_tiles IN SHARE MODE;
 RETURN EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
    WHERE a.world_id=p_world AND a.reset_epoch=p_epoch
     AND (e.map_revision,e.server_digest,e.frame_digest)=(p_map,p_server,p_frame))
   AND EXISTS(SELECT 1 FROM (
      SELECT server_artifact_digest,frame_binding_digest FROM game_content_activations
      WHERE world_id=p_world AND channel_id=p_channel ORDER BY activation_sequence DESC LIMIT 1
    ) current_pin WHERE (server_artifact_digest,frame_binding_digest)=(p_server,p_frame));
END $$;

DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY['game_server_save_publish_due(uuid,numeric,boolean,boolean)'::regprocedure,
                           'game_server_save_pin(uuid,uuid,numeric,text,bytea,bytea)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;
