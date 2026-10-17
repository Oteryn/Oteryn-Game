-- PRIVATE DEPOT0 persistence proposal, applied only by the native PG tests.
-- This file is deliberately outside migrations/. It is not a deployment, an
-- allocation receipt, a Content provider or a current-locker admission API.
-- Compose after the exact 0113 tree recorded in BEFORE_CODE.md. No runtime or
-- control role receives access to the new table, evidence or physical primitive.
-- A future admitted consumer must run the real Character/node/session fence,
-- current locker/open-view and Content/capacity checks before this primitive.

CREATE TABLE game_item_character_depot_locations (
 item_instance_id UUID PRIMARY KEY,
 world_id UUID NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 box SMALLINT NOT NULL CHECK(box BETWEEN 1 AND 17),
 ordinal NUMERIC(20,0) NOT NULL CHECK(ordinal BETWEEN 1 AND 18446744073709551615),
 placed_transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(placed_transaction_id)),
 UNIQUE(character_id,box,ordinal),
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id)
);
REVOKE ALL ON game_item_character_depot_locations FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

ALTER TABLE game_item_transfer_reservations
 DROP CONSTRAINT game_item_transfer_reservations_source_kind_check,
 DROP CONSTRAINT game_item_transfer_reservations_destination_kind_check,
 DROP CONSTRAINT game_item_transfer_house_scope,
 ADD CONSTRAINT game_item_transfer_reservations_source_kind_check CHECK(source_kind BETWEEN 0 AND 6),
 ADD CONSTRAINT game_item_transfer_reservations_destination_kind_check CHECK(destination_kind BETWEEN 1 AND 6),
 ADD COLUMN depot_intent JSONB,
 ADD CONSTRAINT game_item_transfer_house_scope CHECK(
 (source_kind=0 AND channel_id IS NOT NULL AND house_key IS NULL AND house_scope_generation IS NULL AND house_intent IS NULL AND destination_kind IN(1,2) AND depot_intent IS NULL)
 OR(source_kind BETWEEN 1 AND 4 AND channel_id IS NULL AND house_key IS NOT NULL AND house_scope_generation IS NOT NULL AND game_house_scope_key_valid(house_key)
    AND house_intent IS NOT NULL AND jsonb_typeof(house_intent)='object' AND octet_length(house_intent::text)<=16384
    AND house_scope_generation BETWEEN 1 AND 18446744073709551615 AND destination_kind IN(3,4,5) AND depot_intent IS NULL)
 OR(source_kind IN(5,6) AND channel_id IS NULL AND house_key IS NULL AND house_scope_generation IS NULL AND house_intent IS NULL
    AND ((source_kind=5 AND destination_kind=6) OR(source_kind=6 AND destination_kind IN(1,2)))
    AND depot_intent IS NOT NULL AND jsonb_typeof(depot_intent)='object' AND octet_length(depot_intent::text)<=4096)),
 ADD CONSTRAINT game_item_transfer_depot_intent_binding CHECK(
  depot_intent IS NULL OR intent_binding=decode('03','hex')||sha256(convert_to(depot_intent::text,'UTF8')));

ALTER TABLE game_item_transfer_receipts
 DROP CONSTRAINT game_item_transfer_receipts_shape_check,
 DROP CONSTRAINT game_item_transfer_shape_conservation,
 ADD CONSTRAINT game_item_transfer_receipts_shape_check CHECK(shape BETWEEN 1 AND 9),
 ADD COLUMN depot_box SMALLINT,
 ADD COLUMN depot_ordinal NUMERIC(20,0),
 ADD CONSTRAINT game_item_transfer_depot_position CHECK(
  (shape BETWEEN 1 AND 7 AND depot_box IS NULL AND depot_ordinal IS NULL)
  OR(shape IN(8,9) AND depot_box IS NOT NULL AND depot_box BETWEEN 1 AND 17
     AND depot_ordinal IS NOT NULL AND depot_ordinal BETWEEN 1 AND 18446744073709551615)),
 ADD CONSTRAINT game_item_transfer_shape_conservation CHECK(CASE shape
 WHEN 1 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NULL
             AND source_quantity_after=source_quantity_before
 WHEN 2 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NOT NULL
             AND source_quantity_after=source_quantity_before
 WHEN 3 THEN receiver_item_instance_id IS NOT NULL AND destination_ordinal IS NULL
             AND source_quantity_after=0 AND receiver_quantity_after=receiver_quantity_before+source_quantity_before
 WHEN 4 THEN receiver_item_instance_id IS NOT NULL AND destination_ordinal IS NOT NULL
             AND source_quantity_after>0 AND receiver_quantity_after>receiver_quantity_before
             AND receiver_quantity_after-receiver_quantity_before=source_quantity_before-source_quantity_after
 WHEN 8 THEN receiver_item_instance_id IS NULL AND destination_ordinal IS NULL
             AND source_quantity_after=source_quantity_before
 WHEN 9 THEN receiver_item_instance_id IS NULL AND source_quantity_after=source_quantity_before
 ELSE receiver_item_instance_id IS NULL AND source_quantity_after=source_quantity_before
      AND ((shape=6)=(destination_ordinal IS NOT NULL)) END);

-- Only the physical primitive captures these before facts from locked native
-- rows. The runtime cannot insert evidence or choose a claimed old quantity.
CREATE TABLE game_item_depot_shape_proofs (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_transfer_receipts(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 event_id UUID NOT NULL UNIQUE,
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL,world_id UUID NOT NULL,item_instance_id UUID NOT NULL,
 shape SMALLINT NOT NULL CHECK(shape IN(8,9)),source_kind SMALLINT NOT NULL CHECK(source_kind IN(5,6)),
 destination_kind SMALLINT NOT NULL CHECK(destination_kind IN(1,2,6)),
 quantity_before BIGINT NOT NULL CHECK(quantity_before BETWEEN 1 AND 100),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 1 AND 18446744073709551614),
 definition_family TEXT NOT NULL,definition_production_key TEXT NOT NULL,definition_revision_ref TEXT NOT NULL,
 key_number BIGINT NOT NULL,
 source_parent UUID,source_ordinal NUMERIC(20,0) NOT NULL,source_placement UUID NOT NULL,
 destination_parent UUID,destination_ordinal NUMERIC(20,0),
 depot_box SMALLINT NOT NULL CHECK(depot_box BETWEEN 1 AND 17),
 depot_ordinal NUMERIC(20,0) NOT NULL CHECK(depot_ordinal BETWEEN 1 AND 18446744073709551615),
 actor_channel UUID NOT NULL,connection_generation NUMERIC(20,0) NOT NULL,
 lease_generation NUMERIC(20,0) NOT NULL,scope_generation NUMERIC(20,0) NOT NULL,
 character_revision_before NUMERIC(20,0) NOT NULL,
 content_revision TEXT NOT NULL,ruleset_revision TEXT NOT NULL,sim_revision TEXT NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((shape=8 AND source_kind=5 AND destination_kind=6 AND source_parent IS NOT NULL
              AND destination_parent IS NULL AND destination_ordinal IS NULL)
    OR(shape=9 AND source_kind=6 AND source_parent IS NULL AND destination_kind IN(1,2)
              AND ((destination_kind=1 AND destination_parent IS NULL AND destination_ordinal IS NULL)
                 OR(destination_kind=2 AND destination_parent IS NOT NULL AND destination_ordinal IS NOT NULL))))
);
CREATE TABLE game_item_depot_removal_evidence (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_depot_shape_proofs(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 item_instance_id UUID NOT NULL,world_id UUID NOT NULL,character_id UUID NOT NULL,
 box SMALLINT NOT NULL,ordinal NUMERIC(20,0) NOT NULL,placed_transaction_id UUID NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_item_depot_shape_proofs,game_item_depot_removal_evidence
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_depot_proof_stamp BEFORE INSERT ON game_item_depot_shape_proofs
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_depot_removal_stamp BEFORE INSERT ON game_item_depot_removal_evidence
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_depot_proof_immutable BEFORE UPDATE OR DELETE ON game_item_depot_shape_proofs
 FOR EACH ROW EXECUTE FUNCTION game_house_shape_immutable();
CREATE TRIGGER game_depot_removal_immutable BEFORE UPDATE OR DELETE ON game_item_depot_removal_evidence
 FOR EACH ROW EXECUTE FUNCTION game_house_shape_immutable();
CREATE TRIGGER game_depot_proof_no_truncate BEFORE TRUNCATE ON game_item_depot_shape_proofs
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_depot_removal_no_truncate BEFORE TRUNCATE ON game_item_depot_removal_evidence
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_depot_no_truncate BEFORE TRUNCATE ON game_item_character_depot_locations
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_depot_shape_complete(p_tx UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_item_depot_shape_proofs%ROWTYPE;r game_item_transfer_receipts%ROWTYPE;
 i game_item_instances%ROWTYPE;v game_item_transfer_reservations%ROWTYPE;
BEGIN
 SELECT * INTO p FROM game_item_depot_shape_proofs WHERE transaction_id=p_tx;
 IF NOT FOUND OR p.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'depot proof not current physical transaction' USING ERRCODE='23514';END IF;
 SELECT * INTO r FROM game_item_transfer_receipts WHERE transaction_id=p_tx;
 IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id()
  OR(r.game_session_id,r.command_id,r.character_id,r.event_id,r.shape,r.source_item_instance_id,r.source_quantity_before,r.source_quantity_after)
   IS DISTINCT FROM(p.game_session_id,p.command_id,p.character_id,p.event_id,p.shape,p.item_instance_id,p.quantity_before,p.quantity_before)
  OR r.receiver_item_instance_id IS NOT NULL OR r.receiver_quantity_before IS NOT NULL OR r.receiver_quantity_after IS NOT NULL
  OR(r.destination_parent_item_instance_id,r.destination_ordinal,r.depot_box,r.depot_ordinal)
   IS DISTINCT FROM(p.destination_parent,p.destination_ordinal,p.depot_box,p.depot_ordinal)
 THEN RAISE EXCEPTION 'depot receipt not exact shape' USING ERRCODE='23514';END IF;
 SELECT * INTO v FROM game_item_transfer_reservations WHERE transaction_id=p_tx;
 IF NOT FOUND OR(v.game_session_id,v.command_id,v.character_id,v.world_id,v.source_item_instance_id,v.event_id,v.occurred_at,v.intent_binding)
   IS DISTINCT FROM(r.game_session_id,r.command_id,r.character_id,p.world_id,r.source_item_instance_id,r.event_id,r.occurred_at,r.intent_binding)
  OR v.channel_id IS NOT NULL OR v.source_kind<>p.source_kind OR v.destination_kind<>p.destination_kind
  OR v.work_units_used NOT BETWEEN 1 AND 3
  OR(v.depot_intent->>'box')::smallint IS DISTINCT FROM p.depot_box
  OR(v.depot_intent->>'content_revision',v.depot_intent->>'ruleset_revision',v.depot_intent->>'sim_revision')
    IS DISTINCT FROM(p.content_revision,p.ruleset_revision,p.sim_revision)
 THEN RAISE EXCEPTION 'depot receipt lacks exact reservation' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_tx AND a.event_id=p.event_id
  AND a.item_instance_id=p.item_instance_id AND a.created_xact_id=pg_current_xact_id()
  AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
  AND a.event_type_id=2 AND a.transaction_count=1 AND a.transaction_ordinal=1
  AND a.publication_state=1 AND a.published_at IS NULL;
 IF NOT FOUND THEN RAISE EXCEPTION 'depot receipt lacks same-transaction audit' USING ERRCODE='23514';END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=p.item_instance_id;
 IF NOT FOUND OR i.lifecycle<>1 OR i.world_id<>p.world_id OR i.quantity<>p.quantity_before
  OR i.last_transaction_id IS DISTINCT FROM p_tx OR i.state_revision<>p.state_revision_before+1
  OR(i.definition_family,i.definition_production_key,i.definition_revision_ref)
    IS DISTINCT FROM(p.definition_family,p.definition_production_key,p.definition_revision_ref)
  OR coalesce((SELECT key_number FROM game_item_keys WHERE item_instance_id=p.item_instance_id),0)<>p.key_number
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q
    WHERE q.item_instance_id=p.item_instance_id AND q.transaction_id=p_tx AND q.quantity_before=p.quantity_before)
 THEN RAISE EXCEPTION 'depot item differs from actual before evidence' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_character_roots cr WHERE cr.character_id=p.character_id AND cr.world_id=p.world_id
  AND cr.lifecycle=1 AND cr.character_revision=p.character_revision_before FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'depot item move changed CharacterRevision or World' USING ERRCODE='23514';END IF;
 IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=p.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p.item_instance_id)
 THEN RAISE EXCEPTION 'depot item has contents' USING ERRCODE='23514';END IF;
 IF p.shape=8 THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_character_depot_locations d WHERE d.item_instance_id=p.item_instance_id
    AND d.world_id=p.world_id AND d.character_id=p.character_id AND d.box=p.depot_box
    AND d.ordinal=p.depot_ordinal AND d.placed_transaction_id=p_tx)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_depot_removal_evidence WHERE transaction_id=p_tx)
  THEN RAISE EXCEPTION 'depot deposit lacks exact destination or source removal' USING ERRCODE='23514';END IF;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_item_depot_removal_evidence e
    WHERE e.transaction_id=p_tx AND e.item_instance_id=p.item_instance_id AND e.world_id=p.world_id
     AND e.character_id=p.character_id AND e.box=p.depot_box AND e.ordinal=p.source_ordinal
     AND e.placed_transaction_id=p.source_placement AND e.created_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id=p.item_instance_id)
  THEN RAISE EXCEPTION 'depot withdrawal lacks physical removal evidence' USING ERRCODE='23514';END IF;
  IF p.destination_kind=1 THEN
   IF NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=p.item_instance_id
     AND s.character_id=p.character_id AND s.world_id=p.world_id AND s.placed_transaction_id=p_tx)
   THEN RAISE EXCEPTION 'depot withdrawal lacks exact container slot' USING ERRCODE='23514';END IF;
  ELSE
   IF NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
     ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id AND s.world_id=e.world_id
     WHERE e.item_instance_id=p.item_instance_id AND e.character_id=p.character_id AND e.world_id=p.world_id
      AND e.parent_item_instance_id=p.destination_parent AND e.placement_ordinal=p.destination_ordinal AND e.placed_transaction_id=p_tx)
    OR(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=p.destination_parent)>20
   THEN RAISE EXCEPTION 'depot withdrawal lacks bounded main-backpack entry' USING ERRCODE='23514';END IF;
  END IF;
 END IF;
 IF(SELECT count(*) FROM game_item_character_depot_locations WHERE character_id=p.character_id)>15000
 THEN RAISE EXCEPTION 'depot absolute entry ceiling exceeded' USING ERRCODE='23514';END IF;
 PERFORM game_item_location_exclusive(p.item_instance_id);
END $$;

CREATE FUNCTION game_depot_shape_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_depot_shape_complete(NEW.transaction_id);RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER game_depot_shape_proven AFTER INSERT ON game_item_depot_shape_proofs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_depot_shape_proven();

-- DELETE evidence comes from OLD inside a protected trigger, never from a
-- raw command or a historical receipt. Updates/renumbering remain forbidden.
CREATE FUNCTION game_depot_location_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_item_depot_shape_proofs%ROWTYPE;
BEGIN
 IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'depot location is immutable' USING ERRCODE='23514';END IF;
 IF TG_OP='DELETE' THEN
  SELECT * INTO p FROM game_item_depot_shape_proofs q WHERE q.shape=9
   AND q.item_instance_id=OLD.item_instance_id AND q.world_id=OLD.world_id AND q.character_id=OLD.character_id
   AND q.depot_box=OLD.box AND q.source_ordinal=OLD.ordinal AND q.source_placement=OLD.placed_transaction_id
   AND q.created_xact_id=pg_current_xact_id();
  IF NOT FOUND THEN RAISE EXCEPTION 'depot removal lacks own actual source' USING ERRCODE='23514';END IF;
  INSERT INTO game_item_depot_removal_evidence(transaction_id,item_instance_id,world_id,character_id,box,ordinal,placed_transaction_id)
   VALUES(p.transaction_id,OLD.item_instance_id,OLD.world_id,OLD.character_id,OLD.box,OLD.ordinal,OLD.placed_transaction_id);
  RETURN OLD;
 END IF;
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=NEW.world_id AND lifecycle=1 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'depot Character and item World differ' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_depot_location_guard BEFORE INSERT OR UPDATE OR DELETE ON game_item_character_depot_locations
 FOR EACH ROW EXECUTE FUNCTION game_depot_location_guard();
CREATE FUNCTION game_depot_location_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p_tx UUID;
BEGIN
 IF TG_OP='INSERT' THEN
  SELECT p.transaction_id INTO p_tx FROM game_item_depot_shape_proofs p WHERE p.shape=8
   AND p.transaction_id=NEW.placed_transaction_id AND p.item_instance_id=NEW.item_instance_id
   AND p.world_id=NEW.world_id AND p.character_id=NEW.character_id AND p.depot_box=NEW.box AND p.depot_ordinal=NEW.ordinal
   AND p.created_xact_id=pg_current_xact_id();
 ELSE
  SELECT e.transaction_id INTO p_tx FROM game_item_depot_removal_evidence e WHERE e.item_instance_id=OLD.item_instance_id
   AND e.world_id=OLD.world_id AND e.character_id=OLD.character_id AND e.box=OLD.box AND e.ordinal=OLD.ordinal
   AND e.placed_transaction_id=OLD.placed_transaction_id AND e.created_xact_id=pg_current_xact_id();
 END IF;
 IF p_tx IS NULL THEN RAISE EXCEPTION 'depot custody change lacks own physical proof' USING ERRCODE='23514';END IF;
 PERFORM game_depot_shape_complete(p_tx);RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_depot_location_proven AFTER INSERT OR DELETE ON game_item_character_depot_locations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_depot_location_proven();

-- Physical persistence primitive only. The test caller separately acquires the
-- real native node/session/admission fence with typed native values. This SQL
-- also checks actual session/root facts, but does not grant locker access,
-- decide Premium limits, infer Content admission, or substitute for CAP lookup.
CREATE FUNCTION game_depot_proposal_apply(p_session UUID,p_command NUMERIC,p_channel UUID,
 p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC) RETURNS JSONB
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE v game_item_transfer_reservations%ROWTYPE;r game_character_roots%ROWTYPE;
 s game_durability_reconnect_sessions%ROWTYPE;i game_item_instances%ROWTYPE;
 p game_item_depot_shape_proofs%ROWTYPE;c game_item_container_entries%ROWTYPE;
 d game_item_character_depot_locations%ROWTYPE;slot game_item_container_slots%ROWTYPE;
 k TEXT;j JSONB;receipt game_item_transfer_receipts%ROWTYPE;
BEGIN
 SELECT * INTO v FROM game_item_transfer_reservations WHERE game_session_id=p_session AND command_id=p_command FOR UPDATE;
 IF NOT FOUND OR v.source_kind NOT IN(5,6) OR v.depot_intent IS NULL
  OR v.work_units_used NOT BETWEEN 1 AND 3 THEN RAISE EXCEPTION 'DEPOT_OCCURRENCE_NOT_RESERVED';END IF;
 j:=v.depot_intent;
 IF NOT j ?& ARRAY['character','world','item','source_kind','destination_kind','box','content_revision','ruleset_revision','sim_revision','weight_generation_digest']
  OR (SELECT count(*) FROM jsonb_object_keys(j))<>10
  OR EXISTS(SELECT 1 FROM jsonb_each(j) x WHERE x.value='null'::jsonb)
  OR(j->>'character')::uuid<>v.character_id OR(j->>'world')::uuid<>v.world_id
  OR(j->>'item')::uuid<>v.source_item_instance_id OR(j->>'source_kind')::smallint<>v.source_kind
  OR(j->>'destination_kind')::smallint<>v.destination_kind OR(j->>'box')::smallint NOT BETWEEN 1 AND 17
  OR j->>'weight_generation_digest' !~ '^[0-9a-f]{64}$'
 THEN RAISE EXCEPTION 'DEPOT_INVALID_INTENT';END IF;
 FOREACH k IN ARRAY ARRAY['content_revision','ruleset_revision','sim_revision'] LOOP
  IF jsonb_typeof(j->k)<>'string' OR octet_length(j->>k) NOT BETWEEN 1 AND 512 THEN
   RAISE EXCEPTION 'DEPOT_INVALID_INTENT';END IF;
 END LOOP;
 SELECT * INTO receipt FROM game_item_transfer_receipts WHERE transaction_id=v.transaction_id;
 IF FOUND THEN
  IF(receipt.game_session_id,receipt.command_id,receipt.character_id,receipt.intent_binding,receipt.event_id,receipt.source_item_instance_id)
    IS DISTINCT FROM(v.game_session_id,v.command_id,v.character_id,v.intent_binding,v.event_id,v.source_item_instance_id)
   OR receipt.shape<>(CASE WHEN v.source_kind=5 THEN 8 ELSE 9 END) THEN RAISE EXCEPTION 'DEPOT_RECEIPT_CONFLICT';END IF;
  RETURN jsonb_build_object('replayed',true,'transaction_id',v.transaction_id);
 END IF;
 SELECT * INTO s FROM game_durability_reconnect_sessions WHERE game_session_id=v.game_session_id
  AND character_id=v.character_id AND world_id=v.world_id AND runtime_scope_kind=1
  AND runtime_scope_world_id=v.world_id AND runtime_scope_channel_id=p_channel AND runtime_scope_instance_id IS NULL
  AND current_generation=p_connection AND character_lease_generation=p_lease AND scope_ownership_generation=p_scope
  AND session_state IN(1,2) FOR SHARE;
 IF NOT FOUND OR p_connection NOT BETWEEN 1 AND 18446744073709551615
  OR p_lease NOT BETWEEN 1 AND 18446744073709551615 OR p_scope NOT BETWEEN 1 AND 18446744073709551615
 THEN RAISE EXCEPTION 'DEPOT_NOT_CURRENT';END IF;
 SELECT * INTO r FROM game_character_roots WHERE character_id=v.character_id AND account_id=s.account_id
  AND world_id=v.world_id AND lifecycle=1 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'DEPOT_NOT_CURRENT';END IF;
 IF r.content_revision IS DISTINCT FROM j->>'content_revision' OR r.ruleset_revision IS DISTINCT FROM j->>'ruleset_revision'
 THEN RAISE EXCEPTION 'DEPOT_CONTENT_CHANGED';END IF;
 -- Character-root lock serializes count and max+1 even when a box has no rows.
 -- Items follow in UUID order; only afterwards is the inventory slot locked.
 PERFORM 1 FROM game_item_instances WHERE item_instance_id=v.source_item_instance_id
  OR item_instance_id IN(SELECT item_instance_id FROM game_item_container_slots WHERE character_id=v.character_id)
  ORDER BY item_instance_id FOR UPDATE;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=v.source_item_instance_id AND world_id=v.world_id AND lifecycle=1;
 IF NOT FOUND OR i.quantity NOT BETWEEN 1 AND 100 OR i.state_revision NOT BETWEEN 1 AND 18446744073709551614
  OR EXISTS(SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=i.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION')
 THEN RAISE EXCEPTION 'DEPOT_SOURCE_CHANGED';END IF;
 IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'DEPOT_ITEM_HAS_CONTENTS';END IF;
 PERFORM game_item_location_exclusive(i.item_instance_id);
 SELECT * INTO slot FROM game_item_container_slots WHERE character_id=v.character_id FOR UPDATE;
 p.transaction_id:=v.transaction_id;p.event_id:=v.event_id;p.game_session_id:=v.game_session_id;p.command_id:=v.command_id;
 p.character_id:=v.character_id;p.world_id:=v.world_id;p.item_instance_id:=i.item_instance_id;
 p.shape:=CASE WHEN v.source_kind=5 THEN 8 ELSE 9 END;p.source_kind:=v.source_kind;p.destination_kind:=v.destination_kind;
 p.quantity_before:=i.quantity;p.state_revision_before:=i.state_revision;
 p.definition_family:=i.definition_family;p.definition_production_key:=i.definition_production_key;p.definition_revision_ref:=i.definition_revision_ref;
 p.key_number:=coalesce((SELECT key_number FROM game_item_keys WHERE item_instance_id=i.item_instance_id),0);
 p.actor_channel:=p_channel;p.connection_generation:=p_connection;p.lease_generation:=p_lease;p.scope_generation:=p_scope;
 p.character_revision_before:=r.character_revision;p.content_revision:=r.content_revision;p.ruleset_revision:=r.ruleset_revision;
 p.sim_revision:=j->>'sim_revision';p.depot_box:=(j->>'box')::smallint;p.created_xact_id:=pg_current_xact_id();
 IF p.shape=8 THEN
  SELECT * INTO c FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND character_id=v.character_id
   AND world_id=v.world_id AND parent_item_instance_id=slot.item_instance_id FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'DEPOT_SOURCE_NOT_DIRECT_BACKPACK';END IF;
  IF(SELECT count(*) FROM game_item_character_depot_locations WHERE character_id=v.character_id)>=15000
  THEN RAISE EXCEPTION 'DEPOT_HARD_CAP';END IF;
  SELECT coalesce(max(q.ordinal),0)+1 INTO p.depot_ordinal FROM game_item_character_depot_locations q
   WHERE q.character_id=v.character_id AND q.box=p.depot_box;
  IF p.depot_ordinal>18446744073709551615 THEN RAISE EXCEPTION 'DEPOT_ORDINAL_OVERFLOW';END IF;
  p.source_parent:=c.parent_item_instance_id;p.source_ordinal:=c.placement_ordinal;p.source_placement:=c.placed_transaction_id;
 ELSE
  SELECT * INTO d FROM game_item_character_depot_locations WHERE item_instance_id=i.item_instance_id
   AND character_id=v.character_id AND world_id=v.world_id AND box=p.depot_box FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'DEPOT_SOURCE_NOT_OWN_BOX';END IF;
  p.source_ordinal:=d.ordinal;p.source_placement:=d.placed_transaction_id;p.depot_ordinal:=d.ordinal;
  IF p.destination_kind=1 THEN
   IF slot.item_instance_id IS NOT NULL THEN RAISE EXCEPTION 'DEPOT_CONTAINER_SLOT_OCCUPIED';END IF;
  ELSE
   IF slot.item_instance_id IS NULL OR slot.world_id<>v.world_id OR slot.item_instance_id=i.item_instance_id
    OR NOT EXISTS(SELECT 1 FROM game_item_instances parent WHERE parent.item_instance_id=slot.item_instance_id AND parent.lifecycle=1)
   THEN RAISE EXCEPTION 'DEPOT_NO_MAIN_BACKPACK';END IF;
   IF(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=slot.item_instance_id)>=20
   THEN RAISE EXCEPTION 'DEPOT_BACKPACK_FULL';END IF;
   p.destination_parent:=slot.item_instance_id;
   SELECT coalesce(max(e.placement_ordinal),0)+1 INTO p.destination_ordinal FROM game_item_container_entries e
    WHERE e.parent_item_instance_id=slot.item_instance_id;
   IF p.destination_ordinal>18446744073709551615 THEN RAISE EXCEPTION 'DEPOT_BACKPACK_ORDINAL_OVERFLOW';END IF;
  END IF;
 END IF;
 INSERT INTO game_item_depot_shape_proofs SELECT(p).*;
 IF p.shape=8 THEN
  DELETE FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id;
  INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,box,ordinal,placed_transaction_id)
   VALUES(i.item_instance_id,v.world_id,v.character_id,p.depot_box,p.depot_ordinal,p.transaction_id);
 ELSE
  DELETE FROM game_item_character_depot_locations WHERE item_instance_id=i.item_instance_id;
  IF p.destination_kind=1 THEN
   INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id)
    VALUES(v.character_id,i.item_instance_id,v.world_id,p.transaction_id);
  ELSE
   INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
    VALUES(i.item_instance_id,v.world_id,v.character_id,p.destination_parent,p.destination_ordinal,p.transaction_id);
  END IF;
 END IF;
 UPDATE game_item_instances SET last_transaction_id=p.transaction_id,state_revision=p.state_revision_before+1
  WHERE item_instance_id=i.item_instance_id;
 RETURN to_jsonb(p)||jsonb_build_object('replayed',false,'occurred_at',v.occurred_at);
END $$;

-- Complete native function replacements are appended below. Every preceding
-- House, equipment-swap, timed, trade, spell, bank and other proof arm remains.
-- BEGIN FULL NATIVE PREDECESSORS
CREATE OR REPLACE FUNCTION game_item_instance_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- DEPOT0: immutable whole-item state, actual OLD evidence, same transaction.
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_depot_shape_proofs p
     WHERE p.transaction_id=NEW.last_transaction_id AND p.item_instance_id=OLD.item_instance_id
      AND p.world_id=OLD.world_id AND p.quantity_before=OLD.quantity
      AND p.state_revision_before=OLD.state_revision AND p.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.last_transaction_id IS NOT DISTINCT FROM OLD.last_transaction_id
      OR NEW.state_revision<>OLD.state_revision+1
      OR(to_jsonb(NEW)-'last_transaction_id'-'state_revision')
        IS DISTINCT FROM(to_jsonb(OLD)-'last_transaction_id'-'state_revision') THEN
      RAISE EXCEPTION 'depot transfer preserves item state' USING ERRCODE='23514';END IF;
     INSERT INTO game_item_transfer_quantity_evidence VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
     RETURN NEW;
    END IF;

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
    IF EXISTS(SELECT 1 FROM game_item_depot_shape_proofs p
     WHERE p.transaction_id=NEW.last_transaction_id AND p.item_instance_id=NEW.item_instance_id
      AND p.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_depot_shape_complete(NEW.last_transaction_id);RETURN NULL;END IF;

    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.last_transaction_id
     AND p.item_instance_id=NEW.item_instance_id AND p.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_house_shape_complete(NEW.last_transaction_id);RETURN NULL;END IF;

    -- Same physical transaction's second whole item in the equipment swap.
    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       WHERE r.operation=1 AND r.swap_item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.last_transaction_id AND r.swap_state_revision_before=OLD.state_revision
         AND NEW.state_revision=OLD.state_revision+1 AND NEW.quantity=OLD.quantity
         AND NEW.lifecycle=OLD.lifecycle AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
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

CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_item_depot_shape_proofs p
      WHERE p.shape=8 AND p.source_kind=5 AND p.item_instance_id=OLD.item_instance_id
       AND p.character_id=OLD.character_id AND p.world_id=OLD.world_id
       AND p.source_parent=OLD.parent_item_instance_id AND p.source_ordinal=OLD.placement_ordinal
       AND p.source_placement=OLD.placed_transaction_id AND p.created_xact_id=pg_current_xact_id()) THEN
      RETURN NULL;
     END IF;
    END IF;

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
    -- Separate PL/pgSQL table branches: slot records have no entry fields.
    IF TG_TABLE_NAME='game_item_container_slots' THEN
     IF EXISTS(SELECT 1 FROM game_item_depot_shape_proofs p WHERE p.shape=9 AND p.destination_kind=1
      AND p.transaction_id=NEW.placed_transaction_id AND p.item_instance_id=NEW.item_instance_id
      AND p.character_id=NEW.character_id AND p.world_id=NEW.world_id
      AND p.created_xact_id=pg_current_xact_id()) THEN
      PERFORM game_depot_shape_complete(NEW.placed_transaction_id);RETURN NULL;END IF;
    ELSIF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_item_depot_shape_proofs p WHERE p.shape=9 AND p.destination_kind=2
      AND p.transaction_id=NEW.placed_transaction_id AND p.item_instance_id=NEW.item_instance_id
      AND p.character_id=NEW.character_id AND p.world_id=NEW.world_id
      AND p.destination_parent=NEW.parent_item_instance_id AND p.destination_ordinal=NEW.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN
      PERFORM game_depot_shape_complete(NEW.placed_transaction_id);RETURN NULL;END IF;
    END IF;

    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=6 AND p.transaction_id=NEW.placed_transaction_id
      AND p.item_instance_id=NEW.item_instance_id AND p.world_id=NEW.world_id AND p.character_id=NEW.character_id
      AND p.destination_parent=NEW.parent_item_instance_id AND p.destination_ordinal=NEW.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    -- Guard record-field access before preparing the entry-only swap expression:
    -- the same trigger also receives slot rows, which have no parent or ordinal.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(
       SELECT 1 FROM game_character_equipment_receipts r
       JOIN game_item_instances i ON i.item_instance_id=r.swap_item_instance_id
       WHERE r.operation=1 AND r.swap_item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
         AND r.backpack_item_instance_id=NEW.parent_item_instance_id
         AND r.swap_backpack_ordinal=NEW.placement_ordinal AND r.world_id=NEW.world_id
         AND i.lifecycle=1 AND i.world_id=NEW.world_id AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.swap_state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'equipment swap backpack entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
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
    IF NEW.shape IN(8,9) THEN PERFORM game_depot_shape_complete(NEW.transaction_id);RETURN NULL;END IF;

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
         + (SELECT count(*) FROM game_item_character_depot_locations d
             WHERE d.item_instance_id = src.item_instance_id)
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

CREATE OR REPLACE FUNCTION game_item_location_exclusive(p_item UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_lifecycle SMALLINT;
    v_locations BIGINT;
BEGIN
    -- The row lock serializes two transactions that each place the same item
    -- in a different location table: the later one waits here and then
    -- counts the earlier one's committed row. Every existing writer already
    -- holds this lock from its own item UPDATE.
    SELECT lifecycle INTO v_lifecycle FROM game_item_instances
     WHERE item_instance_id = p_item FOR NO KEY UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    v_locations :=
          (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id = p_item);
    IF v_locations <> (v_lifecycle = 1)::INT THEN
        RAISE EXCEPTION 'DUR-03 item must have exactly one location while live and none when retired'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'HouseInterior item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterInbox item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterDepot item cannot have contents' USING ERRCODE = '23514';
    END IF;
END;
$$;

-- Preserve each prior native function's protected execution context. New
-- functions are private, including the ordinary SQL physical primitive.
DO $$ DECLARE n TEXT;BEGIN
 FOREACH n IN ARRAY ARRAY['game_depot_shape_proven','game_depot_location_guard','game_depot_location_proven',
 'game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven',
 'game_item_placement_proven','game_item_transfer_consistency_guard'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SECURITY DEFINER',n);
  EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
  IF n LIKE 'game_depot_%' THEN
   EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM oteryn_game_runtime,oteryn_game_control',n);
  END IF;
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_depot_shape_complete(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_depot_proposal_apply(UUID,NUMERIC,UUID,NUMERIC,NUMERIC,NUMERIC) SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_depot_shape_complete(UUID),
 game_depot_proposal_apply(UUID,NUMERIC,UUID,NUMERIC,NUMERIC,NUMERIC)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
