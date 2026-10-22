-- MAP-OVERLAY-1c; ADR-0021 §4.7, D1257, FND-04B owner decision 2026-10-22.
-- Epoch zero is explicitly initialized by Control, never seeded by installation.
CREATE TABLE game_world_reset_epochs (
 world_id UUID NOT NULL CHECK(game_node_is_uuid_v7(world_id)),
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615),
 target_digest BYTEA NOT NULL CHECK(octet_length(target_digest)=32 AND target_digest<>decode(repeat('00',32),'hex')),
 map_revision TEXT NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
 server_digest BYTEA NOT NULL CHECK(octet_length(server_digest)=32),
 client_digest BYTEA NOT NULL CHECK(octet_length(client_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 calendar TEXT NOT NULL CHECK(calendar='TIBIA_SERVER_SAVE_BERLIN_10'),
 calendar_revision NUMERIC(20,0) NOT NULL CHECK(calendar_revision BETWEEN 1 AND 18446744073709551615),
 activated_at TIMESTAMPTZ NOT NULL,
 reset_month TEXT NOT NULL CHECK(reset_month ~ '^[0-9]{4}-[0-9]{2}$'),
 PRIMARY KEY(world_id,reset_epoch)
);
CREATE TABLE game_world_reset_active (
 world_id UUID PRIMARY KEY,
 reset_epoch NUMERIC(20,0) NOT NULL,
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs
);
CREATE TABLE game_world_reset_jobs (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch >= 0 AND reset_epoch < 18446744073709551615),
 target_digest BYTEA NOT NULL CHECK(octet_length(target_digest)=32 AND target_digest<>decode(repeat('00',32),'hex')),
 retirement_binding BYTEA NOT NULL CHECK(octet_length(retirement_binding)=32 AND retirement_binding<>decode(repeat('00',32),'hex')),
 target_map_revision TEXT NOT NULL CHECK(octet_length(target_map_revision) BETWEEN 1 AND 512),
 target_server_digest BYTEA NOT NULL CHECK(octet_length(target_server_digest)=32),
 target_client_digest BYTEA NOT NULL CHECK(octet_length(target_client_digest)=32),
 target_frame_digest BYTEA NOT NULL CHECK(octet_length(target_frame_digest)=32),
 target_house_tiles JSONB NOT NULL CHECK(jsonb_typeof(target_house_tiles)='array'),
 calendar_revision NUMERIC(20,0) NOT NULL CHECK(calendar_revision BETWEEN 1 AND 18446744073709551615),
 phase TEXT NOT NULL CHECK(phase IN('RETIRING','ACTIVATED')),
 revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(revision BETWEEN 1 AND 18446744073709551615),
 published_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 CHECK(retirement_binding=sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || target_digest)),
 PRIMARY KEY(world_id,reset_epoch),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs
);
CREATE UNIQUE INDEX game_world_reset_one_retiring ON game_world_reset_jobs(world_id) WHERE phase='RETIRING';
CREATE TABLE game_world_reset_channels (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 channel_id UUID NOT NULL CHECK(game_node_is_uuid_v7(channel_id) AND channel_id<>world_id),
 predecessor_generation NUMERIC(20,0) NOT NULL CHECK(predecessor_generation BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,reset_epoch,channel_id),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs
);
REVOKE ALL ON game_world_reset_epochs,game_world_reset_active,game_world_reset_jobs,game_world_reset_channels FROM PUBLIC;
GRANT SELECT ON game_world_reset_epochs,game_world_reset_active,game_world_reset_jobs,game_world_reset_channels TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_world_reset_history_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'World reset history is immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_world_reset_epochs_immutable BEFORE UPDATE OR DELETE ON game_world_reset_epochs FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_world_reset_channels_immutable BEFORE UPDATE OR DELETE ON game_world_reset_channels FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

-- The record is locked after the existing admission relation fence, as in the
-- owning fresh-loss transaction. This probe grants no control or release rights.
CREATE FUNCTION game_world_reset_loss_binding(p_world UUID,p_epoch NUMERIC,p_binding BYTEA,p_target BYTEA)
 RETURNS BOOLEAN LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs
  WHERE world_id=p_world AND reset_epoch=p_epoch AND retirement_binding=p_binding AND target_digest=p_target AND phase='RETIRING' FOR SHARE)
$$;
REVOKE ALL ON FUNCTION game_world_reset_loss_binding(UUID,NUMERIC,BYTEA,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_loss_binding(UUID,NUMERIC,BYTEA,BYTEA) TO oteryn_game_runtime,oteryn_game_control;

-- Loss and readmission serialize with publication through the existing admission
-- relation locks. A prepared attempt can never cross RETIRING into ACTIVE.
CREATE FUNCTION game_world_reset_admission_closed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID;
BEGIN
 w:=NEW.world_id;
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 IF (TG_OP='INSERT' OR (NEW.session_state=2 AND (OLD.session_state<>2 OR (NEW.runtime_scope_kind,NEW.runtime_scope_world_id,NEW.runtime_scope_channel_id,NEW.runtime_scope_instance_id) IS DISTINCT FROM (OLD.runtime_scope_kind,OLD.runtime_scope_world_id,OLD.runtime_scope_channel_id,OLD.runtime_scope_instance_id)))) AND EXISTS(
  SELECT 1 FROM game_world_reset_jobs WHERE world_id=w AND phase='RETIRING') THEN
  RAISE EXCEPTION 'World reset admission is closed' USING ERRCODE='OTR03';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_world_reset_admission_closed BEFORE INSERT OR UPDATE ON game_durability_reconnect_sessions
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_admission_closed();
CREATE FUNCTION game_world_reset_attempt_closed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 IF EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=NEW.world_id AND phase='RETIRING') THEN
  RAISE EXCEPTION 'World reset admission is closed' USING ERRCODE='OTR03';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_world_reset_attempt_closed BEFORE INSERT ON game_durability_reconnect_attempts
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_attempt_closed();
CREATE TRIGGER game_world_reset_content_closed BEFORE INSERT ON game_content_activations
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_attempt_closed();
CREATE TRIGGER game_world_reset_cohort_closed BEFORE INSERT ON game_runtime_scope_assignments
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_attempt_closed();
DO $$
DECLARE f TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_world_reset_admission_closed','game_world_reset_attempt_closed'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',f);
 END LOOP;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_loss_binding(UUID,NUMERIC,BYTEA,BYTEA) SET search_path=%I,pg_temp',current_schema());
END $$;
