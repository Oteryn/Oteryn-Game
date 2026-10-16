-- HOUSE-RUNTIME-1a, HOUSE_RT_INBOX §1.2/1.3/2.1. No ownership, runtime or wire activation.
-- Production access stays the 0074 no-row stub; HOUSE1a owns its later replacement.
CREATE FUNCTION game_house_access_tile(p_world UUID,p_house TEXT,p_position BYTEA)
RETURNS BOOLEAN LANGUAGE sql AS $$ SELECT false $$;
REVOKE ALL ON FUNCTION game_house_access_tile(UUID,TEXT,BYTEA) FROM PUBLIC;

ALTER TABLE game_item_transfer_reservations ALTER COLUMN channel_id DROP NOT NULL,
 DROP CONSTRAINT game_item_transfer_reservations_destination_kind_check,
 DROP CONSTRAINT game_item_transfer_reservations_channel_id_check,
 ADD CONSTRAINT game_item_transfer_reservations_channel_id_check CHECK(channel_id IS NULL OR game_character_is_uuid_v7(channel_id)),
 ADD CONSTRAINT game_item_transfer_reservations_destination_kind_check CHECK(destination_kind BETWEEN 1 AND 5),
 ADD COLUMN source_kind SMALLINT NOT NULL DEFAULT 0 CHECK(source_kind BETWEEN 0 AND 4),
 ADD COLUMN house_key TEXT,
 ADD COLUMN house_scope_generation NUMERIC(20,0),
 ADD COLUMN house_intent JSONB,
 ADD CONSTRAINT game_item_transfer_house_scope CHECK(
 (source_kind=0 AND channel_id IS NOT NULL AND house_key IS NULL AND house_scope_generation IS NULL AND house_intent IS NULL AND destination_kind IN(1,2))
 OR(source_kind>0 AND channel_id IS NULL AND house_key IS NOT NULL AND house_scope_generation IS NOT NULL AND game_house_scope_key_valid(house_key)
    AND house_intent IS NOT NULL AND jsonb_typeof(house_intent)='object' AND octet_length(house_intent::text)<=16384
    AND house_scope_generation BETWEEN 1 AND 18446744073709551615 AND destination_kind IN(3,4,5)));

ALTER TABLE game_item_transfer_receipts DROP CONSTRAINT game_item_transfer_receipts_shape_check,
 ADD CONSTRAINT game_item_transfer_receipts_shape_check CHECK(shape BETWEEN 1 AND 7);
-- Retain the four original conservation branches verbatim; new House shapes do not merge.
DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_item_transfer_receipts'::regclass
  AND contype='c' AND pg_get_constraintdef(oid) LIKE '%CASE shape%' LOOP
  EXECUTE format('ALTER TABLE game_item_transfer_receipts DROP CONSTRAINT %I',c.conname);
 END LOOP;
END $$;
ALTER TABLE game_item_transfer_receipts ADD CONSTRAINT game_item_transfer_shape_conservation CHECK(CASE shape
 WHEN 1 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NULL
             AND source_quantity_after=source_quantity_before
 WHEN 2 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NOT NULL
             AND source_quantity_after=source_quantity_before
 WHEN 3 THEN receiver_item_instance_id IS NOT NULL AND destination_ordinal IS NULL
             AND source_quantity_after=0 AND receiver_quantity_after=receiver_quantity_before+source_quantity_before
 WHEN 4 THEN receiver_item_instance_id IS NOT NULL AND destination_ordinal IS NOT NULL
             AND source_quantity_after>0 AND receiver_quantity_after>receiver_quantity_before
             AND receiver_quantity_after-receiver_quantity_before=source_quantity_before-source_quantity_after
 ELSE receiver_item_instance_id IS NULL AND source_quantity_after=source_quantity_before
      AND ((shape=6)=(destination_ordinal IS NOT NULL)) END);

-- Owner-produced physical evidence, never caller-supplied before quantities/locations.
CREATE TABLE game_item_house_shape_proofs(
 transaction_id UUID PRIMARY KEY REFERENCES game_item_transfer_receipts(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 event_id UUID NOT NULL UNIQUE,game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL,world_id UUID NOT NULL,house_key TEXT NOT NULL,
 shape SMALLINT NOT NULL CHECK(shape IN(5,6,7)),item_instance_id UUID NOT NULL,
 quantity_before BIGINT NOT NULL CHECK(quantity_before BETWEEN 1 AND 100),
 state_revision_before NUMERIC(20,0) NOT NULL,
 definition_family TEXT NOT NULL,definition_production_key TEXT NOT NULL,definition_revision_ref TEXT NOT NULL,
 key_number BIGINT NOT NULL,
 source_kind SMALLINT NOT NULL CHECK(source_kind BETWEEN 1 AND 4),
 source_parent UUID,source_ordinal NUMERIC(20,0),source_slot SMALLINT,
 source_position BYTEA,source_stack_ordinal NUMERIC(20,0),source_placement UUID,
 reclaim_subject UUID NOT NULL,provenance_before NUMERIC(20,0) NOT NULL,
 destination_position BYTEA,destination_stack_ordinal NUMERIC(20,0),
 destination_parent UUID,destination_ordinal NUMERIC(20,0),
 equipment_revision_before NUMERIC(20,0),
 scope_generation NUMERIC(20,0) NOT NULL,connection_generation NUMERIC(20,0) NOT NULL,lease_generation NUMERIC(20,0) NOT NULL,
 content_revision TEXT NOT NULL,ruleset_revision TEXT NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_item_house_shape_proofs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_house_shape_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'house shape evidence is immutable' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_item_house_shape_proofs_immutable BEFORE UPDATE OR DELETE ON game_item_house_shape_proofs
 FOR EACH ROW EXECUTE FUNCTION game_house_shape_immutable();
CREATE TRIGGER game_item_house_shape_proofs_no_truncate BEFORE TRUNCATE ON game_item_house_shape_proofs
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Returns actual changed facts for native audit encoding in the SAME physical transaction.
-- Deferred proofs below prevent committing a shape without its canonical receipt and event.
CREATE FUNCTION game_house_item_apply(p_shape SMALLINT,p JSONB,p_node UUID,p_node_revision NUMERIC,p_secret BYTEA)
RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 s game_durability_reconnect_sessions%ROWTYPE;r game_character_roots%ROWTYPE;
 i game_item_instances%ROWTYPE;v game_item_transfer_reservations%ROWTYPE;
 h game_item_house_interior_locations%ROWTYPE;pr game_item_house_reclaim_provenance%ROWTYPE;
 e game_item_house_shape_proofs%ROWTYPE;a RECORD;pos BYTEA;eqrev NUMERIC;mode SMALLINT;rel TEXT;preflight BOOLEAN;
BEGIN
 IF p IS NULL OR jsonb_typeof(p)<>'object' OR octet_length(p::text)>16384 OR p_shape NOT IN(5,6,7)
  OR NOT p ?& ARRAY['session','command','transaction','item','character','world','house','scope_generation','connection_generation','lease_generation','acl_revision','content_revision','ruleset_revision']
  OR EXISTS(SELECT 1 FROM jsonb_each(p) x WHERE x.key IN('session','command','transaction','item','character','world','house','scope_generation','connection_generation','lease_generation','acl_revision','content_revision','ruleset_revision') AND x.value='null'::jsonb) THEN RAISE EXCEPTION 'INVALID_INPUT';END IF;
 IF p->>'phase' IS NULL OR p->>'phase' NOT IN('prepare','apply') THEN RAISE EXCEPTION 'INVALID_INPUT';END IF;
 preflight:=(p->>'phase'='prepare');
 -- Same complete relation order as the native semantic transaction, including direct SQL calls.
 FOREACH rel IN ARRAY ARRAY[
 'game_durability_admission_account_guards','game_durability_admission_character_guards',
 'game_durability_admission_guard_history','game_durability_admission_lifecycle_receipts',
 'game_durability_admission_runtime_guards','game_durability_admission_signing_trust_guards',
 'game_durability_control_loss_continuity','game_durability_executor_custody',
 'game_durability_fresh_admission_receipts','game_durability_reconnect_attempts',
 'game_durability_reconnect_pending_commands','game_durability_reconnect_sessions',
 'game_durability_recovery_grant_consumptions','game_durability_session_replacements',
 'game_durability_session_use_ledgers','game_durability_session_use_memberships',
 'game_durability_transport_ref_reservations'] LOOP EXECUTE format('LOCK TABLE %I IN EXCLUSIVE MODE',rel);END LOOP;
 IF game_node_prove_current_incarnation(p_node,p_node_revision,p_secret) IS DISTINCT FROM true THEN RAISE EXCEPTION 'NOT_CURRENT';END IF;
 SELECT * INTO v FROM game_item_transfer_reservations WHERE game_session_id=(p->>'session')::uuid
  AND command_id=(p->>'command')::numeric FOR UPDATE;
 IF NOT FOUND OR v.transaction_id<>(p->>'transaction')::uuid OR v.source_item_instance_id<>(p->>'item')::uuid
  OR v.character_id<>(p->>'character')::uuid OR v.world_id<>(p->>'world')::uuid
  OR v.house_key IS DISTINCT FROM p->>'house' OR v.house_scope_generation<>(p->>'scope_generation')::numeric
  OR v.destination_kind<>p_shape-2 OR (v.work_units_used NOT BETWEEN 0 AND 3 OR(NOT preflight AND v.work_units_used=0))
  OR v.house_intent IS DISTINCT FROM (p-'transaction'-'scope_generation'-'connection_generation'-'lease_generation'-'phase')
  OR v.channel_id IS NOT NULL OR EXISTS(SELECT 1 FROM game_item_transfer_receipts t WHERE t.transaction_id=v.transaction_id)
 THEN RAISE EXCEPTION 'CONFLICTING_OCCURRENCE';END IF;
 SELECT * INTO s FROM game_durability_reconnect_sessions WHERE game_session_id=v.game_session_id
  AND character_id=v.character_id AND world_id=v.world_id AND runtime_scope_kind=2
  AND runtime_scope_world_id=v.world_id AND runtime_scope_channel_id IS NULL
  AND runtime_scope_instance_id=game_house_scope_instance_id(v.world_id,v.house_key)
  AND runtime_scope_house_key=v.house_key AND current_generation=(p->>'connection_generation')::numeric
  AND character_lease_generation=(p->>'lease_generation')::numeric
  AND scope_ownership_generation=v.house_scope_generation AND session_state IN(1,2) FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'NOT_CURRENT';END IF;
 PERFORM 1 FROM game_runtime_scope_assignments WHERE scope_key=game_house_scope_key(v.world_id,v.house_key)
  AND scope_kind=2 AND world_id=v.world_id AND house_key=v.house_key AND state=1
  AND ownership_generation=v.house_scope_generation AND holder_node_id=p_node
  AND holder_registration_revision=p_node_revision FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'NOT_CURRENT';END IF;
 SELECT * INTO r FROM game_character_roots WHERE character_id=v.character_id AND account_id=s.account_id
  AND world_id=v.world_id AND lifecycle=1 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'NOT_CURRENT';END IF;
 IF r.content_revision IS DISTINCT FROM p->>'content_revision' OR r.ruleset_revision IS DISTINCT FROM p->>'ruleset_revision' THEN RAISE EXCEPTION 'CONTENT_CHANGED';END IF;
 -- One lock serializes missing-row storage count and destination ordinal allocation.
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||v.world_id::text||':'||v.house_key,0));
 SELECT q.*,count(*) OVER() AS grant_count INTO a FROM game_house_access(v.world_id,v.house_key,v.character_id) q;
 IF NOT FOUND OR a.grant_count<>1 OR a.role IS NULL OR a.role NOT IN('OWNER','SUBOWNER','GUEST') THEN RAISE EXCEPTION 'NO_ACCESS';END IF;
 IF a.content_fenced IS DISTINCT FROM false THEN RAISE EXCEPTION 'HOUSE_CLOSED';END IF;
 IF a.acl_revision IS DISTINCT FROM (p->>'acl_revision')::numeric
  OR a.guild_revisions IS DISTINCT FROM nullif(p->'guild_revisions','null'::jsonb) THEN RAISE EXCEPTION 'ACCESS_CHANGED';END IF;
 IF p_shape IN(5,7) THEN
  pos:=decode(p->>'destination_position','hex');
  IF pos IS NULL OR octet_length(pos) NOT BETWEEN 1 AND 128 OR game_house_access_tile(v.world_id,v.house_key,pos) IS DISTINCT FROM true
   THEN RAISE EXCEPTION 'NOT_HOUSE_TILE';END IF;
  IF p_shape=5 AND(SELECT count(*) FROM game_item_house_interior_locations WHERE world_id=v.world_id AND house_key=v.house_key)>=2000
   THEN RAISE EXCEPTION 'HOUSE_STORAGE_FULL';END IF;
 END IF;
 -- Parent and item rows use ItemInstanceId order, after the Character/House locks.
 PERFORM 1 FROM game_item_instances WHERE item_instance_id=v.source_item_instance_id
  OR item_instance_id IN(SELECT item_instance_id FROM game_item_container_slots WHERE character_id=v.character_id)
  ORDER BY item_instance_id FOR UPDATE;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=v.source_item_instance_id AND world_id=v.world_id AND lifecycle=1;
 IF NOT FOUND OR i.quantity NOT BETWEEN 1 AND 100 THEN RAISE EXCEPTION 'SOURCE_CHANGED';END IF;
 IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'ITEM_HAS_CONTENTS';END IF;
 e.transaction_id:=v.transaction_id;e.event_id:=v.event_id;e.game_session_id:=v.game_session_id;e.command_id:=v.command_id;
 e.character_id:=v.character_id;e.world_id:=v.world_id;e.house_key:=v.house_key;e.shape:=p_shape;e.item_instance_id:=i.item_instance_id;
 e.quantity_before:=i.quantity;e.definition_family:=i.definition_family;e.definition_production_key:=i.definition_production_key;
 e.definition_revision_ref:=i.definition_revision_ref;
 SELECT coalesce((SELECT key_number FROM game_item_keys WHERE item_instance_id=i.item_instance_id),0) INTO e.key_number;
 e.state_revision_before:=i.state_revision;
 e.source_kind:=v.source_kind;e.reclaim_subject:=v.character_id;e.provenance_before:=0;
 e.scope_generation:=v.house_scope_generation;e.connection_generation:=s.current_generation;e.lease_generation:=s.character_lease_generation;
 e.content_revision:=r.content_revision;e.ruleset_revision:=r.ruleset_revision;e.created_xact_id:=pg_current_xact_id();
 IF p_shape=5 THEN
  IF v.source_kind=1 THEN
   SELECT parent_item_instance_id,placement_ordinal INTO e.source_parent,e.source_ordinal FROM game_item_container_entries
    WHERE item_instance_id=i.item_instance_id AND character_id=v.character_id
     AND parent_item_instance_id=(p->>'source_parent')::uuid AND placement_ordinal=(p->>'source_ordinal')::numeric;
   IF NOT FOUND THEN RAISE EXCEPTION 'SOURCE_CHANGED';END IF;
  ELSIF v.source_kind=2 THEN
   SELECT placed_transaction_id INTO e.source_placement FROM game_item_container_slots
    WHERE item_instance_id=i.item_instance_id AND character_id=v.character_id;
   IF NOT FOUND THEN RAISE EXCEPTION 'SOURCE_CHANGED';END IF;e.source_slot:=9;
  ELSIF v.source_kind=3 THEN
   SELECT revision,combat_mode INTO eqrev,mode FROM game_character_equipment_state WHERE character_id=v.character_id FOR UPDATE;
   PERFORM 1 FROM game_character_equipment_slots WHERE character_id=v.character_id AND item_instance_id=i.item_instance_id AND slot=(p->>'source_slot')::smallint;
   IF NOT FOUND OR eqrev IS NULL THEN RAISE EXCEPTION 'SOURCE_CHANGED';END IF;
   e.source_slot:=(p->>'source_slot')::smallint;e.equipment_revision_before:=eqrev;
  ELSE RAISE EXCEPTION 'INVALID_INPUT';END IF;
 ELSE
  IF v.source_kind<>4 THEN RAISE EXCEPTION 'INVALID_INPUT';END IF;
  SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id AND world_id=v.world_id AND house_key=v.house_key;
  IF NOT FOUND OR h.placed_transaction_id IS DISTINCT FROM (p->>'source_placement')::uuid THEN RAISE EXCEPTION 'SOURCE_CHANGED';END IF;
  IF game_house_access_tile(v.world_id,v.house_key,h.spatial_position) IS DISTINCT FROM true THEN RAISE EXCEPTION 'NOT_HOUSE_TILE';END IF;
  SELECT * INTO pr FROM game_item_house_reclaim_provenance WHERE item_instance_id=i.item_instance_id;
  IF NOT FOUND OR pr.provenance_revision IS DISTINCT FROM (p->>'source_provenance_revision')::numeric THEN RAISE EXCEPTION 'SOURCE_CHANGED';END IF;
  IF a.role='GUEST' AND p_shape=6 AND pr.reclaim_subject_character_id<>v.character_id THEN RAISE EXCEPTION 'NO_ACCESS';END IF;
  e.source_position:=h.spatial_position;e.source_stack_ordinal:=h.stack_ordinal;e.source_placement:=h.placed_transaction_id;
  e.reclaim_subject:=pr.reclaim_subject_character_id;e.provenance_before:=pr.provenance_revision;
 END IF;
 IF p_shape IN(5,7) THEN
  IF p_shape=7 AND pos=e.source_position THEN RAISE EXCEPTION 'NO_CHANGE';END IF;
  e.destination_position:=pos;
  SELECT coalesce(max(stack_ordinal),0)+1 INTO e.destination_stack_ordinal FROM game_item_house_interior_locations
   WHERE world_id=v.world_id AND house_key=v.house_key AND spatial_position=pos;
  IF e.destination_stack_ordinal>18446744073709551615 THEN RAISE EXCEPTION 'HOUSE_STORAGE_FULL';END IF;
 ELSE
  SELECT item_instance_id INTO e.destination_parent FROM game_item_container_slots WHERE character_id=v.character_id;
  IF NOT FOUND THEN RAISE EXCEPTION 'INVENTORY_FULL';END IF;
  IF (p->>'backpack_capacity')::integer NOT BETWEEN 1 AND 20 OR p->>'backpack_capacity' IS NULL
   OR NOT EXISTS(SELECT 1 FROM game_item_instances b WHERE b.item_instance_id=e.destination_parent AND b.world_id=v.world_id AND b.lifecycle=1
     AND b.definition_family=p->>'backpack_family' AND b.definition_production_key=p->>'backpack_key' AND b.definition_revision_ref=p->>'backpack_revision')
   THEN RAISE EXCEPTION 'CONTENT_CHANGED';END IF;
  IF(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=e.destination_parent)>=(p->>'backpack_capacity')::integer THEN RAISE EXCEPTION 'INVENTORY_FULL';END IF;
  SELECT coalesce(max(placement_ordinal),0)+1 INTO e.destination_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=e.destination_parent;
  IF e.destination_ordinal>18446744073709551615 THEN RAISE EXCEPTION 'INVENTORY_FULL';END IF;
 END IF;
 IF preflight THEN RETURN jsonb_build_object('phase','prepared','facts',to_jsonb(e));END IF;
 INSERT INTO game_item_house_shape_proofs SELECT(e).*;
 IF p_shape=5 THEN
  IF v.source_kind=1 THEN DELETE FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id;
  ELSIF v.source_kind=2 THEN DELETE FROM game_item_container_slots WHERE item_instance_id=i.item_instance_id;
  ELSE DELETE FROM game_character_equipment_slots WHERE item_instance_id=i.item_instance_id;
   UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=v.transaction_id WHERE character_id=v.character_id;
  END IF;
 ELSE DELETE FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id;
 END IF;
 IF p_shape=6 THEN
  DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=i.item_instance_id;
  INSERT INTO game_item_container_entries VALUES(i.item_instance_id,v.world_id,v.character_id,e.destination_parent,e.destination_ordinal,v.transaction_id);
 ELSE
  INSERT INTO game_item_house_interior_locations VALUES(i.item_instance_id,v.world_id,v.house_key,pos,e.destination_stack_ordinal,v.transaction_id);
  IF p_shape=5 THEN INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision)
   VALUES(i.item_instance_id,v.world_id,v.house_key,v.character_id,v.transaction_id,1);
  ELSE UPDATE game_item_house_reclaim_provenance SET placement_transaction_id=v.transaction_id,provenance_revision=provenance_revision+1 WHERE item_instance_id=i.item_instance_id;
  END IF;
 END IF;
 UPDATE game_item_instances SET last_transaction_id=v.transaction_id,state_revision=state_revision+1 WHERE item_instance_id=i.item_instance_id;
 RETURN to_jsonb(e);
END $$;

CREATE FUNCTION game_house_item_place(p JSONB,n UUID,r NUMERIC,s BYTEA) RETURNS JSONB
 LANGUAGE sql SECURITY DEFINER AS $$ SELECT game_house_item_apply(5::smallint,p,n,r,s) $$;
CREATE FUNCTION game_house_item_take(p JSONB,n UUID,r NUMERIC,s BYTEA) RETURNS JSONB
 LANGUAGE sql SECURITY DEFINER AS $$ SELECT game_house_item_apply(6::smallint,p,n,r,s) $$;
CREATE FUNCTION game_house_item_move(p JSONB,n UUID,r NUMERIC,s BYTEA) RETURNS JSONB
 LANGUAGE sql SECURITY DEFINER AS $$ SELECT game_house_item_apply(7::smallint,p,n,r,s) $$;

-- All remaining same-transaction conservation/placement proof extensions follow below.
CREATE FUNCTION game_house_shape_complete(p_tx UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_item_house_shape_proofs%ROWTYPE;r game_item_transfer_receipts%ROWTYPE;i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO p FROM game_item_house_shape_proofs WHERE transaction_id=p_tx;
 IF NOT FOUND OR p.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'house proof not current physical transaction' USING ERRCODE='23514';END IF;
 SELECT * INTO r FROM game_item_transfer_receipts WHERE transaction_id=p_tx;
 IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id()
  OR(r.game_session_id,r.command_id,r.character_id,r.event_id,r.shape,r.source_item_instance_id,r.source_quantity_before,r.source_quantity_after)
   IS DISTINCT FROM(p.game_session_id,p.command_id,p.character_id,p.event_id,p.shape,p.item_instance_id,p.quantity_before,p.quantity_before)
  OR r.receiver_item_instance_id IS NOT NULL OR r.receiver_quantity_before IS NOT NULL OR r.receiver_quantity_after IS NOT NULL
  OR(r.destination_parent_item_instance_id,r.destination_ordinal) IS DISTINCT FROM(p.destination_parent,p.destination_ordinal)
 THEN RAISE EXCEPTION 'house receipt not exact shape' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_item_transfer_reservations v WHERE(v.game_session_id,v.command_id,v.character_id,v.world_id,v.source_item_instance_id,v.transaction_id,v.event_id,v.occurred_at,v.intent_binding)
  =(r.game_session_id,r.command_id,r.character_id,p.world_id,r.source_item_instance_id,r.transaction_id,r.event_id,r.occurred_at,r.intent_binding)
  AND v.channel_id IS NULL AND v.house_key=p.house_key AND v.house_scope_generation=p.scope_generation
  AND v.source_kind=p.source_kind AND v.destination_kind=p.shape-2 AND v.work_units_used BETWEEN 1 AND 3;
 IF NOT FOUND THEN RAISE EXCEPTION 'house receipt lacks exact reservation' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_tx AND a.event_id=p.event_id AND a.item_instance_id=p.item_instance_id
  AND a.created_xact_id=pg_current_xact_id() AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
  AND a.event_type_id=2 AND a.transaction_count=1 AND a.transaction_ordinal=1 AND a.publication_state=1;
 IF NOT FOUND THEN RAISE EXCEPTION 'house receipt lacks same-transaction audit' USING ERRCODE='23514';END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=p.item_instance_id;
 IF NOT FOUND OR i.lifecycle<>1 OR i.world_id<>p.world_id OR i.quantity<>p.quantity_before OR i.last_transaction_id<>p_tx
  OR i.state_revision<>p.state_revision_before+1 OR(i.definition_family,i.definition_production_key,i.definition_revision_ref)
   IS DISTINCT FROM(p.definition_family,p.definition_production_key,p.definition_revision_ref)
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=p.item_instance_id AND q.transaction_id=p_tx AND q.quantity_before=p.quantity_before)
 THEN RAISE EXCEPTION 'house item differs from actual before evidence' USING ERRCODE='23514';END IF;
 IF p.shape IN(5,7) THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance pr USING(item_instance_id)
   WHERE h.item_instance_id=p.item_instance_id AND h.world_id=p.world_id AND h.house_key=p.house_key
    AND h.spatial_position=p.destination_position AND h.stack_ordinal=p.destination_stack_ordinal AND h.placed_transaction_id=p_tx
    AND pr.placement_transaction_id=p_tx AND pr.reclaim_subject_character_id=p.reclaim_subject
    AND pr.provenance_revision=p.provenance_before+1 AND pr.written_xact_id=pg_current_xact_id())
   THEN RAISE EXCEPTION 'house destination lacks exact provenance' USING ERRCODE='23514';END IF;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=p.item_instance_id AND e.world_id=p.world_id
   AND e.character_id=p.character_id AND e.parent_item_instance_id=p.destination_parent AND e.placement_ordinal=p.destination_ordinal
   AND e.placed_transaction_id=p_tx) OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=p.item_instance_id)
   THEN RAISE EXCEPTION 'house take lacks exact inventory destination' USING ERRCODE='23514';END IF;
 END IF;
END $$;
CREATE FUNCTION game_house_shape_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_house_shape_complete(NEW.transaction_id);RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER game_house_shape_proven AFTER INSERT ON game_item_house_shape_proofs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_shape_proven();

-- 0011 made B3 slots immutable before this accepted House source existed. Keep
-- the generic immutable function and every other consumer unchanged. Only the
-- actual, owner-observed slot tuple may be removed by this physical House Place.
CREATE FUNCTION game_house_container_slot_removal_proven() RETURNS trigger
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p_tx UUID;
BEGIN
 IF TG_OP='DELETE' THEN
  SELECT p.transaction_id INTO p_tx FROM game_item_house_shape_proofs p
   WHERE p.created_xact_id=pg_current_xact_id() AND p.shape=5 AND p.source_kind=2
    AND p.source_slot=9 AND p.item_instance_id=OLD.item_instance_id
    AND p.character_id=OLD.character_id AND p.world_id=OLD.world_id
    AND p.source_placement=OLD.placed_transaction_id;
  IF FOUND THEN
   IF TG_WHEN='AFTER' THEN PERFORM game_house_shape_complete(p_tx);END IF;
   RETURN OLD;
  END IF;
 END IF;
 RAISE EXCEPTION 'DUR-03 item record is immutable' USING ERRCODE='23514';
END $$;
DROP TRIGGER game_item_container_slot_immutable ON game_item_container_slots;
CREATE TRIGGER game_item_container_slot_immutable BEFORE UPDATE OR DELETE ON game_item_container_slots
 FOR EACH ROW EXECUTE FUNCTION game_house_container_slot_removal_proven();
CREATE CONSTRAINT TRIGGER game_house_container_slot_removed AFTER DELETE ON game_item_container_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_container_slot_removal_proven();

CREATE FUNCTION game_house_location_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.placed_transaction_id
   AND p.item_instance_id=NEW.item_instance_id AND p.world_id=NEW.world_id AND p.house_key=NEW.house_key
   AND p.destination_position=NEW.spatial_position AND p.destination_stack_ordinal=NEW.stack_ordinal
   AND p.shape IN(5,7) AND p.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'house insertion lacks own physical shape' USING ERRCODE='23514';END IF;
 ELSIF TG_OP='DELETE' THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.item_instance_id=OLD.item_instance_id
   AND p.world_id=OLD.world_id AND p.house_key=OLD.house_key AND p.source_position=OLD.spatial_position
   AND p.source_stack_ordinal=OLD.stack_ordinal AND p.source_placement=OLD.placed_transaction_id
   AND p.shape IN(6,7) AND p.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'house removal lacks actual source shape' USING ERRCODE='23514';END IF;
 ELSE RAISE EXCEPTION 'house relocation requires delete/insert' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_location_proven AFTER INSERT OR DELETE OR UPDATE ON game_item_house_interior_locations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_location_proven();

CREATE OR REPLACE FUNCTION game_item_instance_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.last_transaction_id
     AND p.item_instance_id=OLD.item_instance_id AND p.world_id=OLD.world_id AND p.quantity_before=OLD.quantity
     AND p.state_revision_before=OLD.state_revision AND p.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.state_revision<>OLD.state_revision+1 OR(to_jsonb(NEW)-'last_transaction_id'-'state_revision')
      IS DISTINCT FROM(to_jsonb(OLD)-'last_transaction_id'-'state_revision') THEN RAISE EXCEPTION 'house transfer preserves item state' USING ERRCODE='23514';END IF;
     INSERT INTO game_item_transfer_quantity_evidence VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
     RETURN NEW;
    END IF;

    IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
      AND (to_jsonb(NEW)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
        =(to_jsonb(OLD)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
      AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
        WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
          AND r.created_xact_id=pg_current_xact_id() AND r.state_revision_before=OLD.state_revision AND r.quantity_before=OLD.quantity
          AND r.occurred_at_unix_ms>=p.expires_at_unix_ms
          AND (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
          AND ((p.target_family IS NULL AND NEW.lifecycle=2 AND NEW.quantity=0
            AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref))
           OR (p.target_family IS NOT NULL AND NEW.lifecycle=1 AND NEW.quantity=OLD.quantity
             AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(p.target_family,p.target_production_key,p.target_revision)))) THEN
        NEW.state_revision:=OLD.state_revision+1;
        INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before) VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
        RETURN NEW;
    END IF;

    IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
      AND (to_jsonb(NEW)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
        =(to_jsonb(OLD)-'definition_family'-'definition_production_key'-'definition_revision_ref'-'quantity'-'lifecycle'-'last_transaction_id'-'state_revision')
      AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
        WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
          AND r.created_xact_id=pg_current_xact_id() AND r.state_revision_before=OLD.state_revision AND r.quantity_before=OLD.quantity
          AND r.occurred_at_unix_ms>=p.expires_at_unix_ms
          AND (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
          AND ((p.target_family IS NULL AND NEW.lifecycle=2 AND NEW.quantity=0
            AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref))
           OR (p.target_family IS NOT NULL AND NEW.lifecycle=1 AND NEW.quantity=OLD.quantity
             AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)=(p.target_family,p.target_production_key,p.target_revision)))) THEN
        NEW.state_revision:=OLD.state_revision+1;
        INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before) VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
        RETURN NEW;
    END IF;

    NEW.state_revision := OLD.state_revision + 1;
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'quantity' - 'lifecycle' - 'last_transaction_id' - 'state_revision')
         = (to_jsonb(OLD) - 'quantity' - 'lifecycle' - 'last_transaction_id' - 'state_revision') THEN
        INSERT INTO game_item_transfer_quantity_evidence
            (item_instance_id, transaction_id, quantity_before)
        VALUES (OLD.item_instance_id, NEW.last_transaction_id, OLD.quantity);
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1 AND NEW.lifecycle = 1
       AND NEW.quantity = OLD.quantity
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'definition_production_key'
                - 'definition_revision_ref' - 'last_transaction_id' - 'state_revision')
         = (to_jsonb(OLD) - 'definition_production_key'
                - 'definition_revision_ref' - 'last_transaction_id' - 'state_revision')
       AND EXISTS (
           SELECT 1 FROM game_item_timed_state_writes w
            WHERE w.item_instance_id = NEW.item_instance_id
              AND w.transaction_id = NEW.last_transaction_id
              AND w.cause IN (2, 3)
              AND w.created_xact_id = pg_current_xact_id()
              AND (w.definition_before_family, w.definition_before_production_key,
                   w.definition_before_revision_ref)
                = (OLD.definition_family, OLD.definition_production_key,
                   OLD.definition_revision_ref)
              AND (w.definition_after_family, w.definition_after_production_key,
                   w.definition_after_revision_ref)
                = (NEW.definition_family, NEW.definition_production_key,
                   NEW.definition_revision_ref)) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item record changes only through a TRANSFER'
        USING ERRCODE = '23514';
END;
$$;

CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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

CREATE OR REPLACE FUNCTION game_item_transfer_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    src game_item_instances%ROWTYPE;
    rcv game_item_instances%ROWTYPE;
    slot_item UUID;
    src_places INTEGER;
    corpse_ev game_item_corpse_entry_removal_evidence%ROWTYPE;
    from_corpse BOOLEAN;
    corpse_top UUID;
    corpse_materialized BIGINT;
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF NEW.shape IN(5,6,7) THEN PERFORM game_house_shape_complete(NEW.transaction_id);RETURN NULL;END IF;

    SELECT * INTO src FROM game_item_instances WHERE item_instance_id = NEW.source_item_instance_id;
    -- D134: the corpse ItemInstance itself is never a legal TRANSFER source.
    IF EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
         WHERE m.item_instance_id = NEW.source_item_instance_id
           AND m.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'a corpse item is never a TRANSFER source' USING ERRCODE = '23514';
    END IF;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_corpse_container_entries ce
             WHERE ce.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_character_inbox_locations ib
             WHERE ib.item_instance_id = src.item_instance_id)
      INTO src_places;
    SELECT * INTO corpse_ev FROM game_item_corpse_entry_removal_evidence ev
     WHERE ev.item_instance_id = NEW.source_item_instance_id
       AND ev.created_xact_id = pg_current_xact_id();
    from_corpse := FOUND;
    IF NOT EXISTS (
           SELECT 1 FROM game_item_transfer_reservations v
            WHERE (v.game_session_id, v.command_id, v.character_id, v.source_item_instance_id,
                   v.intent_binding, v.transaction_id, v.event_id, v.occurred_at)
                = (NEW.game_session_id, NEW.command_id, NEW.character_id,
                   NEW.source_item_instance_id, NEW.intent_binding, NEW.transaction_id,
                   NEW.event_id, NEW.occurred_at)
              AND v.world_id = src.world_id
              AND (v.destination_kind = 1) = (NEW.shape = 1))
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.source_item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              -- A new TRANSFER's audit event starts pending and unpublished;
              -- only the publisher's acknowledgement may advance it.
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The destination Character must be rooted in the source item's World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = src.world_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.source_item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.source_quantity_before)
       -- The reservation's World and Channel must equal the custody this
       -- TRANSFER actually removed, proven from the guarded pre-DELETE
       -- evidence (the source row is gone by now): a Ground row, or a corpse
       -- entry together with its corpse's live Ground.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_reservations v
            WHERE v.game_session_id = NEW.game_session_id AND v.command_id = NEW.command_id
              AND (EXISTS (
                     SELECT 1 FROM game_item_ground_removal_evidence gre
                      WHERE gre.item_instance_id = NEW.source_item_instance_id
                        AND gre.world_id = v.world_id
                        AND gre.channel_id = v.channel_id)
                   OR (from_corpse
                       AND corpse_ev.world_id = v.world_id
                       AND corpse_ev.channel_id = v.channel_id)))
       OR src.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR src.quantity <> NEW.source_quantity_after
       OR src.lifecycle <> (CASE WHEN NEW.source_quantity_after = 0 THEN 2 ELSE 1 END)
       OR src_places <> (CASE WHEN NEW.shape = 3 THEN 0 ELSE 1 END)
       OR (NEW.shape = 1 AND NOT EXISTS (
           SELECT 1 FROM game_item_container_slots s
            WHERE s.character_id = NEW.character_id
              AND s.item_instance_id = src.item_instance_id
              AND s.placed_transaction_id = NEW.transaction_id))
       OR (NEW.shape IN (2,4) AND NOT EXISTS (
           SELECT 1 FROM game_item_container_entries e
            WHERE e.item_instance_id = src.item_instance_id
              AND e.character_id = NEW.character_id
              AND e.parent_item_instance_id = NEW.destination_parent_item_instance_id
              AND e.parent_item_instance_id = slot_item
              AND e.placement_ordinal = NEW.destination_ordinal
              AND e.placed_transaction_id = NEW.transaction_id)) THEN
        RAISE EXCEPTION 'item TRANSFER must commit its reservation, audit event and complete item state together'
            USING ERRCODE = '23514';
    END IF;
    -- D133: during [materialized_at, materialized_at + 10 s) only the corpse's
    -- top-damage Character may take an entry out; afterwards anyone may. The
    -- deadline is judged by the database clock, never a caller-supplied time.
    IF from_corpse THEN
        SELECT r.corpse_top_damage_character_id, r.materialized_at
          INTO corpse_top, corpse_materialized
          FROM game_item_mint_receipts r
         WHERE r.item_instance_id = corpse_ev.parent_item_instance_id
           AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
           AND r.draw_ordinal = 0;
        IF NOT FOUND OR corpse_top IS NULL OR corpse_materialized IS NULL THEN
            RAISE EXCEPTION 'corpse container entry source requires a materialized corpse'
                USING ERRCODE = '23514';
        END IF;
        IF NEW.character_id IS DISTINCT FROM corpse_top
           AND floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
               < corpse_materialized + 10000 THEN
            RAISE EXCEPTION 'corpse loot is exclusive to its top-damage character until the D133 window closes'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NEW.receiver_item_instance_id IS NOT NULL THEN
        SELECT * INTO rcv FROM game_item_instances
         WHERE item_instance_id = NEW.receiver_item_instance_id;
        IF rcv.last_transaction_id IS DISTINCT FROM NEW.transaction_id
           OR rcv.lifecycle <> 1
           OR rcv.quantity <> NEW.receiver_quantity_after
           OR (rcv.world_id, rcv.definition_family, rcv.definition_production_key,
               rcv.definition_revision_ref)
              IS DISTINCT FROM (src.world_id, src.definition_family,
                                src.definition_production_key, src.definition_revision_ref)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_container_entries e
                WHERE e.item_instance_id = rcv.item_instance_id
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = slot_item)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_transfer_quantity_evidence ev
                WHERE ev.item_instance_id = rcv.item_instance_id
                  AND ev.transaction_id = NEW.transaction_id
                  AND ev.quantity_before = NEW.receiver_quantity_before) THEN
            RAISE EXCEPTION 'item TRANSFER receiver must be a compatible stack in the main backpack'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=5 AND p.source_kind=1
      AND p.item_instance_id=OLD.item_instance_id AND p.character_id=OLD.character_id AND p.world_id=OLD.world_id
      AND p.source_parent=OLD.parent_item_instance_id AND p.source_ordinal=OLD.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    -- 0082: the whole BURN of an NPC trade line of this same physical transaction (NPC-0 §5.5).
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND t.character_id = OLD.character_id
           AND t.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND i.lifecycle = 2
           AND l.created_xact_id = pg_current_xact_id()
           AND t.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    -- 0083: the entry of a used stack spent to zero, by the item-use record of this same
    -- physical transaction.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_use_records u
          JOIN game_item_instances i ON i.item_instance_id = u.source_item_instance_id
         WHERE u.source_item_instance_id = OLD.item_instance_id
           AND u.source_quantity_after = 0
           AND u.character_id = OLD.character_id
           AND u.source_parent_item_instance_id = OLD.parent_item_instance_id
           AND u.source_placement_ordinal = OLD.placement_ordinal
           AND i.last_transaction_id = u.transaction_id
           AND i.lifecycle = 2
           AND u.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND (l.operation_kind=6 OR r.character_id=OLD.character_id)
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.to_slot IS NOT NULL AND r.item_instance_id=OLD.item_instance_id
         AND r.character_id=OLD.character_id AND r.backpack_item_instance_id=OLD.parent_item_instance_id
         AND r.backpack_ordinal=OLD.placement_ordinal AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND f.character_id = OLD.character_id
           AND f.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND f.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
          JOIN game_item_instances i ON i.item_instance_id = w.item_instance_id
         WHERE w.item_instance_id = OLD.item_instance_id
           AND w.cause IN (2, 3)
           AND w.holder_character_id = OLD.character_id
           AND (w.definition_before_family, w.definition_before_production_key,
                w.definition_before_revision_ref)
             = (w.definition_after_family, w.definition_after_production_key,
                w.definition_after_revision_ref)
           AND i.lifecycle = 2
           AND i.last_transaction_id = w.transaction_id
           AND w.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.direction = 1
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND o.acting_character_id = OLD.character_id
           AND o.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND o.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=6 AND p.transaction_id=NEW.placed_transaction_id
      AND p.item_instance_id=NEW.item_instance_id AND p.world_id=NEW.world_id AND p.character_id=NEW.character_id
      AND p.destination_parent=NEW.parent_item_instance_id AND p.destination_ordinal=NEW.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    IF TG_TABLE_NAME='game_item_container_entries' THEN
    -- 0082: the new entry of an NPC trade MINT line of this same physical transaction.
    IF EXISTS(
       SELECT 1 FROM game_npc_trade_lines l
         JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
         JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
        WHERE l.transaction_id = NEW.placed_transaction_id
          AND l.item_instance_id = NEW.item_instance_id
          AND l.line_kind IN (2, 4)
          AND l.placement_ordinal = NEW.placement_ordinal
          AND t.character_id = NEW.character_id
          AND t.backpack_item_instance_id = NEW.parent_item_instance_id
          AND i.lifecycle = 1
          AND l.created_xact_id = pg_current_xact_id()
          AND t.created_xact_id = pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    -- 0083: the fresh flask item of an item-use record of this same physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u JOIN game_item_instances i ON i.item_instance_id = NEW.item_instance_id
       WHERE u.shape = 4 AND u.flask_item_instance_id = NEW.item_instance_id
         AND u.transaction_id = NEW.placed_transaction_id AND u.character_id = NEW.character_id
         AND u.source_parent_item_instance_id = NEW.parent_item_instance_id
         AND u.flask_placement_ordinal = NEW.placement_ordinal
         AND i.lifecycle = 1 AND u.created_xact_id = pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
           RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
       END IF;
       RETURN NULL;
    END IF;
    IF EXISTS(
       SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.from_slot IS NOT NULL AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
         AND r.backpack_item_instance_id=NEW.parent_item_instance_id AND r.backpack_ordinal=NEW.placement_ordinal
         AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
         AND i.lifecycle=1 AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'equipment backpack entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.operation_kind=1 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
        AND l.destination_parent_item_instance_id=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
        AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514'; END IF;
       RETURN NULL;
    END IF;
    END IF;
    IF TG_TABLE_NAME = 'game_item_container_slots' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape = 1
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER receipt'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_item_container_entries' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape IN (2,4)
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_reward_claim_mint_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.item_instance_id = NEW.item_instance_id
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_item_fee_burns f
             WHERE f.transaction_id = NEW.placed_transaction_id
               AND f.character_id = NEW.character_id
               AND f.backpack_item_instance_id = NEW.parent_item_instance_id
               AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                            f.change_gold_item_instance_id)
               AND f.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_account_bank_coin_lines l
              JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
             WHERE o.transaction_id = NEW.placed_transaction_id
               AND o.acting_character_id = NEW.character_id
               AND o.backpack_item_instance_id = NEW.parent_item_instance_id
               AND l.item_instance_id = NEW.item_instance_id
               AND l.direction = 2
               AND l.placement_ordinal = NEW.placement_ordinal
               AND l.created_xact_id = pg_current_xact_id()
               AND o.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER, reward-claim MINT or fee change receipt'
                USING ERRCODE = '23514';
        END IF;
        PERFORM 1 FROM game_item_container_slots
         WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
        IF (SELECT count(*) FROM game_item_container_entries e
             WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
            RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_equipment_slot_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=5 AND p.source_kind=3
      AND p.item_instance_id=OLD.item_instance_id AND p.world_id=OLD.world_id AND p.character_id=OLD.character_id
      AND p.source_slot=OLD.slot AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=3
        AND r.character_id=OLD.character_id AND r.source_slot=OLD.slot AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
      JOIN game_spell_equipment_consumptions c USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind=5 AND l.quantity_after=0 AND l.item_instance_id=OLD.item_instance_id AND l.source_equipment_slot=OLD.slot
       AND l.source_equipment_revision=c.revision_before AND c.character_id=OLD.character_id AND r.character_id=OLD.character_id
       AND r.world_id=OLD.world_id AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=l.transaction_id
       AND i.state_revision=l.state_revision_before+1 AND l.created_xact_id=pg_current_xact_id()
       AND r.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'equipment slot updates require delete/insert' USING ERRCODE='23514'; END IF;
    IF TG_OP='INSERT' THEN
        IF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
            WHERE r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
              AND r.item_instance_id=NEW.item_instance_id AND r.to_slot=NEW.slot AND r.operation=1
              AND r.world_id=NEW.world_id AND i.world_id=NEW.world_id AND i.lifecycle=1
              AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
              AND r.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'equipment insertion lacks actual movement receipt' USING ERRCODE='23514';
        END IF;
    ELSIF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r
        WHERE r.character_id=OLD.character_id AND r.item_instance_id=OLD.item_instance_id
          AND r.from_slot=OLD.slot AND r.operation=1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipment removal lacks actual movement receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION game_equipment_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=5 AND p.source_kind=3
     AND p.character_id=NEW.character_id AND p.transaction_id=NEW.last_transaction_id
     AND p.equipment_revision_before=OLD.revision AND NEW.revision=OLD.revision+1
     AND OLD.combat_mode IS NOT DISTINCT FROM NEW.combat_mode AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;

    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
      WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id AND r.custody_kind=3
        AND r.equipment_revision_before=OLD.revision AND NEW.revision=OLD.revision+1
        AND NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_equipment_consumptions r WHERE r.transaction_id=NEW.last_transaction_id
      AND r.character_id=NEW.character_id AND r.revision_before=OLD.revision AND r.revision_after=NEW.revision
      AND OLD.combat_mode IS NOT DISTINCT FROM NEW.combat_mode AND r.combat_mode IS NOT DISTINCT FROM NEW.combat_mode
      AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r
        WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id
          AND r.revision_after=NEW.revision AND r.revision_before=OLD.revision
          AND r.mode_before IS NOT DISTINCT FROM OLD.combat_mode
          AND r.mode_after IS NOT DISTINCT FROM NEW.combat_mode
          AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipment successor lacks same-transaction receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

-- Evidence tables are private. Trigger functions read them as their migration owner;
-- RETURNS trigger functions cannot be invoked as ordinary SQL mutation interfaces.
DO $$ DECLARE n TEXT;BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_shape_immutable','game_house_shape_proven','game_house_location_proven','game_house_container_slot_removal_proven',
 'game_item_instance_guard','game_item_instance_change_proven','game_item_transfer_consistency_guard',
 'game_item_container_entry_removal_proven','game_item_placement_proven','game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SECURITY DEFINER',n);
  EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_shape_complete(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_item_apply(SMALLINT,JSONB,UUID,NUMERIC,BYTEA) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_tile(UUID,TEXT,BYTEA) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 FOREACH n IN ARRAY ARRAY['game_house_item_place','game_house_item_take','game_house_item_move'] LOOP
  EXECUTE format('ALTER FUNCTION %I(JSONB,UUID,NUMERIC,BYTEA) SET search_path = pg_catalog, %I, pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I(JSONB,UUID,NUMERIC,BYTEA) FROM PUBLIC',n);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %I(JSONB,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime',n);
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_house_shape_complete(UUID),game_house_item_apply(SMALLINT,JSONB,UUID,NUMERIC,BYTEA) FROM PUBLIC;
-- The existing admission access EXECUTE and all earlier grant/guard bodies remain unchanged.
