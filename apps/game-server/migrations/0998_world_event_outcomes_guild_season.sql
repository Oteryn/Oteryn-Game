-- T94 owner decision 2026-11-06: enabled by default; recovery still fails closed.
CREATE TABLE game_t94_control (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 enabled boolean NOT NULL DEFAULT true,
 projection_epoch numeric(20,0) NOT NULL DEFAULT 1 CHECK(projection_epoch BETWEEN 1 AND 18446744073709551615),
 recovery_generation numeric(20,0) NOT NULL DEFAULT 0 CHECK(recovery_generation BETWEEN 0 AND 18446744073709551615),
 reconciled boolean NOT NULL DEFAULT false,
 source_authority text NOT NULL DEFAULT 'oteryn:game:primary',
 source_started_at bigint NOT NULL DEFAULT 0 CHECK(source_started_at>=0),
 complete_through numeric(20,0) NOT NULL DEFAULT 0 CHECK(complete_through BETWEEN 0 AND 18446744073709551615),
 last_resync_request_id uuid,
 quarantined boolean NOT NULL DEFAULT false,
 resync_complete boolean NOT NULL DEFAULT false
);
INSERT INTO game_t94_control(singleton) VALUES(true);
CREATE TABLE game_t94_native_receipts (
 receipt_id uuid PRIMARY KEY DEFAULT uuidv7(),
 delivery_order bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
 world_id uuid NOT NULL,
 kind text NOT NULL CHECK(kind IN ('RAID','BOSS','DAILY_WORLD_GATE','WORLD_CHANGE','GUILD_SOURCE')),
 occurrence_id uuid NOT NULL,
 source_key text NOT NULL,
 source_cut text NOT NULL,
 recovery_generation numeric(20,0) NOT NULL CHECK(recovery_generation>0),
 receipt bytea NOT NULL CHECK(octet_length(receipt) BETWEEN 0 AND 16384),
 receipt_sha256 bytea NOT NULL CHECK(octet_length(receipt_sha256)=32),
 observed_at bigint NOT NULL CHECK(observed_at>0),
 expired_at bigint,
 UNIQUE(world_id,kind,occurrence_id,source_cut),
 UNIQUE(world_id,kind,source_key,source_cut)
);
CREATE INDEX game_t94_native_receipts_source ON game_t94_native_receipts(world_id,occurrence_id,source_key,recovery_generation,observed_at,receipt_id);
CREATE TABLE game_t94_raid_windows (
 firing_id uuid NOT NULL REFERENCES game_world_raid_firings(firing_id),
 channel_id uuid NOT NULL,
 owner_generation numeric(20,0) NOT NULL CHECK(owner_generation>0),
 first_wave_at_ms bigint NOT NULL CHECK(first_wave_at_ms>=0),
 deadline_ms bigint NOT NULL CHECK(deadline_ms>=first_wave_at_ms),
 recovery_generation numeric(20,0) NOT NULL CHECK(recovery_generation>0),
 PRIMARY KEY(firing_id,channel_id,owner_generation)
);
REVOKE ALL ON game_t94_raid_windows FROM PUBLIC;
GRANT SELECT,INSERT ON game_t94_raid_windows TO oteryn_game_runtime;
GRANT SELECT ON game_t94_raid_windows TO oteryn_game_control;
CREATE TABLE game_t94_evidence (
 evidence_sha256 bytea PRIMARY KEY CHECK(octet_length(evidence_sha256)=32),
 world_id uuid NOT NULL,
 event_id uuid NOT NULL,
 recovery_generation numeric(20,0) NOT NULL CHECK(recovery_generation>0),
 evidence bytea NOT NULL CHECK(octet_length(evidence)<=131072),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
REVOKE ALL ON game_t94_evidence FROM PUBLIC;
GRANT SELECT,INSERT ON game_t94_evidence TO oteryn_game_runtime;
GRANT UPDATE(evidence) ON game_t94_evidence TO oteryn_game_runtime;
GRANT SELECT ON game_t94_evidence TO oteryn_game_control;
CREATE TABLE game_t94_subjects (
 family text NOT NULL CHECK(family IN ('world_event_outcome','guild_season')),
 world_id uuid NOT NULL,
 subject_id uuid NOT NULL,
 occurrence_id uuid,
 question_key text,
 projection_revision numeric(20,0) NOT NULL CHECK(projection_revision BETWEEN 1 AND 18446744073709551615),
 record bytea NOT NULL,
 record_sha256 bytea NOT NULL CHECK(octet_length(record_sha256)=32),
 final_state boolean NOT NULL,
 display_withdrawn boolean NOT NULL DEFAULT false,
 final_at bigint,
 PRIMARY KEY(family,world_id,subject_id),
 UNIQUE(subject_id),
 UNIQUE(world_id,occurrence_id,question_key)
);
CREATE TABLE game_t94_publications (
 publication_id uuid PRIMARY KEY DEFAULT uuidv7(),
 delivery_order bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
 family text NOT NULL,
 world_id uuid NOT NULL,
 subject_id uuid NOT NULL,
 projection_epoch numeric(20,0) NOT NULL CHECK(projection_epoch BETWEEN 1 AND 18446744073709551615),
 projection_revision numeric(20,0) NOT NULL CHECK(projection_revision BETWEEN 1 AND 18446744073709551615),
 source_observed_at bigint NOT NULL CHECK(source_observed_at>0),
 wire bytea NOT NULL CHECK(octet_length(wire) BETWEEN 1 AND 262144),
 wire_sha256 bytea NOT NULL CHECK(octet_length(wire_sha256)=32),
 audit_event_id uuid NOT NULL DEFAULT uuidv7(),
 transaction_id uuid NOT NULL DEFAULT uuidv7(),
 occurred_at_ms bigint NOT NULL CHECK(occurred_at_ms>0),
 audit bytea NOT NULL CHECK(octet_length(audit)<=262144),
 resync_member boolean NOT NULL DEFAULT false,
 acknowledged boolean NOT NULL DEFAULT false,
 audit_acknowledged boolean NOT NULL DEFAULT false,
 UNIQUE(family,world_id,subject_id,projection_epoch,projection_revision),
 FOREIGN KEY(family,world_id,subject_id) REFERENCES game_t94_subjects(family,world_id,subject_id)
);
CREATE INDEX game_t94_publications_pending ON game_t94_publications(family,delivery_order) WHERE NOT acknowledged;
CREATE INDEX game_t94_publications_subject ON game_t94_publications(family,world_id,subject_id);
CREATE INDEX game_t94_publications_audit ON game_t94_publications(occurred_at_ms,publication_id) WHERE NOT audit_acknowledged;
CREATE TABLE game_t94_delivery_lanes (
 family text PRIMARY KEY CHECK(family IN ('world_event_outcome','guild_season')),
 token uuid,
 lease_until timestamptz
);
INSERT INTO game_t94_delivery_lanes(family) VALUES('world_event_outcome'),('guild_season');
REVOKE ALL ON game_t94_delivery_lanes FROM PUBLIC;
GRANT SELECT,UPDATE ON game_t94_delivery_lanes TO oteryn_game_runtime;
GRANT SELECT ON game_t94_delivery_lanes TO oteryn_game_control;
REVOKE ALL ON SEQUENCE game_t94_publications_delivery_order_seq FROM PUBLIC;
GRANT USAGE ON SEQUENCE game_t94_publications_delivery_order_seq TO oteryn_game_runtime;
CREATE TABLE game_t94_contributions (
 season_id uuid NOT NULL,
 challenge_id uuid NOT NULL,
 occurrence_id uuid NOT NULL,
 guild_id uuid NOT NULL,
 world_id uuid NOT NULL,
 score numeric(20,0) NOT NULL CHECK(score BETWEEN 0 AND 18446744073709551615),
 guild_name text NOT NULL,
 membership_cut bytea NOT NULL CHECK(octet_length(membership_cut)=32),
 source_receipt_id uuid NOT NULL REFERENCES game_t94_native_receipts(receipt_id),
 revision numeric(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 receipt_sha256 bytea NOT NULL CHECK(octet_length(receipt_sha256)=32),
 PRIMARY KEY(season_id,challenge_id,occurrence_id,guild_id)
);
CREATE INDEX game_t94_contributions_source ON game_t94_contributions(source_receipt_id);
CREATE INDEX game_t94_contributions_season ON game_t94_contributions(world_id,season_id,guild_id,challenge_id);
CREATE TABLE game_t94_world_policies (
 world_id uuid PRIMARY KEY,
 first_start bigint NOT NULL CHECK(first_start>0),
 fury_gate_realization_available boolean NOT NULL DEFAULT false,
 defaults_sha256 bytea NOT NULL CHECK(octet_length(defaults_sha256)=32),
 season_seconds bigint NOT NULL CHECK(season_seconds BETWEEN 1 AND 31536000)
);
REVOKE ALL ON game_t94_world_policies FROM PUBLIC;
GRANT SELECT,INSERT ON game_t94_world_policies TO oteryn_game_runtime;
GRANT SELECT ON game_t94_world_policies TO oteryn_game_control;
CREATE TABLE game_t94_default_seasons (
 world_id uuid NOT NULL,
 starts_at bigint NOT NULL CHECK(starts_at>0),
 season_id uuid NOT NULL DEFAULT uuidv7(),
 challenge_id uuid NOT NULL DEFAULT uuidv7(),
 PRIMARY KEY(world_id,starts_at),
 UNIQUE(season_id)
);
REVOKE ALL ON game_t94_default_seasons FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_t94_default_seasons TO oteryn_game_runtime;
GRANT SELECT ON game_t94_default_seasons TO oteryn_game_control;
CREATE TABLE game_t94_privacy_watch (
 guild_id uuid NOT NULL,
 world_id uuid NOT NULL,
 season_id uuid NOT NULL,
 public_name text NOT NULL,
 PRIMARY KEY(guild_id,world_id,season_id)
);
CREATE INDEX game_t94_privacy_watch_season ON game_t94_privacy_watch(world_id,season_id);
REVOKE ALL ON game_t94_privacy_watch FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_t94_privacy_watch TO oteryn_game_runtime;
GRANT SELECT ON game_t94_privacy_watch TO oteryn_game_control;
CREATE TABLE game_t94_corrections (
 request_id uuid PRIMARY KEY DEFAULT uuidv7(),
 request bytea NOT NULL CHECK(octet_length(request) BETWEEN 1 AND 4096),
 approved boolean NOT NULL DEFAULT false,
 approved_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 receipt bytea,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
REVOKE ALL ON game_t94_corrections FROM PUBLIC;
GRANT SELECT ON game_t94_corrections TO oteryn_game_runtime,oteryn_game_control;
GRANT UPDATE(receipt) ON game_t94_corrections TO oteryn_game_runtime;
CREATE FUNCTION game_t94_immutable_correction() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN NEW.approved_at=clock_timestamp();RETURN NEW;END IF;
 IF OLD.approved AND OLD.approved_at IS DISTINCT FROM NEW.approved_at THEN RAISE EXCEPTION 'immutable T94 approval cut' USING ERRCODE='23514';END IF;
 IF NOT OLD.approved AND NEW.approved THEN NEW.approved_at=clock_timestamp();END IF;
 IF OLD.request IS DISTINCT FROM NEW.request OR (OLD.receipt IS NOT NULL AND OLD.receipt IS DISTINCT FROM NEW.receipt) THEN
  RAISE EXCEPTION 'immutable T94 correction' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_t94_immutable_correction BEFORE INSERT OR UPDATE ON game_t94_corrections FOR EACH ROW EXECUTE FUNCTION game_t94_immutable_correction();
CREATE TABLE game_t94_bindings (
 family text NOT NULL CHECK(family IN ('world_event_outcome','guild_season')),
 world_id uuid NOT NULL,
 subject_id uuid NOT NULL,
 approved boolean NOT NULL DEFAULT false,
 approved_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 binding bytea NOT NULL CHECK(octet_length(binding) BETWEEN 1 AND 262144),
 checked_at bigint NOT NULL DEFAULT 0,
 native_seen_order bigint NOT NULL DEFAULT 0,
 PRIMARY KEY(family,world_id,subject_id)
);
CREATE INDEX game_t94_bindings_ready ON game_t94_bindings(checked_at,family,world_id,subject_id) WHERE approved;
REVOKE ALL ON game_t94_bindings FROM PUBLIC;
GRANT SELECT ON game_t94_bindings TO oteryn_game_runtime,oteryn_game_control;
GRANT INSERT ON game_t94_bindings TO oteryn_game_runtime;
GRANT UPDATE(checked_at,binding,native_seen_order) ON game_t94_bindings TO oteryn_game_runtime;
GRANT UPDATE(source_started_at,source_authority,recovery_generation,reconciled,resync_complete,complete_through,quarantined,last_resync_request_id,projection_epoch) ON game_t94_control TO oteryn_game_runtime;
REVOKE ALL ON game_t94_control,game_t94_native_receipts,game_t94_subjects,game_t94_publications,game_t94_contributions FROM PUBLIC;
GRANT SELECT ON game_t94_control TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_t94_native_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_t94_subjects,game_t94_publications,game_t94_contributions TO oteryn_game_runtime;
GRANT SELECT ON game_t94_control,game_t94_native_receipts,game_t94_subjects,game_t94_publications,game_t94_contributions TO oteryn_game_control;

GRANT DELETE ON game_t94_publications TO oteryn_game_runtime;
GRANT UPDATE(receipt,expired_at) ON game_t94_native_receipts TO oteryn_game_runtime;

REVOKE ALL ON SEQUENCE game_t94_native_receipts_delivery_order_seq FROM PUBLIC;
GRANT USAGE ON SEQUENCE game_t94_native_receipts_delivery_order_seq TO oteryn_game_runtime;
CREATE INDEX game_t94_native_receipts_family_order ON game_t94_native_receipts(world_id,kind,delivery_order);
CREATE FUNCTION game_t94_binding_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE old_binding jsonb;new_binding jsonb;
BEGIN
 IF (OLD.family,OLD.world_id,OLD.subject_id) IS DISTINCT FROM (NEW.family,NEW.world_id,NEW.subject_id) THEN RAISE EXCEPTION 'immutable T94 binding identity' USING ERRCODE='23514';END IF;
 IF OLD.approved AND OLD.approved_at IS DISTINCT FROM NEW.approved_at THEN RAISE EXCEPTION 'immutable T94 approval cut' USING ERRCODE='23514';END IF;
 IF NOT OLD.approved AND NEW.approved THEN NEW.approved_at=clock_timestamp();END IF;
 IF (OLD.family='guild_season' AND (convert_from(OLD.binding,'UTF8')::jsonb->'season'->>'starts_at')::numeric<=floor(extract(epoch FROM statement_timestamp()))) OR EXISTS(SELECT 1 FROM game_t94_subjects s WHERE (s.family,s.world_id,s.subject_id)=(OLD.family,OLD.world_id,OLD.subject_id)) THEN
  old_binding=convert_from(OLD.binding,'UTF8')::jsonb;
  new_binding=convert_from(NEW.binding,'UTF8')::jsonb;
  IF coalesce((new_binding->>'complete_through')::numeric,0)<coalesce((old_binding->>'complete_through')::numeric,0) THEN RAISE EXCEPTION 'T94 coverage rollback' USING ERRCODE='23514'; END IF;
  IF (old_binding->>'corrections_closed')::boolean AND NOT coalesce((new_binding->>'corrections_closed')::boolean,false) THEN
   RAISE EXCEPTION 'closed T94 correction window' USING ERRCODE='23514';
  END IF;
  IF ((old_binding-ARRAY['complete_through','withdraw_display','corrections_closed','tombstone_name'])#-'{season,state}') IS DISTINCT FROM
     ((new_binding-ARRAY['complete_through','withdraw_display','corrections_closed','tombstone_name'])#-'{season,state}') THEN
   RAISE EXCEPTION 'immutable T94 binding' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_t94_binding_guard BEFORE UPDATE ON game_t94_bindings FOR EACH ROW EXECUTE FUNCTION game_t94_binding_guard();
CREATE INDEX game_t94_subjects_retention ON game_t94_subjects(final_at,family,world_id,subject_id) WHERE final_state;
CREATE INDEX game_t94_native_retention ON game_t94_native_receipts(observed_at,receipt_id) WHERE octet_length(receipt)>0;
CREATE INDEX game_t94_audit_retention ON game_t94_publications(occurred_at_ms,delivery_order) WHERE octet_length(audit)>0;

-- Transport acknowledgement can never rewrite a committed pair's content.
CREATE FUNCTION game_t94_publication_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  IF NOT EXISTS(SELECT 1 FROM game_t94_subjects s WHERE (s.family,s.world_id,s.subject_id)=(OLD.family,OLD.world_id,OLD.subject_id) AND s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000) THEN
   RAISE EXCEPTION 'T94 serving retention has not expired' USING ERRCODE='23514';
  END IF;
  RETURN OLD;
 END IF;
 IF (to_jsonb(OLD)-ARRAY['acknowledged','audit_acknowledged','audit']) IS DISTINCT FROM
    (to_jsonb(NEW)-ARRAY['acknowledged','audit_acknowledged','audit']) OR
    (OLD.acknowledged AND NOT NEW.acknowledged) OR (OLD.audit_acknowledged AND NOT NEW.audit_acknowledged) THEN
  RAISE EXCEPTION 'immutable T94 publication' USING ERRCODE='23514';
 END IF;
 IF OLD.audit IS DISTINCT FROM NEW.audit AND NOT (octet_length(NEW.audit)=0 AND OLD.occurred_at_ms<floor(extract(epoch FROM statement_timestamp())*1000)::bigint-7776000000) THEN
  RAISE EXCEPTION 'T94 audit retention has not expired' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_t94_publication_guard BEFORE UPDATE OR DELETE ON game_t94_publications FOR EACH ROW EXECUTE FUNCTION game_t94_publication_guard();

CREATE FUNCTION game_t94_binding_insert_approval() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.approved_at=clock_timestamp();RETURN NEW;END $$;
CREATE TRIGGER game_t94_binding_insert_approval BEFORE INSERT ON game_t94_bindings FOR EACH ROW EXECUTE FUNCTION game_t94_binding_insert_approval();

-- Approval is an input cut too. Take the barrier before acquiring row locks,
-- so freeze/completeness cannot race an uncommitted configuration approval.
CREATE FUNCTION game_t94_configuration_barrier() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM singleton FROM game_t94_control WHERE singleton FOR SHARE;
 RETURN NULL;
END $$;
CREATE TRIGGER game_t94_binding_configuration_barrier BEFORE INSERT OR UPDATE ON game_t94_bindings FOR EACH STATEMENT EXECUTE FUNCTION game_t94_configuration_barrier();
CREATE TRIGGER game_t94_correction_configuration_barrier BEFORE INSERT OR UPDATE ON game_t94_corrections FOR EACH STATEMENT EXECUTE FUNCTION game_t94_configuration_barrier();
