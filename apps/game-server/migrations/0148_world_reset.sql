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
-- The census reads one World's bounded cohort and nonterminal session pages.
CREATE INDEX game_world_reset_channel_census ON game_runtime_scope_assignments(world_id,channel_id) WHERE scope_kind=1;
CREATE INDEX game_world_reset_session_census ON game_durability_reconnect_sessions(world_id,game_session_id) WHERE session_state<>3;
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
-- Assignment replacement uses INSERT ... ON CONFLICT, so BEFORE INSERT must
-- distinguish an existing Channel scope from an expansion of the cohort.
CREATE FUNCTION game_world_reset_cohort_closed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 IF NEW.scope_kind=1 AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE scope_key=NEW.scope_key)
  AND EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=NEW.world_id AND phase='RETIRING') THEN
  RAISE EXCEPTION 'World reset Channel cohort is closed' USING ERRCODE='OTR03';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_world_reset_cohort_closed BEFORE INSERT ON game_runtime_scope_assignments
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_cohort_closed();
DO $$
DECLARE f TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_world_reset_admission_closed','game_world_reset_attempt_closed','game_world_reset_cohort_closed'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',f);
 END LOOP;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_loss_binding(UUID,NUMERIC,BYTEA,BYTEA) SET search_path=%I,pg_temp',current_schema());
END $$;


ALTER TABLE game_item_ground_removal_evidence ADD COLUMN created_xact_id xid8;
CREATE FUNCTION game_world_reset_ground_removal_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_world_reset_ground_removal_stamp BEFORE INSERT OR UPDATE ON game_item_ground_removal_evidence
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_ground_removal_stamp();
REVOKE ALL ON FUNCTION game_world_reset_ground_removal_stamp() FROM PUBLIC;

-- Both admitted causes retain the existing per-item uniqueness. Corpse-only
-- evidence is absent for reset; the Ground root is separately named.
DO $$
DECLARE t TEXT; c RECORD; n INTEGER;
BEGIN
 FOREACH t IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
  EXECUTE format('ALTER TABLE %I ADD COLUMN cause_kind TEXT NOT NULL DEFAULT ''CORPSE_DECAY'', ADD COLUMN reset_epoch NUMERIC(20,0), ADD COLUMN retirement_binding BYTEA, ADD COLUMN target_digest BYTEA, ADD COLUMN ground_root_item_instance_id UUID REFERENCES game_item_instances(item_instance_id)',t);
  EXECUTE format('ALTER TABLE %I ALTER COLUMN corpse_item_instance_id DROP NOT NULL, ALTER COLUMN deadline DROP NOT NULL',t);
  n:=0;
  FOR c IN SELECT conname FROM pg_constraint WHERE conrelid=to_regclass(t) AND contype='c' AND pg_get_constraintdef(oid) ~ '\m(deadline|corpse_item_instance_id)\M' LOOP
   EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',t,c.conname); n:=n+1;
  END LOOP;
  IF n<>3 THEN RAISE EXCEPTION 'unexpected retirement constraint shape on %',t; END IF;
  EXECUTE format('ALTER TABLE %I ADD CONSTRAINT game_reset_retire_shape CHECK (
   occurred_at>0 AND (
    (cause_kind=''CORPSE_DECAY'' AND corpse_item_instance_id IS NOT NULL AND deadline IS NOT NULL AND deadline>=60000 AND occurred_at>=deadline
     AND (item_instance_id=corpse_item_instance_id)=(placement_ordinal IS NULL)
     AND reset_epoch IS NULL AND retirement_binding IS NULL AND target_digest IS NULL AND ground_root_item_instance_id IS NULL)
    OR
    (cause_kind=''WORLD_RESET'' AND corpse_item_instance_id IS NULL AND deadline IS NULL
     AND reset_epoch BETWEEN 0 AND 18446744073709551614 AND reset_epoch IS NOT NULL
     AND octet_length(retirement_binding)=32 AND retirement_binding IS NOT NULL
     AND octet_length(target_digest)=32 AND target_digest IS NOT NULL
     AND ground_root_item_instance_id IS NOT NULL
     AND (item_instance_id=ground_root_item_instance_id)=(placement_ordinal IS NULL)
     AND retirement_binding=sha256(convert_to(''oteryn:world-reset-retirement:v1'',''UTF8'') || target_digest))))',t);
 END LOOP;
END $$;

-- The caller has locked admission relations and the reset record. This SQL
-- recheck covers the complete cohort; Rust additionally decodes every typed
-- Runtime guard through its existing historical/mirror verifier.
CREATE FUNCTION game_world_reset_drained(p_world UUID,p_epoch NUMERIC) RETURNS BOOLEAN
LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j JOIN game_world_reset_active a USING(world_id,reset_epoch)
               WHERE j.world_id=p_world AND j.reset_epoch=p_epoch AND j.phase='RETIRING')
 AND (SELECT count(*) FROM game_world_reset_channels WHERE world_id=p_world AND reset_epoch=p_epoch) BETWEEN 1 AND 128
 AND NOT EXISTS(SELECT 1 FROM (SELECT * FROM game_runtime_scope_assignments WHERE scope_kind=1 AND world_id=p_world) a
  FULL JOIN (SELECT * FROM game_world_reset_channels WHERE world_id=p_world AND reset_epoch=p_epoch) c
   ON a.world_id=c.world_id AND a.channel_id=c.channel_id
  LEFT JOIN game_durability_admission_runtime_guards g ON g.scope_key=a.scope_key
  WHERE (a.world_id=p_world OR c.world_id=p_world) AND
   (a.scope_key IS NULL OR c.channel_id IS NULL OR a.state<>1 OR a.ownership_generation<=c.predecessor_generation
    OR g.scope_key IS NULL OR g.ready OR g.ownership_generation<>a.ownership_generation
    OR NOT game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision)))
 AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE world_id=p_world AND session_state<>3)
$$;

-- A same-transaction reset receipt proves exactly one conserved retirement,
-- its removed Ground location, frozen reservation and immutable audit outbox.
CREATE FUNCTION game_world_reset_item_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_decay_retire_receipts%ROWTYPE; j game_world_reset_jobs%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_decay_retire_receipts
  WHERE item_instance_id=p_item AND cause_kind='WORLD_RESET' AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=r.world_id AND reset_epoch=r.reset_epoch FOR UPDATE;
 IF NOT FOUND OR j.phase<>'RETIRING' OR j.target_digest<>r.target_digest OR j.retirement_binding<>r.retirement_binding
  OR NOT game_world_reset_drained(r.world_id,r.reset_epoch)
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p_item AND i.world_id=r.world_id
   AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p_item)
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence ev WHERE ev.item_instance_id=p_item
   AND ev.transaction_id=r.transaction_id AND ev.quantity_before=r.quantity_before)
  OR NOT EXISTS(SELECT 1 FROM game_item_decay_retire_reservations v
   JOIN game_runtime_scope_assignments a ON a.world_id=v.world_id AND a.channel_id=v.channel_id
   WHERE (v.item_instance_id,v.fence_scope_ownership_generation,v.world_id,v.channel_id,v.transaction_id,v.event_id,v.quantity_before,v.occurred_at,v.cause_kind,v.reset_epoch,v.retirement_binding,v.target_digest,v.ground_root_item_instance_id)
       =(r.item_instance_id,r.fence_scope_ownership_generation,r.world_id,r.channel_id,r.transaction_id,r.event_id,r.quantity_before,r.occurred_at,r.cause_kind,r.reset_epoch,r.retirement_binding,r.target_digest,r.ground_root_item_instance_id)
    AND v.placement_ordinal IS NOT DISTINCT FROM r.placement_ordinal AND sha256(v.envelope)=r.envelope_sha256
    AND a.ownership_generation=v.fence_scope_ownership_generation AND a.holder_node_id=v.fence_holder_node_id
    AND a.holder_registration_revision=v.fence_holder_registration_revision)
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.item_instance_id=p_item AND o.event_id=r.event_id
   AND o.transaction_id=r.transaction_id AND o.occurred_at=r.occurred_at AND o.envelope_sha256=r.envelope_sha256
   AND o.created_xact_id=pg_current_xact_id() AND o.publication_state=1 AND o.published_at IS NULL)
  OR (r.placement_ordinal IS NULL AND (
   NOT EXISTS(SELECT 1 FROM game_item_ground_removal_evidence e WHERE e.item_instance_id=p_item AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.created_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p_item)))
  OR (r.placement_ordinal IS NOT NULL AND (
   NOT EXISTS(SELECT 1 FROM game_item_corpse_entry_removal_evidence e WHERE e.item_instance_id=p_item
    AND e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal=r.placement_ordinal
    AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.created_xact_id=pg_current_xact_id())
   OR NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
    WHERE g.item_instance_id=r.ground_root_item_instance_id AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND i.lifecycle=1)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal<r.placement_ordinal))) THEN
  RAISE EXCEPTION 'World reset requires current drain, conserved item, removal, reservation and audit in one transaction' USING ERRCODE='23514';
 END IF;
 RETURN TRUE;
END $$;

-- Extend the installed latest bodies, preserving every merged arm rather than
-- copying a stale guard from an earlier migration. CREATE OR REPLACE retains
-- the definer attribute from pg_get_functiondef; pin it explicitly as D1416 asks.
DO $$
DECLARE f TEXT; body TEXT; item_ref TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_item_decay_retire_consistency_guard','game_item_instance_change_proven','game_item_ground_removal_proven','game_item_corpse_entry_removal_proven'] LOOP
  SELECT pg_get_functiondef(to_regprocedure(f||'()')) INTO STRICT body;
  item_ref:=CASE WHEN f IN('game_item_ground_removal_proven','game_item_corpse_entry_removal_proven') THEN 'OLD.item_instance_id' ELSE 'NEW.item_instance_id' END;
  body:=regexp_replace(body,'\mBEGIN\M','BEGIN IF game_world_reset_item_proven('||item_ref||') THEN RETURN NULL; END IF;');
  EXECUTE body;
  EXECUTE format('ALTER FUNCTION %I() SECURITY DEFINER SET search_path=%I,pg_temp',f,current_schema());
 END LOOP;
END $$;

-- Revision is derived solely from committed reset receipts, never a runtime
-- caller's direct job UPDATE (runtime and Control retain SELECT only).
CREATE FUNCTION game_world_reset_receipt_revision() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.cause_kind='WORLD_RESET' THEN
  UPDATE game_world_reset_jobs SET revision=revision+1 WHERE world_id=NEW.world_id AND reset_epoch=NEW.reset_epoch AND phase='RETIRING';
  IF NOT FOUND THEN RAISE EXCEPTION 'reset receipt has no RETIRING job' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_world_reset_receipt_revision AFTER INSERT ON game_item_decay_retire_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_receipt_revision();
DO $$
DECLARE f TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_world_reset_drained(uuid,numeric)','game_world_reset_item_proven(uuid)','game_world_reset_receipt_revision()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;

-- Runtime has SELECT only on reset state. Row locking stays in this narrow
-- read-only definer; no UPDATE grant is needed to fence a source/map receipt.
CREATE FUNCTION game_world_reset_map_epoch_current(p_world UUID,p_epoch NUMERIC) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 PERFORM 1 FROM game_world_reset_active WHERE world_id=p_world FOR SHARE;
 RETURN p_epoch=coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=p_world),0)
  AND NOT EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=p_world AND phase='RETIRING');
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_map_epoch_current(UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_world_reset_map_epoch_current(UUID,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_map_epoch_current(UUID,NUMERIC) TO oteryn_game_runtime,oteryn_game_control;

-- Epoch-zero intent bytes stay unchanged. Later epochs bind their identity into
-- both the source receipt and the audit, so a same-digest reset remints once.
ALTER TABLE game_native_map_item_receipts ADD COLUMN reset_epoch NUMERIC(20,0) NOT NULL DEFAULT 0
 CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615);
DO $$
DECLARE c RECORD; n INTEGER:=0; body TEXT;
BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_native_map_item_receipts'::regclass
  AND pg_get_constraintdef(oid)='UNIQUE (world_id, channel_id, content_digest, placement_key)' LOOP
  EXECUTE format('ALTER TABLE game_native_map_item_receipts DROP CONSTRAINT %I',c.conname); n:=n+1;
 END LOOP;
 IF n<>1 THEN RAISE EXCEPTION 'unknown source receipt uniqueness'; END IF;
 SELECT pg_get_functiondef('game_native_map_item_stamp()'::regprocedure) INTO body;
 body:=replace(body,'IF j IS DISTINCT FROM jsonb_build_object','IF (j - ''reset_epoch'') IS DISTINCT FROM jsonb_build_object');
 body:=replace(body,'j:=convert_from(NEW.source_intent,''UTF8'')::jsonb;',
 'j:=convert_from(NEW.source_intent,''UTF8'')::jsonb;
  IF NOT game_world_reset_map_epoch_current(NEW.world_id,NEW.reset_epoch)
   OR (NEW.reset_epoch=0 AND j ? ''reset_epoch'')
   OR (NEW.reset_epoch>0 AND j->>''reset_epoch'' IS DISTINCT FROM NEW.reset_epoch::text) THEN
   RAISE EXCEPTION ''source receipt outside active reset epoch'' USING ERRCODE=''23514'';
  END IF;');
 IF body NOT LIKE '%source receipt outside active reset epoch%' OR body NOT LIKE '%(j - ''reset_epoch'')%' THEN
  RAISE EXCEPTION 'unknown source intent stamp shape';
 END IF;
 EXECUTE body;
END $$;
ALTER TABLE game_native_map_item_receipts ADD UNIQUE(world_id,channel_id,reset_epoch,content_digest,placement_key);

-- Historical receipts remain replayable; a fresh value-moving map MINT must
-- name the active epoch and cannot straddle RETIRING publication.
CREATE FUNCTION game_world_reset_map_mint_epoch_check() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT game_world_reset_map_epoch_current(NEW.world_id,NEW.reset_epoch) THEN
  RAISE EXCEPTION 'map MINT outside active reset epoch' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_map_mint_epoch_check() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_world_reset_map_mint_epoch_check() FROM PUBLIC;
CREATE TRIGGER game_world_reset_map_mint_epoch_check BEFORE INSERT ON game_map_item_mint_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_map_mint_epoch_check();


-- Caller-transaction issuance core: move the installed CAS/replay owner, removing
-- only its external grant check. Neither runtime nor control can call this core.
DO $$
DECLARE body TEXT; original TEXT;
BEGIN
 SELECT pg_get_functiondef('game_content_record_activation(uuid,uuid,numeric,numeric,bytea,bytea,bytea)'::regprocedure) INTO original;
 body:=replace(original,'game_content_record_activation(', 'game_content_record_activation_core(');
 body:=regexp_replace(body,'IF NOT EXISTS \(.*?END IF;','', 's');
 IF body=original OR body LIKE '%content activation is not granted%' OR body NOT LIKE '%pg_advisory_xact_lock%' THEN
  RAISE EXCEPTION 'unknown installed content issuance body';
 END IF;
 EXECUTE body;
END $$;
REVOKE ALL ON FUNCTION game_content_record_activation_core(UUID,UUID,NUMERIC,NUMERIC,BYTEA,BYTEA,BYTEA) FROM PUBLIC;
CREATE OR REPLACE FUNCTION game_content_record_activation(
 p_world_id UUID,p_channel_id UUID,p_sequence NUMERIC,p_previous NUMERIC,p_server BYTEA,p_client BYTEA,p_frame BYTEA)
 RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 IF NOT EXISTS(SELECT 1 FROM game_control_scope_grants WHERE control_role=session_user
  AND world_id=p_world_id AND channel_id=p_channel_id AND operation=4) THEN
  RAISE EXCEPTION 'content activation is not granted to this control role' USING ERRCODE='42501';
 END IF;
 IF EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=p_world_id AND phase='RETIRING') THEN
  RAISE EXCEPTION 'World reset content activation closed' USING ERRCODE='OTR03';
 END IF;
 -- The existing CAS owner still checks exact historical replay. Only a fresh
 -- issuance can change the active content and therefore requires a reset.
 IF NOT EXISTS(SELECT 1 FROM game_content_activations WHERE world_id=p_world_id AND channel_id=p_channel_id AND activation_sequence=p_sequence)
  AND EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
  WHERE a.world_id=p_world_id AND (e.server_digest,e.client_digest,e.frame_digest) IS DISTINCT FROM (p_server,p_client,p_frame)) THEN
  RAISE EXCEPTION 'content cutover requires a World reset' USING ERRCODE='OTC01';
 END IF;
 PERFORM game_content_record_activation_core(p_world_id,p_channel_id,p_sequence,p_previous,p_server,p_client,p_frame);
END $$;

-- The same lexical admission relation fence used by the owning Rust writer.
CREATE FUNCTION game_world_reset_lock_admission() RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE relation TEXT;
BEGIN
 FOREACH relation IN ARRAY ARRAY[
  'game_durability_admission_account_guards','game_durability_admission_character_guards',
  'game_durability_admission_guard_history','game_durability_admission_lifecycle_receipts',
  'game_durability_admission_runtime_guards','game_durability_admission_signing_trust_guards',
  'game_durability_control_loss_continuity','game_durability_executor_custody',
  'game_durability_fresh_admission_receipts','game_durability_reconnect_attempts',
  'game_durability_reconnect_pending_commands','game_durability_reconnect_sessions',
  'game_durability_recovery_grant_consumptions','game_durability_session_replacements',
  'game_durability_session_use_ledgers','game_durability_session_use_memberships',
  'game_durability_transport_ref_reservations'] LOOP
  EXECUTE format('LOCK TABLE %I IN EXCLUSIVE MODE',relation);
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_lock_admission() FROM PUBLIC;

-- Explicit owner-only Control initializer. No installation seed or request time.
-- Publication/operator capabilities are deliberately not installed in this slice.
CREATE FUNCTION game_world_reset_initialize(p_world UUID,p_target BYTEA,p_map TEXT,
 p_server BYTEA,p_client BYTEA,p_frame BYTEA,p_calendar TEXT,p_revision NUMERIC)
 RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE e game_world_reset_epochs%ROWTYPE; instant TIMESTAMPTZ; n INTEGER;
BEGIN
 PERFORM game_world_reset_lock_admission();
 LOCK TABLE game_world_reset_jobs IN SHARE ROW EXCLUSIVE MODE;
 SELECT * INTO e FROM game_world_reset_epochs WHERE world_id=p_world AND reset_epoch=0;
 IF FOUND THEN
  IF (e.target_digest,e.map_revision,e.server_digest,e.client_digest,e.frame_digest,e.calendar,e.calendar_revision)
     IS DISTINCT FROM (p_target,p_map,p_server,p_client,p_frame,p_calendar,p_revision) THEN
   RAISE EXCEPTION 'conflicting epoch zero initialization' USING ERRCODE='23514';
  END IF;
  RETURN FALSE;
 END IF;
 IF p_calendar IS DISTINCT FROM 'TIBIA_SERVER_SAVE_BERLIN_10' OR p_revision IS NULL OR p_revision<1
  OR p_revision>18446744073709551615 OR p_revision<>trunc(p_revision) THEN
  RAISE EXCEPTION 'unsupported reset calendar' USING ERRCODE='23514';
 END IF;
 SELECT count(*) INTO n FROM (SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=p_world AND scope_kind=1 LIMIT 129) a;
 IF n<1 OR n>128 OR EXISTS(SELECT 1 FROM game_runtime_scope_assignments a LEFT JOIN LATERAL
   (SELECT * FROM game_content_activations c WHERE c.world_id=a.world_id AND c.channel_id=a.channel_id ORDER BY activation_sequence DESC LIMIT 1) c ON TRUE
   WHERE a.world_id=p_world AND a.scope_kind=1 AND (c.server_artifact_digest,c.client_artifact_digest,c.frame_binding_digest)
    IS DISTINCT FROM (p_server,p_client,p_frame)) THEN
  RAISE EXCEPTION 'initial content is not issued for every Channel' USING ERRCODE='23514';
 END IF;
 instant:=clock_timestamp();
 INSERT INTO game_world_reset_epochs VALUES(p_world,0,p_target,p_map,p_server,p_client,p_frame,p_calendar,p_revision,
  instant,to_char((instant AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM'));
 INSERT INTO game_world_reset_active VALUES(p_world,0);
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_initialize(UUID,BYTEA,TEXT,BYTEA,BYTEA,BYTEA,TEXT,NUMERIC) FROM PUBLIC;

-- Owner-only publication seam, with no installed runtime/control EXECUTE grant.
-- HOUSE-1a is absent: any target House or existing House custody refuses.
CREATE FUNCTION game_world_reset_publish(p_world UUID,p_epoch NUMERIC,p_target BYTEA,p_map TEXT,
 p_server BYTEA,p_client BYTEA,p_frame BYTEA,p_house_tiles JSONB,p_revision NUMERIC)
 RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE e game_world_reset_epochs%ROWTYPE; j game_world_reset_jobs%ROWTYPE; n INTEGER;
BEGIN
 PERFORM game_world_reset_lock_admission();
 LOCK TABLE game_world_reset_jobs IN SHARE ROW EXCLUSIVE MODE;
 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=p_world AND reset_epoch=p_epoch FOR UPDATE;
 IF FOUND THEN
  IF (j.target_digest,j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest,j.target_house_tiles,j.calendar_revision)
     IS DISTINCT FROM (p_target,p_map,p_server,p_client,p_frame,p_house_tiles,p_revision) THEN
   RAISE EXCEPTION 'conflicting World reset publication' USING ERRCODE='23514';
  END IF;
  RETURN FALSE;
 END IF;
 SELECT h.* INTO e FROM game_world_reset_active a JOIN game_world_reset_epochs h USING(world_id,reset_epoch)
  WHERE a.world_id=p_world AND a.reset_epoch=p_epoch FOR UPDATE OF a;
 IF NOT FOUND OR p_epoch>=18446744073709551615 THEN
  RAISE EXCEPTION 'reset epoch is not current' USING ERRCODE='23514';
 END IF;
 LOCK TABLE game_item_house_interior_locations IN SHARE MODE;
 IF p_target IS DISTINCT FROM p_server OR p_revision IS NULL OR p_revision<1
  OR p_revision>18446744073709551615 OR p_revision<>trunc(p_revision) THEN
  RAISE EXCEPTION 'invalid World reset target/calendar' USING ERRCODE='23514';
 END IF;
 IF p_house_tiles IS DISTINCT FROM '[]'::jsonb OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=p_world)
  OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=p_world) THEN
  RAISE EXCEPTION 'World reset House qualification unavailable' USING ERRCODE='23514';
 END IF;
 LOCK TABLE game_runtime_scope_assignments IN SHARE MODE;
 SELECT count(*) INTO n FROM (SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=p_world AND scope_kind=1 LIMIT 129) a;
 IF n<1 OR n>128 OR EXISTS(SELECT 1 FROM game_runtime_scope_assignments a LEFT JOIN LATERAL
  (SELECT * FROM game_content_activations c WHERE c.world_id=a.world_id AND c.channel_id=a.channel_id ORDER BY activation_sequence DESC LIMIT 1) c ON TRUE
  WHERE a.world_id=p_world AND a.scope_kind=1 AND (c.server_artifact_digest,c.client_artifact_digest,c.frame_binding_digest)
   IS DISTINCT FROM (e.server_digest,e.client_digest,e.frame_digest)) THEN
  RAISE EXCEPTION 'reset predecessor content/cohort mismatch' USING ERRCODE='23514';
 END IF;
 INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,
  target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase)
 VALUES(p_world,p_epoch,p_target,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8')||p_target),
  p_map,p_server,p_client,p_frame,p_house_tiles,p_revision,'RETIRING');
 INSERT INTO game_world_reset_channels SELECT p_world,p_epoch,channel_id,ownership_generation
  FROM game_runtime_scope_assignments WHERE world_id=p_world AND scope_kind=1;
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_publish(UUID,NUMERIC,BYTEA,TEXT,BYTEA,BYTEA,BYTEA,JSONB,NUMERIC) FROM PUBLIC;

-- Final commit: lock record, House relation, assignments and Ground; recheck
-- drain and target; advance epoch and all Channel issuances in this transaction.
-- Historical returns evidence only, never a new issuance or generation.
CREATE FUNCTION game_world_reset_activate(p_world UUID,p_epoch NUMERIC,p_binding BYTEA,p_target BYTEA,
 p_map TEXT,p_server BYTEA,p_client BYTEA,p_frame BYTEA,p_node UUID,p_registration NUMERIC)
 RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE j game_world_reset_jobs%ROWTYPE; e game_world_reset_epochs%ROWTYPE; c RECORD;
 previous NUMERIC; instant TIMESTAMPTZ;
BEGIN
 PERFORM game_world_reset_lock_admission();
 LOCK TABLE game_world_reset_jobs IN SHARE MODE;
 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=p_world AND reset_epoch=p_epoch FOR UPDATE;
 IF NOT FOUND OR (j.retirement_binding,j.target_digest,j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest)
  IS DISTINCT FROM (p_binding,p_target,p_map,p_server,p_client,p_frame) THEN
  RAISE EXCEPTION 'World reset target mismatch' USING ERRCODE='23514';
 END IF;
 IF j.phase='ACTIVATED' THEN
  IF EXISTS(SELECT 1 FROM game_world_reset_active WHERE world_id=p_world AND reset_epoch=p_epoch+1) THEN RETURN 'HISTORICAL'; END IF;
  RETURN NULL;
 END IF;
 LOCK TABLE game_item_house_interior_locations IN SHARE MODE;
 IF j.target_house_tiles<>'[]'::jsonb OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=p_world)
  OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=p_world) THEN
  RAISE EXCEPTION 'World reset House qualification unavailable' USING ERRCODE='23514';
 END IF;
 LOCK TABLE game_runtime_scope_assignments IN SHARE MODE;
 LOCK TABLE game_item_ground_locations IN SHARE MODE;
 IF NOT game_world_reset_drained(p_world,p_epoch)
  OR NOT game_node_lock_current_registration(p_node,p_registration)
  OR EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=p_world AND scope_kind=1
    AND (holder_node_id,holder_registration_revision) IS DISTINCT FROM (p_node,p_registration))
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE world_id=p_world) THEN RETURN NULL; END IF;
 SELECT h.* INTO STRICT e FROM game_world_reset_active a JOIN game_world_reset_epochs h USING(world_id,reset_epoch)
  WHERE a.world_id=p_world AND a.reset_epoch=p_epoch FOR UPDATE OF a;
 instant:=clock_timestamp();
 INSERT INTO game_world_reset_epochs VALUES(p_world,p_epoch+1,p_target,p_map,p_server,p_client,p_frame,e.calendar,j.calendar_revision,
  instant,to_char((instant AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM'));
 UPDATE game_world_reset_active SET reset_epoch=p_epoch+1 WHERE world_id=p_world;
 UPDATE game_world_reset_jobs SET phase='ACTIVATED' WHERE world_id=p_world AND reset_epoch=p_epoch;
 FOR c IN SELECT channel_id FROM game_world_reset_channels WHERE world_id=p_world AND reset_epoch=p_epoch ORDER BY channel_id LIMIT 128 LOOP
  SELECT max(activation_sequence) INTO previous FROM game_content_activations WHERE world_id=p_world AND channel_id=c.channel_id;
  IF previous IS NULL OR previous>=18446744073709551615 THEN RAISE EXCEPTION 'reset content sequence unavailable' USING ERRCODE='23514'; END IF;
  PERFORM game_content_record_activation_core(p_world,c.channel_id,previous+1,previous,p_server,p_client,p_frame);
 END LOOP;
 RETURN 'NEW_EPOCH';
END $$;
REVOKE ALL ON FUNCTION game_world_reset_activate(UUID,NUMERIC,BYTEA,BYTEA,TEXT,BYTEA,BYTEA,BYTEA,UUID,NUMERIC) FROM PUBLIC;

DO $$
DECLARE f REGPROCEDURE;
BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_content_record_activation(uuid,uuid,numeric,numeric,bytea,bytea,bytea)'::regprocedure,
  'game_world_reset_lock_admission()'::regprocedure,
  'game_world_reset_initialize(uuid,bytea,text,bytea,bytea,bytea,text,numeric)'::regprocedure,
  'game_world_reset_publish(uuid,numeric,bytea,text,bytea,bytea,bytea,jsonb,numeric)'::regprocedure,
  'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 END LOOP;
END $$;
