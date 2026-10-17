
-- D1257/0096. Explicit Control initialization only.
CREATE FUNCTION game_world_reset_month_key(p_instant TIMESTAMPTZ,p_policy TEXT,p_revision NUMERIC)
RETURNS TEXT LANGUAGE SQL IMMUTABLE STRICT AS $$
 SELECT CASE WHEN isfinite(p_instant) AND p_policy='TIBIA_SERVER_SAVE_BERLIN_10'
 AND p_revision BETWEEN 1 AND 18446744073709551615 AND trunc(p_revision)=p_revision
 THEN to_char((p_instant AT TIME ZONE 'Europe/Berlin')-INTERVAL '10 hours','YYYY-MM') ELSE NULL END;
$$;
CREATE TABLE game_world_reset_epoch_history (
 world_id UUID NOT NULL CHECK(game_node_is_uuid_v7(world_id)),
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615),
 map_revision TEXT NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
 server_digest BYTEA NOT NULL CHECK(octet_length(server_digest)=32),
 client_digest BYTEA NOT NULL CHECK(octet_length(client_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 calendar_policy TEXT NOT NULL CHECK(calendar_policy='TIBIA_SERVER_SAVE_BERLIN_10'),
 calendar_revision NUMERIC(20,0) NOT NULL CHECK(calendar_revision BETWEEN 1 AND 18446744073709551615),
 calendar_parity TEXT NOT NULL CHECK(calendar_parity='PARITY_PENDING'),
 month_key TEXT NOT NULL CHECK(month_key ~ '^[0-9]{4}-[0-9]{2}$'),
 recorded_at TIMESTAMPTZ NOT NULL CHECK(isfinite(recorded_at)),
 issued_by TEXT NOT NULL,
 PRIMARY KEY(world_id,reset_epoch)
);
CREATE TABLE game_world_reset_active_epochs (
 world_id UUID PRIMARY KEY, reset_epoch NUMERIC(20,0) NOT NULL,
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epoch_history(world_id,reset_epoch)
);
CREATE TRIGGER game_world_reset_history_immutable BEFORE UPDATE OR DELETE ON game_world_reset_epoch_history FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_world_reset_history_no_truncate BEFORE TRUNCATE ON game_world_reset_epoch_history FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_world_reset_active_no_delete BEFORE DELETE ON game_world_reset_active_epochs FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_world_reset_active_no_truncate BEFORE TRUNCATE ON game_world_reset_active_epochs FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_world_reset_epoch_history,game_world_reset_active_epochs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_world_reset_epoch_history,game_world_reset_active_epochs TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_world_reset_initialize_epoch(p_world UUID,p_map TEXT,p_server BYTEA,p_client BYTEA,p_frame BYTEA,p_policy TEXT,p_revision NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE v_existing game_world_reset_epoch_history%ROWTYPE; v_instant TIMESTAMPTZ; v_month TEXT;
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:world-reset:'||p_world::text,0));
 SELECT * INTO v_existing FROM game_world_reset_epoch_history WHERE world_id=p_world AND reset_epoch=0;
 IF FOUND THEN
  IF (v_existing.map_revision,v_existing.server_digest,v_existing.client_digest,v_existing.frame_digest,v_existing.calendar_policy,v_existing.calendar_revision)
   IS NOT DISTINCT FROM (p_map,p_server,p_client,p_frame,p_policy,p_revision)
   AND EXISTS(SELECT 1 FROM game_world_reset_active_epochs WHERE world_id=p_world) THEN RETURN; END IF;
  RAISE EXCEPTION 'World reset initialization conflicts' USING ERRCODE='OTR01';
 END IF;
 IF EXISTS(SELECT 1 FROM game_world_reset_epoch_history WHERE world_id=p_world) THEN
  RAISE EXCEPTION 'World reset initialization is not fresh' USING ERRCODE='OTR01'; END IF;
 v_instant:=clock_timestamp();
 v_month:=game_world_reset_month_key(v_instant,p_policy,p_revision);
 IF p_world IS NULL OR NOT game_node_is_uuid_v7(p_world) OR p_server IS NULL OR p_client IS NULL OR p_frame IS NULL OR p_map IS NULL OR octet_length(p_map) NOT BETWEEN 1 AND 512 OR octet_length(p_server)<>32 OR octet_length(p_client)<>32 OR octet_length(p_frame)<>32 OR v_month IS NULL THEN
  RAISE EXCEPTION 'Invalid World reset initialization' USING ERRCODE='OTR01'; END IF;
 INSERT INTO game_world_reset_epoch_history(world_id,reset_epoch,map_revision,server_digest,client_digest,frame_digest,calendar_policy,calendar_revision,calendar_parity,month_key,recorded_at,issued_by)
 VALUES(p_world,0,p_map,p_server,p_client,p_frame,p_policy,p_revision,'PARITY_PENDING',v_month,v_instant,session_user);
 INSERT INTO game_world_reset_active_epochs(world_id,reset_epoch) VALUES(p_world,0);
END $$;
REVOKE ALL ON FUNCTION game_world_reset_initialize_epoch(UUID,TEXT,BYTEA,BYTEA,BYTEA,TEXT,NUMERIC) FROM PUBLIC,oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_world_reset_initialize_epoch(UUID,TEXT,BYTEA,BYTEA,BYTEA,TEXT,NUMERIC) TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_world_reset_month_key(TIMESTAMPTZ,TEXT,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_month_key(TIMESTAMPTZ,TEXT,NUMERIC) TO oteryn_game_runtime,oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_initialize_epoch(uuid,text,bytea,bytea,bytea,text,numeric) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_world_reset_month_key(timestamptz,text,numeric) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- These are reset records/cohorts, NOT a second retirement ledger. Only control publishes them.
CREATE TABLE game_world_reset_jobs (
 world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551614),
 target_digest BYTEA NOT NULL CHECK(octet_length(target_digest)=32),
 active_map_revision TEXT NOT NULL CHECK(octet_length(active_map_revision) BETWEEN 1 AND 512),
 -- Separate target facts, never aliases of target_digest or active_map_revision.
 -- All NULL is a legacy retirement-only job; final activation must refuse it.
 target_map_revision TEXT CHECK(octet_length(target_map_revision) BETWEEN 1 AND 512),
 target_server_digest BYTEA CHECK(octet_length(target_server_digest)=32),
 target_client_digest BYTEA CHECK(octet_length(target_client_digest)=32),
 target_frame_digest BYTEA CHECK(octet_length(target_frame_digest)=32),
 target_calendar_policy TEXT CHECK(target_calendar_policy='TIBIA_SERVER_SAVE_BERLIN_10'),
 target_calendar_revision NUMERIC(20,0) CHECK(target_calendar_revision BETWEEN 1 AND 18446744073709551615),
 target_calendar_parity TEXT CHECK(target_calendar_parity='PARITY_PENDING'),
 CHECK ((target_map_revision IS NULL AND target_server_digest IS NULL AND target_client_digest IS NULL AND target_frame_digest IS NULL AND target_calendar_policy IS NULL AND target_calendar_revision IS NULL AND target_calendar_parity IS NULL)
 OR (target_map_revision IS NOT NULL AND target_server_digest IS NOT NULL AND target_client_digest IS NOT NULL AND target_frame_digest IS NOT NULL AND target_calendar_policy IS NOT NULL AND target_calendar_revision IS NOT NULL AND target_calendar_parity IS NOT NULL)),
 phase TEXT NOT NULL CHECK(phase IN('RETIRING','ACTIVATED')),
 revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(revision BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,reset_epoch)
);
CREATE FUNCTION game_world_reset_job_initialized() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.phase IS DISTINCT FROM 'RETIRING' THEN
  RAISE EXCEPTION 'World reset jobs must start RETIRING' USING ERRCODE='23514'; END IF;
 PERFORM 1 FROM game_world_reset_active_epochs a JOIN game_world_reset_epoch_history h USING(world_id,reset_epoch)
 WHERE a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch AND h.map_revision=NEW.active_map_revision FOR SHARE OF a;
 IF NOT FOUND THEN
 RAISE EXCEPTION 'World reset epoch not initialized/current' USING ERRCODE='OTR02'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_world_reset_job_initialized BEFORE INSERT ON game_world_reset_jobs FOR EACH ROW EXECUTE FUNCTION game_world_reset_job_initialized();
CREATE UNIQUE INDEX game_world_reset_one_retiring ON game_world_reset_jobs(world_id) WHERE phase='RETIRING';
CREATE TABLE game_world_reset_scopes (
 world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL, channel_id UUID NOT NULL,
 predecessor_generation NUMERIC(20,0) NOT NULL CHECK(predecessor_generation BETWEEN 1 AND 18446744073709551614),
 PRIMARY KEY(world_id,reset_epoch,channel_id),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch)
);
CREATE FUNCTION game_world_reset_job_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.phase='RETIRING' AND NEW.revision=OLD.revision+1
 AND (to_jsonb(NEW)-'revision')=(to_jsonb(OLD)-'revision') THEN RETURN NEW; END IF;
 IF TG_OP='UPDATE' AND OLD.phase='RETIRING' AND NEW.phase='ACTIVATED' AND NEW.revision=OLD.revision+1
 AND (to_jsonb(NEW)-'revision'-'phase')=(to_jsonb(OLD)-'revision'-'phase')
 AND EXISTS(SELECT 1 FROM game_world_reset_active_epochs a JOIN game_world_reset_epoch_history h USING(world_id,reset_epoch)
 WHERE a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch+1
 AND (h.map_revision,h.server_digest,h.client_digest,h.frame_digest,h.calendar_policy,h.calendar_revision,h.calendar_parity)
 IS NOT DISTINCT FROM (NEW.target_map_revision,NEW.target_server_digest,NEW.target_client_digest,NEW.target_frame_digest,NEW.target_calendar_policy,NEW.target_calendar_revision,NEW.target_calendar_parity))
 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'reset job binding is immutable; activation belongs to its inseparable owner' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_world_reset_job_change BEFORE UPDATE OR DELETE ON game_world_reset_jobs FOR EACH ROW EXECUTE FUNCTION game_world_reset_job_change();
CREATE TRIGGER game_world_reset_scope_immutable BEFORE UPDATE OR DELETE ON game_world_reset_scopes FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_world_reset_jobs_no_truncate BEFORE TRUNCATE ON game_world_reset_jobs FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_world_reset_scopes_no_truncate BEFORE TRUNCATE ON game_world_reset_scopes FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_world_reset_jobs,game_world_reset_scopes FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_reset_jobs,game_world_reset_scopes TO oteryn_game_control;
GRANT SELECT ON game_world_reset_jobs,game_world_reset_scopes TO oteryn_game_runtime;
GRANT UPDATE(revision) ON game_world_reset_jobs TO oteryn_game_runtime;

ALTER TABLE game_item_decay_retire_reservations ADD COLUMN cause_kind TEXT NOT NULL DEFAULT 'CORPSE_DECAY' CHECK(cause_kind IN('CORPSE_DECAY','WORLD_RESET'));
ALTER TABLE game_item_decay_retire_reservations ADD COLUMN reset_epoch NUMERIC(20,0), ADD COLUMN reset_parent_item UUID;
ALTER TABLE game_item_decay_retire_reservations ALTER COLUMN corpse_item_instance_id DROP NOT NULL, ALTER COLUMN deadline DROP NOT NULL;
ALTER TABLE game_item_decay_retire_receipts ADD COLUMN cause_kind TEXT NOT NULL DEFAULT 'CORPSE_DECAY' CHECK(cause_kind IN('CORPSE_DECAY','WORLD_RESET'));
ALTER TABLE game_item_decay_retire_receipts ADD COLUMN reset_epoch NUMERIC(20,0), ADD COLUMN reset_parent_item UUID;
ALTER TABLE game_item_decay_retire_receipts ALTER COLUMN corpse_item_instance_id DROP NOT NULL, ALTER COLUMN deadline DROP NOT NULL;
ALTER TABLE game_item_decay_retire_reservations ADD CHECK (
 (cause_kind='CORPSE_DECAY' AND corpse_item_instance_id IS NOT NULL AND deadline IS NOT NULL AND reset_epoch IS NULL AND reset_parent_item IS NULL) OR
 (cause_kind='WORLD_RESET' AND corpse_item_instance_id IS NULL AND deadline IS NULL AND reset_epoch IS NOT NULL AND reset_parent_item IS NOT NULL AND ((item_instance_id=reset_parent_item)=(placement_ordinal IS NULL)))
);
ALTER TABLE game_item_decay_retire_receipts ADD CHECK (
 (cause_kind='CORPSE_DECAY' AND corpse_item_instance_id IS NOT NULL AND deadline IS NOT NULL AND reset_epoch IS NULL AND reset_parent_item IS NULL) OR
 (cause_kind='WORLD_RESET' AND corpse_item_instance_id IS NULL AND deadline IS NULL AND reset_epoch IS NOT NULL AND reset_parent_item IS NOT NULL AND ((item_instance_id=reset_parent_item)=(placement_ordinal IS NULL)))
);
ALTER TABLE game_item_decay_retire_reservations ADD FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch);
ALTER TABLE game_item_decay_retire_receipts ADD FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch);

CREATE FUNCTION game_world_reset_cohort_current(w UUID,e NUMERIC,n UUID,r NUMERIC) RETURNS BOOLEAN
LANGUAGE SQL SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=w AND j.reset_epoch=e AND j.phase='RETIRING')
 AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=w AND a.scope_kind=1)
 AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
 LEFT JOIN game_world_reset_scopes c ON (c.world_id,c.reset_epoch,c.channel_id)=(a.world_id,e,a.channel_id)
 LEFT JOIN game_durability_admission_runtime_guards g USING(scope_key)
 WHERE a.world_id=w AND a.scope_kind=1 AND (c.channel_id IS NULL OR a.state<>1 OR a.ownership_generation<=c.predecessor_generation OR a.holder_node_id<>n OR a.holder_registration_revision<>r OR g.scope_key IS NULL OR g.ready OR g.ownership_generation<>a.ownership_generation))
 AND NOT EXISTS(SELECT 1 FROM game_world_reset_scopes c WHERE c.world_id=w AND c.reset_epoch=e AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=w AND a.scope_kind=1 AND a.channel_id=c.channel_id))
 AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.world_id=w AND s.session_state IN(1,2));
$$;
-- This proof is shared by native deferred guards. Every positive reset clause still requires
-- its complete reservation, quantity evidence, actual pre-DELETE location and audit in THIS xact.
CREATE FUNCTION game_world_reset_item_proven(p UUID) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d game_item_decay_retire_receipts%ROWTYPE; v game_item_decay_retire_reservations%ROWTYPE;
BEGIN
 SELECT * INTO d FROM game_item_decay_retire_receipts WHERE item_instance_id=p AND cause_kind='WORLD_RESET' AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO v FROM game_item_decay_retire_reservations WHERE item_instance_id=p AND fence_scope_ownership_generation=d.fence_scope_ownership_generation;
 IF NOT FOUND OR v.cause_kind<>'WORLD_RESET' OR v.work_units_used<1
 OR (v.world_id,v.channel_id,v.transaction_id,v.event_id,v.quantity_before,v.occurred_at,v.reset_epoch,v.reset_parent_item)
 IS DISTINCT FROM (d.world_id,d.channel_id,d.transaction_id,d.event_id,d.quantity_before,d.occurred_at,d.reset_epoch,d.reset_parent_item)
 OR v.placement_ordinal IS DISTINCT FROM d.placement_ordinal OR sha256(v.envelope)<>d.envelope_sha256
 OR NOT game_world_reset_cohort_current(d.world_id,d.reset_epoch,v.fence_holder_node_id,v.fence_holder_registration_revision)
 OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=d.world_id AND a.channel_id=d.channel_id AND a.state=1 AND a.ownership_generation=d.fence_scope_ownership_generation AND a.holder_node_id=v.fence_holder_node_id AND a.holder_registration_revision=v.fence_holder_registration_revision)
 OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p AND i.world_id=d.world_id AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=d.transaction_id)
 OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=p AND q.transaction_id=d.transaction_id AND q.quantity_before=d.quantity_before)
 OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=d.event_id AND a.transaction_id=d.transaction_id AND a.item_instance_id=p AND a.envelope_sha256=d.envelope_sha256 AND a.envelope=v.envelope AND a.occurred_at=d.occurred_at AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL)
 OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p)
 THEN RAISE EXCEPTION 'WorldReset requires exact current reservation/receipt/retirement/audit' USING ERRCODE='23514'; END IF;
 IF d.placement_ordinal IS NULL THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_ground_removal_evidence g WHERE g.item_instance_id=p AND g.world_id=d.world_id AND g.channel_id=d.channel_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=p)
  THEN RAISE EXCEPTION 'WorldReset root requires actual Ground removal and no contents' USING ERRCODE='23514'; END IF;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_item_corpse_entry_removal_evidence c JOIN game_item_ground_locations g ON g.item_instance_id=c.parent_item_instance_id JOIN game_item_instances i ON i.item_instance_id=g.item_instance_id
    WHERE c.item_instance_id=p AND c.parent_item_instance_id=d.reset_parent_item AND c.placement_ordinal=d.placement_ordinal AND c.world_id=d.world_id AND c.channel_id=d.channel_id AND c.created_xact_id=pg_current_xact_id() AND g.world_id=d.world_id AND g.channel_id=d.channel_id AND i.lifecycle=1)
  THEN RAISE EXCEPTION 'WorldReset entry requires actual live Ground parent' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN TRUE;
END $$;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_world_reset_cohort_current(uuid,numeric,uuid,numeric)','game_world_reset_item_proven(uuid)','game_world_reset_job_initialized()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;

CREATE FUNCTION game_content_record_activation_core(
    p_world_id UUID, p_channel_id UUID, p_sequence NUMERIC, p_previous NUMERIC,
    p_server BYTEA, p_client BYTEA, p_frame BYTEA
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_existing game_content_activations%ROWTYPE;
    v_current NUMERIC;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'oteryn:content-activation:' || p_world_id::text || ':' || p_channel_id::text, 0));
    SELECT * INTO v_existing FROM game_content_activations
        WHERE world_id = p_world_id AND channel_id = p_channel_id
          AND activation_sequence = p_sequence;
    IF FOUND THEN
        IF v_existing.previous_sequence IS NOT DISTINCT FROM p_previous
           AND (v_existing.server_artifact_digest, v_existing.client_artifact_digest,
                v_existing.frame_binding_digest) = (p_server, p_client, p_frame) THEN
            RETURN;
        END IF;
        RAISE EXCEPTION 'content activation conflicts with the recorded sequence' USING ERRCODE = 'OTC01';
    END IF;
    SELECT max(activation_sequence) INTO v_current FROM game_content_activations
        WHERE world_id = p_world_id AND channel_id = p_channel_id;
    IF p_previous IS DISTINCT FROM v_current OR p_sequence <= coalesce(v_current, 0) THEN
        RAISE EXCEPTION 'content activation predecessor is stale or its sequence is not newer'
            USING ERRCODE = 'OTC01';
    END IF;
    INSERT INTO game_content_activations (world_id, channel_id, activation_sequence,
        previous_sequence, server_artifact_digest, client_artifact_digest, frame_binding_digest,
        issued_by, issued_at)
    VALUES (p_world_id, p_channel_id, p_sequence, p_previous, p_server, p_client, p_frame,
        session_user, floor(extract(epoch FROM statement_timestamp()))::BIGINT);
END; $$;
CREATE OR REPLACE FUNCTION game_content_record_activation(
    p_world_id UUID, p_channel_id UUID, p_sequence NUMERIC, p_previous NUMERIC,
    p_server BYTEA, p_client BYTEA, p_frame BYTEA
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_control_scope_grants g
        WHERE g.control_role = session_user AND g.world_id = p_world_id
          AND g.channel_id = p_channel_id AND g.operation = 4
    ) THEN
        RAISE EXCEPTION 'content activation is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    PERFORM game_content_record_activation_core(p_world_id,p_channel_id,p_sequence,p_previous,p_server,p_client,p_frame);
END; $$;
REVOKE ALL ON FUNCTION game_content_record_activation_core(UUID,UUID,NUMERIC,NUMERIC,BYTEA,BYTEA,BYTEA) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $$ BEGIN
EXECUTE format('ALTER FUNCTION game_content_record_activation_core(uuid,uuid,numeric,numeric,bytea,bytea,bytea) SET search_path = pg_catalog, %I, pg_temp',current_schema());
EXECUTE format('ALTER FUNCTION game_content_record_activation(uuid,uuid,numeric,numeric,bytea,bytea,bytea) SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;

CREATE FUNCTION game_world_reset_u64_be(v NUMERIC) RETURNS BYTEA LANGUAGE SQL IMMUTABLE STRICT AS $$
 SELECT set_byte(x,0,get_byte(x,0)#128) FROM (SELECT int8send((v-9223372036854775808)::int8) AS x) s;
$$;
REVOKE ALL ON FUNCTION game_world_reset_u64_be(NUMERIC) FROM PUBLIC;
ALTER FUNCTION game_world_reset_u64_be(NUMERIC) SET search_path=pg_catalog,pg_temp;
CREATE FUNCTION game_world_reset_activate(w UUID,e NUMERIC,r NUMERIC,b BYTEA,n UUID,nr NUMERIC,
 d BYTEA,hk TEXT[],hp BYTEA[],hs TEXT[])
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE j game_world_reset_jobs%ROWTYPE; a NUMERIC; v_channel RECORD;
 v_history game_world_reset_epoch_history%ROWTYPE; v_current game_content_activations%ROWTYPE;
 v_previous NUMERIC; v_sequence NUMERIC; v_instant TIMESTAMPTZ; v_month TEXT;
BEGIN
 -- SQL NULL must never turn a current-authority predicate into UNKNOWN/false IF.
 -- Refuse malformed occurrence/node data before taking any mutation fence.
 IF w IS NULL OR e IS NULL OR r IS NULL OR b IS NULL OR n IS NULL OR nr IS NULL
 OR NOT game_node_is_uuid_v7(w) OR NOT game_node_is_uuid_v7(n) OR octet_length(b)<>32
 OR e<0 OR e>=18446744073709551615 OR trunc(e)<>e
 OR r<1 OR r>=18446744073709551615 OR trunc(r)<>r
 OR nr<1 OR nr>18446744073709551615 OR trunc(nr)<>nr
 THEN RAISE EXCEPTION 'World reset occurrence/node data refused' USING ERRCODE='OTR03'; END IF;
 -- Drain digest and House evidence are bounded, one-dimensional and NULL-free data.
 IF d IS NULL OR octet_length(d)<>32 OR hk IS NULL OR hp IS NULL OR hs IS NULL
 OR coalesce(array_ndims(hk),1)<>1 OR coalesce(array_ndims(hp),1)<>1 OR coalesce(array_ndims(hs),1)<>1
 OR cardinality(hk)<>cardinality(hp) OR cardinality(hk)>4194304 OR cardinality(hs)>1024
 OR EXISTS(SELECT 1 FROM unnest(hk) x WHERE x IS NULL) OR EXISTS(SELECT 1 FROM unnest(hp) x WHERE x IS NULL)
 OR EXISTS(SELECT 1 FROM unnest(hs) x WHERE x IS NULL)
 THEN RAISE EXCEPTION 'World reset drain/House evidence data refused' USING ERRCODE='OTR03'; END IF;
 -- Independently acquire the exact native admission relation fence, in its lexical
 -- order, even on direct runtime SQL invocation. These durable locks are not I/C proof.
 LOCK TABLE game_durability_admission_account_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_character_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_guard_history IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_lifecycle_receipts IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_runtime_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_signing_trust_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_control_loss_continuity IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_executor_custody IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_fresh_admission_receipts IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_reconnect_attempts IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_reconnect_pending_commands IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_reconnect_sessions IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_recovery_grant_consumptions IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_session_replacements IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_session_use_ledgers IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_session_use_memberships IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_transport_ref_reservations IN EXCLUSIVE MODE;
 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=w AND reset_epoch=e FOR UPDATE;
 IF NOT FOUND OR j.revision<>r OR j.target_digest IS DISTINCT FROM b OR j.phase<>'RETIRING'
 OR e>=18446744073709551615 OR r>=18446744073709551615 OR j.target_map_revision IS NULL
 THEN RAISE EXCEPTION 'World reset occurrence/target stale or unqualified' USING ERRCODE='OTR03'; END IF;
 LOCK TABLE game_runtime_scope_assignments IN SHARE MODE;
 SELECT reset_epoch INTO a FROM game_world_reset_active_epochs WHERE world_id=w FOR UPDATE;
 IF NOT FOUND OR a<>e OR NOT EXISTS(SELECT 1 FROM game_world_reset_epoch_history h
 WHERE h.world_id=w AND h.reset_epoch=e AND h.map_revision=j.active_map_revision)
 OR NOT game_node_lock_current_registration(n,nr)
 THEN RAISE EXCEPTION 'World reset active epoch/incarnation refused' USING ERRCODE='OTR03'; END IF;
 SELECT * INTO v_history FROM game_world_reset_epoch_history WHERE world_id=w AND reset_epoch=e;
 IF NOT FOUND THEN RAISE EXCEPTION 'World reset predecessor history absent' USING ERRCODE='OTR03'; END IF;
 LOCK TABLE game_item_ground_locations,game_item_corpse_container_entries IN SHARE MODE;
 LOCK TABLE game_item_house_interior_locations,game_house_ownership,game_world_house_instances IN SHARE MODE;
 -- Re-read durable prerequisites only after all cohort, admission, Ground and House fences.
 IF NOT game_world_reset_cohort_current(w,e,n,nr)
 OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE world_id=w)
 OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries ce JOIN game_item_ground_locations g
 ON g.item_instance_id=ce.parent_item_instance_id WHERE g.world_id=w)
 THEN RAISE EXCEPTION 'World reset current durable prerequisites refused' USING ERRCODE='OTR03'; END IF;
 -- D1233 drain binding, rechecked here under the SHARE assignment lock: every World Channel
 -- is held by this exact incarnation and the borrowed proof's digest equals the digest of
 -- the independently read cohort (domain, World, node, revision, count, sorted Channels).
 IF EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1
 AND (state<>1 OR holder_node_id IS DISTINCT FROM n OR holder_registration_revision IS DISTINCT FROM nr))
 OR d IS DISTINCT FROM (SELECT sha256(convert_to('OTERYN_WORLD_RESET_DRAIN_BINDING','UTF8')||decode('0001','hex')
 ||uuid_send(w)||uuid_send(n)||game_world_reset_u64_be(nr)||game_world_reset_u64_be(count(*))
 ||coalesce(string_agg(uuid_send(channel_id)||game_world_reset_u64_be(ownership_generation),''::bytea
 ORDER BY uuid_send(channel_id)),''::bytea))
 FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1)
 THEN RAISE EXCEPTION 'World reset drain binding refused' USING ERRCODE='OTR03'; END IF;
 -- House recheck under the SHARE House locks: every live interior cell is a target catalogue
 -- cell and every owned/Instance House is one whose Active and Target definitions agree.
 IF EXISTS(SELECT house_key,spatial_position FROM game_item_house_interior_locations WHERE world_id=w
 EXCEPT SELECT k,p FROM unnest(hk,hp) AS c(k,p))
 OR EXISTS(SELECT house_key FROM game_house_ownership WHERE world_id=w
 UNION SELECT house_key FROM game_world_house_instances WHERE world_id=w
 EXCEPT SELECT k FROM unnest(hs) AS s(k))
 THEN RAISE EXCEPTION 'World reset House evidence refused' USING ERRCODE='OTR03'; END IF;
 FOR v_channel IN SELECT channel_id FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1 ORDER BY uuid_send(channel_id) LOOP
  -- Native sequence is per Channel and is NOT the World epoch. Serialize with op4.
  PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:content-activation:'||w::text||':'||v_channel.channel_id::text,0));
  SELECT * INTO v_current FROM game_content_activations WHERE world_id=w AND channel_id=v_channel.channel_id ORDER BY activation_sequence DESC LIMIT 1;
  IF NOT FOUND OR (v_current.server_artifact_digest,v_current.client_artifact_digest,v_current.frame_binding_digest)
   IS DISTINCT FROM (v_history.server_digest,v_history.client_digest,v_history.frame_digest)
  THEN RAISE EXCEPTION 'World reset current Channel activation disagrees with predecessor history' USING ERRCODE='OTR03'; END IF;
  v_previous:=v_current.activation_sequence;
  v_sequence:=v_previous+1;
  IF v_sequence>18446744073709551615 THEN RAISE EXCEPTION 'Content issuance sequence exhausted' USING ERRCODE='OTR03'; END IF;
  PERFORM game_content_record_activation_core(w,v_channel.channel_id,v_sequence,v_previous,j.target_server_digest,j.target_client_digest,j.target_frame_digest);
 END LOOP;
 v_instant:=clock_timestamp();
 v_month:=game_world_reset_month_key(v_instant,j.target_calendar_policy,j.target_calendar_revision);
 IF v_month IS NULL THEN RAISE EXCEPTION 'World reset calendar refused' USING ERRCODE='OTR03'; END IF;
 INSERT INTO game_world_reset_epoch_history(world_id,reset_epoch,map_revision,server_digest,client_digest,frame_digest,calendar_policy,calendar_revision,calendar_parity,month_key,recorded_at,issued_by)
 VALUES(w,e+1,j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest,j.target_calendar_policy,j.target_calendar_revision,'PARITY_PENDING',v_month,v_instant,session_user);
 UPDATE game_world_reset_active_epochs SET reset_epoch=e+1 WHERE world_id=w AND reset_epoch=e;
 IF NOT FOUND THEN RAISE EXCEPTION 'World reset active CAS refused' USING ERRCODE='OTR03'; END IF;
 UPDATE game_world_reset_jobs SET phase='ACTIVATED',revision=r+1 WHERE world_id=w AND reset_epoch=e AND phase='RETIRING' AND revision=r;
 IF NOT FOUND THEN RAISE EXCEPTION 'World reset job CAS refused' USING ERRCODE='OTR03'; END IF;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_activate(UUID,NUMERIC,NUMERIC,BYTEA,UUID,NUMERIC,BYTEA,TEXT[],BYTEA[],TEXT[]) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_world_reset_activate(UUID,NUMERIC,NUMERIC,BYTEA,UUID,NUMERIC,BYTEA,TEXT[],BYTEA[],TEXT[]) TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_activate(uuid,numeric,numeric,bytea,uuid,numeric,bytea,text[],bytea[],text[]) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

CREATE OR REPLACE FUNCTION game_item_decay_retire_consistency_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    item game_item_instances%ROWTYPE;
    corpse_materialized BIGINT;
    places INTEGER;
BEGIN
    IF game_world_reset_item_proven(NEW.item_instance_id) THEN RETURN NULL; END IF;
    SELECT r.materialized_at INTO corpse_materialized
      FROM game_item_mint_receipts r
     WHERE r.item_instance_id = NEW.corpse_item_instance_id
       AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
       AND r.draw_ordinal = 0
       FOR UPDATE;
    IF NOT FOUND OR corpse_materialized IS NULL
       OR NEW.deadline <> corpse_materialized + 60000
       OR floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint < NEW.deadline THEN
        RAISE EXCEPTION 'DECAY_RETIRE requires a materialized corpse past its materialized_at + 60 s deadline'
            USING ERRCODE = '23514';
    END IF;
    SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_corpse_container_entries ce
             WHERE ce.item_instance_id = NEW.item_instance_id)
      INTO places;
    IF item.item_instance_id IS NULL
       OR item.lifecycle <> 2 OR item.quantity <> 0
       OR item.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR item.world_id <> NEW.world_id
       OR places <> 0
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.quantity_before)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_decay_retire_reservations v
            WHERE (v.item_instance_id, v.fence_scope_ownership_generation,
                   v.corpse_item_instance_id, v.world_id, v.channel_id, v.transaction_id,
                   v.event_id, v.quantity_before, v.deadline, v.occurred_at)
                = (NEW.item_instance_id, NEW.fence_scope_ownership_generation,
                   NEW.corpse_item_instance_id, NEW.world_id, NEW.channel_id,
                   NEW.transaction_id, NEW.event_id, NEW.quantity_before, NEW.deadline,
                   NEW.occurred_at)
              AND v.placement_ordinal IS NOT DISTINCT FROM NEW.placement_ordinal
              AND sha256(v.envelope) = NEW.envelope_sha256)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The corpse's own step: its own Ground removed in this transaction
       -- (the evidence is unique per item and only a proven removal writes
       -- it), and no live entry left under it.
       OR (NEW.item_instance_id = NEW.corpse_item_instance_id AND (
               NOT EXISTS (
                   SELECT 1 FROM game_item_ground_removal_evidence gre
                    WHERE gre.item_instance_id = NEW.item_instance_id
                      AND gre.world_id = NEW.world_id
                      AND gre.channel_id = NEW.channel_id)
               OR EXISTS (
                   SELECT 1 FROM game_item_corpse_container_entries ce
                    WHERE ce.parent_item_instance_id = NEW.corpse_item_instance_id)))
       -- An entry's step: that entry of this corpse removed in this physical
       -- transaction, while the corpse was live on Ground in this scope.
       OR (NEW.item_instance_id <> NEW.corpse_item_instance_id AND NOT EXISTS (
               SELECT 1 FROM game_item_corpse_entry_removal_evidence cre
                WHERE cre.item_instance_id = NEW.item_instance_id
                  AND cre.parent_item_instance_id = NEW.corpse_item_instance_id
                  AND cre.placement_ordinal = NEW.placement_ordinal
                  AND cre.world_id = NEW.world_id
                  AND cre.channel_id = NEW.channel_id
                  AND cre.created_xact_id = pg_current_xact_id())) THEN
        RAISE EXCEPTION 'DECAY_RETIRE must commit its reservation, audit event, removal and retirement together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_item_decay_retire_consistency_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;
ALTER FUNCTION game_item_decay_retire_consistency_guard() SECURITY DEFINER;

CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF game_world_reset_item_proven(NEW.item_instance_id) THEN RETURN NULL; END IF;
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.last_transaction_id
     AND p.item_instance_id=NEW.item_instance_id AND p.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_house_shape_complete(NEW.last_transaction_id);RETURN NULL;END IF;

    -- 0082: the BURN line of an NPC trade of this same physical transaction.
    IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
                                AND t.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    -- 0083: the used stack (and the flask stack it grows) of an item-use record of this same
    -- physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u
                WHERE u.transaction_id = NEW.last_transaction_id
                  AND u.created_xact_id = pg_current_xact_id()
                  AND (u.source_item_instance_id = NEW.item_instance_id
                       OR (u.shape = 3 AND u.flask_item_instance_id = NEW.item_instance_id))) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       WHERE r.operation=1 AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.last_transaction_id AND r.state_revision_before=OLD.state_revision
         AND NEW.state_revision=OLD.state_revision+1 AND NEW.quantity=OLD.quantity
         AND NEW.lifecycle=OLD.lifecycle AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=NEW.item_instance_id AND e.transaction_id=NEW.last_transaction_id
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind<>1
         AND l.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
         WHERE r.transaction_id = NEW.last_transaction_id
           AND (r.source_item_instance_id = NEW.item_instance_id
                OR r.receiver_item_instance_id = NEW.item_instance_id)
           -- Repair generation 2 (finding 2's root cause, applied here too):
           -- the matched receipt must be this SAME physical transaction's
           -- own receipt, never a historical one whose logical TransactionId
           -- is replayed from a different, already-committed transaction.
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
         WHERE d.transaction_id = NEW.last_transaction_id
           AND d.item_instance_id = NEW.item_instance_id
           AND d.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          -- The line's fee record is this same physical transaction's own record, never a
          -- committed one a later line is appended to.
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
                                    AND f.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
         WHERE w.transaction_id = NEW.last_transaction_id
           AND w.item_instance_id = NEW.item_instance_id
           AND w.cause IN (2, 3)
           AND w.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
                                             AND o.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.direction = 1
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;
ALTER FUNCTION game_item_instance_change_proven() SECURITY DEFINER;

CREATE OR REPLACE FUNCTION game_item_ground_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF game_world_reset_item_proven(OLD.item_instance_id) THEN RETURN NULL; END IF;
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=1
        AND r.source_stack_ordinal=OLD.stack_ordinal AND r.world_id=OLD.world_id AND r.channel_id=OLD.channel_id AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=1
        AND r.source_stack_ordinal=OLD.stack_ordinal AND r.world_id=OLD.world_id AND r.channel_id=OLD.channel_id AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=OLD.item_instance_id AND e.source_stack_ordinal=OLD.stack_ordinal AND (e.world_id,e.channel_id)=(OLD.world_id,OLD.channel_id)
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
       WHERE l.item_instance_id=OLD.item_instance_id AND l.operation_kind IN(2,3)
         AND (l.world_id,l.channel_id,l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
           =(OLD.world_id,OLD.channel_id,OLD.spatial_position,OLD.map_revision,OLD.content_revision,OLD.native_room_placement_context,OLD.stack_ordinal)
         AND i.last_transaction_id=l.transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
         WHERE m.item_instance_id = OLD.item_instance_id
           AND m.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'a corpse Ground location is never removed by a TRANSFER'
            USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Ground removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_item_ground_removal_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;

CREATE OR REPLACE FUNCTION game_item_corpse_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF game_world_reset_item_proven(OLD.item_instance_id) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND l.operation_kind=6 AND l.source_entry_is_corpse
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.parent_item_instance_id
           AND d.item_instance_id <> d.corpse_item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'corpse container entry removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_item_corpse_entry_removal_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;
