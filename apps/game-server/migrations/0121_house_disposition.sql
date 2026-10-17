-- HOUSE-1b canonical disposition producer and native custody removal proof.
-- D1382 / D1384: one installed schema for native qualification and production.
-- 0095 stays unchanged; its exact player INSERT/DELETE arms are retained below.
-- Schema installation grants no direct runtime/control custody mutation authority.

CREATE TABLE game_house_dispositions (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL, house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision BETWEEN 1 AND 18446744073709551615),
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 command_binding BYTEA NOT NULL CHECK(octet_length(command_binding) BETWEEN 1 AND 2048),
 cause TEXT NOT NULL DEFAULT 'MOVE_OUT' CHECK(cause='MOVE_OUT'),
 requested_at_ms BIGINT NOT NULL,
 due_at_ms BIGINT NOT NULL CHECK(due_at_ms::numeric-requested_at_ms::numeric BETWEEN 86400000 AND 2592000000),
 state SMALLINT NOT NULL DEFAULT 1 CHECK(state IN(1,2,3,4)),
 cancel_session_id UUID,cancel_command_id NUMERIC(20,0),cancel_binding BYTEA,cancelled_at_ms BIGINT,
 frozen_count BIGINT CHECK(frozen_count BETWEEN 0 AND 2000),frozen_digest BYTEA CHECK(octet_length(frozen_digest)=32),
 next_step NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(next_step BETWEEN 0 AND 18446744073709551615),
 UNIQUE(game_session_id,command_id),UNIQUE(cancel_session_id,cancel_command_id),
 CHECK((state IN(1,4))=(frozen_count IS NULL)),CHECK((state IN(1,4))=(frozen_digest IS NULL)),
 CHECK((state=4)=(cancel_session_id IS NOT NULL)),CHECK((state=4)=(cancel_command_id IS NOT NULL)),
 CHECK((state=4)=(cancel_binding IS NOT NULL)),CHECK((state=4)=(cancelled_at_ms IS NOT NULL)),
 CHECK(cancel_command_id>0),CHECK(octet_length(cancel_binding) BETWEEN 1 AND 2048),
 CHECK(cancelled_at_ms<due_at_ms AND cancelled_at_ms>=requested_at_ms)

);
CREATE UNIQUE INDEX house_disposition_live_house ON game_house_dispositions(world_id,house_key) WHERE state<>4;
CREATE TABLE game_house_disposition_items (
 operation_id UUID NOT NULL REFERENCES game_house_dispositions(operation_id),item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 source_location JSONB NOT NULL,source_provenance JSONB NOT NULL,source_item JSONB NOT NULL,
 moved_step NUMERIC(20,0),moving_xact_id XID8,ordinal NUMERIC(20,0),
 PRIMARY KEY(operation_id,item_instance_id),CHECK((moved_step IS NULL)=(moving_xact_id IS NULL)),CHECK(ordinal IS NULL OR (ordinal>0 AND moved_step IS NOT NULL))
);
CREATE TABLE game_house_disposition_steps (
 operation_id UUID NOT NULL REFERENCES game_house_dispositions(operation_id),step NUMERIC(20,0) NOT NULL CHECK(step>=0),
 moved_count BIGINT NOT NULL CHECK(moved_count BETWEEN 0 AND 100),created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(operation_id,step)
);
INSERT INTO game_character_inbox_cause_kinds(cause_kind) VALUES('HOUSE_DISPOSITION');
CREATE FUNCTION game_house_disposition_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'disposition evidence is retained' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='game_house_dispositions' THEN
  IF OLD.state=1 AND NEW.state=4 THEN
   IF (to_jsonb(NEW)-ARRAY['state','cancel_session_id','cancel_command_id','cancel_binding','cancelled_at_ms']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['state','cancel_session_id','cancel_command_id','cancel_binding','cancelled_at_ms'])
   THEN RAISE EXCEPTION 'cancellation changed immutable notice' USING ERRCODE='23514'; END IF;
   RETURN NEW;
  END IF;
  IF (to_jsonb(NEW)-ARRAY['state','frozen_count','frozen_digest','next_step']) IS DISTINCT FROM
     (to_jsonb(OLD)-ARRAY['state','frozen_count','frozen_digest','next_step']) OR
     NOT ((OLD.state=1 AND NEW.state=2 AND NEW.next_step=OLD.next_step) OR
          (OLD.state=2 AND NEW.state IN(2,3) AND NEW.next_step=OLD.next_step+1 AND NEW.frozen_count=OLD.frozen_count AND NEW.frozen_digest=OLD.frozen_digest))
   THEN RAISE EXCEPTION 'immutable disposition binding/cause or invalid successor' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='game_house_disposition_items' THEN
  IF (to_jsonb(NEW)-ARRAY['moved_step','moving_xact_id','ordinal']) IS DISTINCT FROM
     (to_jsonb(OLD)-ARRAY['moved_step','moving_xact_id','ordinal']) OR
     NOT ((OLD.moved_step IS NULL AND NEW.moved_step IS NOT NULL AND NEW.moving_xact_id=pg_current_xact_id() AND NEW.ordinal IS NULL) OR
          (OLD.moved_step=NEW.moved_step AND OLD.moving_xact_id=pg_current_xact_id() AND NEW.moving_xact_id=OLD.moving_xact_id AND OLD.ordinal IS NULL AND NEW.ordinal IS NOT NULL))
   THEN RAISE EXCEPTION 'immutable census or already returned item' USING ERRCODE='23514'; END IF;
 ELSE RAISE EXCEPTION 'step evidence is immutable' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER house_disposition_immutable BEFORE UPDATE OR DELETE ON game_house_dispositions FOR EACH ROW EXECUTE FUNCTION game_house_disposition_immutable();
CREATE TRIGGER house_disposition_item_immutable BEFORE UPDATE OR DELETE ON game_house_disposition_items FOR EACH ROW EXECUTE FUNCTION game_house_disposition_immutable();
CREATE TRIGGER house_disposition_step_immutable BEFORE UPDATE OR DELETE ON game_house_disposition_steps FOR EACH ROW EXECUTE FUNCTION game_house_disposition_immutable();
CREATE FUNCTION game_house_disposition_fence_check(w UUID,h TEXT,i UUID,deleting BOOLEAN) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE op UUID;guild_relation REGCLASS;guild_guard REGPROCEDURE;
BEGIN
 -- Native row triggers can arrive with an item/provenance row already locked. Never
 -- wait here behind a disposition that owns this anchor and is waiting for that row.
 -- Root writers prelock the anchor; late generic writers fail closed with55P03.
 PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=h FOR SHARE NOWAIT;
 SELECT operation_id INTO op FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state IN(2,3);
 IF FOUND AND NOT (deleting AND EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=op AND c.item_instance_id=i AND c.moved_step IS NOT NULL AND c.moving_xact_id=pg_current_xact_id()))
 THEN RAISE EXCEPTION 'house disposition content fence' USING ERRCODE='23514'; END IF;
 -- The typed Guild owner supplies its actual census/reservation guard. Both native item
 -- mutations and House entry already pass through this one boundary. Ordinary DRAINED
 -- history remains fenced; a Guild release cannot bypass the ordinary branch above.
 guild_relation:=to_regclass(format('%I.game_house_guild_dispositions',current_schema()));
 IF guild_relation IS NOT NULL OR EXISTS(SELECT 1 FROM pg_catalog.pg_attribute
     WHERE attrelid='game_house_ownership'::regclass AND attname='owner_guild_id' AND NOT attisdropped) THEN
  guild_guard:=to_regprocedure(format('%I.game_house_guild_disposition_fence_check(uuid,text,uuid,boolean)',current_schema()));
  IF guild_relation IS NULL OR guild_guard IS NULL OR NOT EXISTS(SELECT 1 FROM pg_catalog.pg_proc
      WHERE oid=guild_guard AND prokind='f' AND prorettype='void'::regtype) THEN
   RAISE EXCEPTION 'Guild disposition fence owner is not installed' USING ERRCODE='23514';
  END IF;
  EXECUTE format('SELECT %I.game_house_guild_disposition_fence_check($1,$2,$3,$4)',current_schema())
   USING w,h,i,deleting;
 END IF;
END $$;
CREATE FUNCTION game_house_disposition_content_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN PERFORM game_house_disposition_fence_check(OLD.world_id,OLD.house_key,OLD.item_instance_id,TG_OP='DELETE'); END IF;
 IF TG_OP<>'DELETE' THEN PERFORM game_house_disposition_fence_check(NEW.world_id,NEW.house_key,NEW.item_instance_id,false); END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_content_fence BEFORE INSERT OR UPDATE OR DELETE ON game_item_house_interior_locations FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();
CREATE TRIGGER a_house_disposition_provenance_fence BEFORE INSERT OR UPDATE OR DELETE ON game_item_house_reclaim_provenance FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();
CREATE FUNCTION game_house_disposition_item_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE location RECORD;
BEGIN
 FOR location IN SELECT world_id,house_key FROM game_item_house_interior_locations
     WHERE item_instance_id=OLD.item_instance_id ORDER BY world_id,house_key LOOP
  -- A custody return moves its location/provenance; it never rewrites the frozen item.
  PERFORM game_house_disposition_fence_check(location.world_id,location.house_key,OLD.item_instance_id,false);
 END LOOP;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_item_fence BEFORE UPDATE OR DELETE ON game_item_instances FOR EACH ROW EXECUTE FUNCTION game_house_disposition_item_fence();
CREATE FUNCTION game_house_disposition_entry_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.state IN(1,2) THEN
  PERFORM game_house_disposition_fence_check(NEW.world_id,NEW.house_key,NULL,false);
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_entry_fence BEFORE INSERT OR UPDATE ON game_house_scope_handoffs FOR EACH ROW EXECUTE FUNCTION game_house_disposition_entry_fence();
CREATE FUNCTION game_house_disposition_delivery_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.moved_step IS NOT NULL THEN
  IF NOT EXISTS(SELECT 1 FROM game_house_disposition_items c JOIN game_house_dispositions o USING(operation_id)
   JOIN game_item_character_inbox_locations l ON l.item_instance_id=c.item_instance_id
   JOIN game_character_inbox_deliveries d ON d.item_instance_id=l.item_instance_id AND d.world_id=l.world_id AND d.character_id=l.character_id AND d.ordinal=l.ordinal
   JOIN game_item_instances i ON i.item_instance_id=c.item_instance_id
   JOIN game_house_disposition_steps s ON s.operation_id=c.operation_id AND s.step=c.moved_step
   WHERE c.operation_id=NEW.operation_id AND c.item_instance_id=NEW.item_instance_id AND c.ordinal=l.ordinal
    AND l.character_id=(c.source_provenance->>'reclaim_subject_character_id')::uuid AND l.world_id=o.world_id
    AND d.cause_kind='HOUSE_DISPOSITION' AND d.cause_ref=o.operation_id::text AND d.created_xact_id=c.moving_xact_id
    AND s.created_xact_id=c.moving_xact_id AND to_jsonb(i)=c.source_item
    AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=c.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=c.item_instance_id))
  THEN RAISE EXCEPTION 'disposition item lacks native delivery/source/step proof' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER house_disposition_delivery_proven AFTER UPDATE ON game_house_disposition_items DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_delivery_proven();
-- Installed with the real common, location, provenance, item and entry guards above.
-- The typed Guild owner requires this protected ABI before freezing or applying effects;
-- an older ordinary-only fence has no such capability and must refuse Guild integration.
CREATE FUNCTION game_house_disposition_guild_fence_abi() RETURNS INTEGER LANGUAGE sql IMMUTABLE AS $$ SELECT 1 $$;
REVOKE ALL ON game_house_dispositions,game_house_disposition_items,game_house_disposition_steps FROM PUBLIC;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_disposition_immutable','game_house_disposition_content_fence','game_house_disposition_item_fence','game_house_disposition_entry_fence','game_house_disposition_delivery_proven','game_house_disposition_guild_fence_abi'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_disposition_fence_check(UUID,TEXT,UUID,BOOLEAN) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_disposition_fence_check(UUID,TEXT,UUID,BOOLEAN) FROM PUBLIC;
-- The genuine Guild participant reads this constant on its runtime transaction. This
-- grants no trigger, mutation, table or terminal-release authority to that role.
GRANT EXECUTE ON FUNCTION game_house_disposition_guild_fence_abi() TO oteryn_game_runtime;


CREATE FUNCTION game_house_disposition_census_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_house_dispositions o
  JOIN game_item_house_interior_locations h ON h.item_instance_id=NEW.item_instance_id AND h.world_id=o.world_id AND h.house_key=o.house_key
  JOIN game_item_house_reclaim_provenance p ON p.item_instance_id=h.item_instance_id
  JOIN game_item_instances i ON i.item_instance_id=h.item_instance_id
  WHERE o.operation_id=NEW.operation_id AND o.state=1 AND NEW.moved_step IS NULL AND NEW.ordinal IS NULL
   AND to_jsonb(h)=NEW.source_location AND to_jsonb(p)=NEW.source_provenance AND to_jsonb(i)=NEW.source_item
   AND p.world_id=h.world_id AND p.house_key=h.house_key AND p.placement_transaction_id=h.placed_transaction_id)
 THEN RAISE EXCEPTION 'census must capture actual native source/provenance' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER house_disposition_census_proven BEFORE INSERT ON game_house_disposition_items FOR EACH ROW EXECUTE FUNCTION game_house_disposition_census_proven();
CREATE FUNCTION game_house_disposition_operation_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE n BIGINT;d BYTEA;
BEGIN
 IF NEW.state=4 THEN RETURN NULL; END IF;
 SELECT count(*),sha256(COALESCE(string_agg(uuid_send(item_instance_id),''::bytea ORDER BY item_instance_id),''::bytea)) INTO n,d FROM game_house_disposition_items WHERE operation_id=NEW.operation_id;
 IF n<>NEW.frozen_count OR d<>NEW.frozen_digest THEN RAISE EXCEPTION 'frozen census count/digest changed' USING ERRCODE='23514'; END IF;
 IF OLD.state=1 THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.world_id=NEW.world_id AND h.house_key=NEW.house_key AND NOT EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.item_instance_id=h.item_instance_id)) OR
     EXISTS(SELECT 1 FROM game_house_disposition_items c LEFT JOIN game_item_house_interior_locations h USING(item_instance_id) LEFT JOIN game_item_house_reclaim_provenance p USING(item_instance_id) LEFT JOIN game_item_instances i USING(item_instance_id) WHERE c.operation_id=NEW.operation_id AND (to_jsonb(h) IS DISTINCT FROM c.source_location OR to_jsonb(p) IS DISTINCT FROM c.source_provenance OR to_jsonb(i) IS DISTINCT FROM c.source_item))
   THEN RAISE EXCEPTION 'frozen set must equal native source set' USING ERRCODE='23514'; END IF;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_house_disposition_steps s WHERE s.operation_id=NEW.operation_id AND s.step=OLD.next_step AND s.created_xact_id=pg_current_xact_id()
    AND s.moved_count=(SELECT count(*) FROM game_house_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.moved_step=s.step AND c.moving_xact_id=s.created_xact_id)) OR
    ((NEW.state=3) <> NOT EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.moved_step IS NULL))
   THEN RAISE EXCEPTION 'step lacks same-transaction census evidence' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER house_disposition_operation_proven AFTER UPDATE ON game_house_dispositions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_operation_proven();
CREATE FUNCTION game_house_disposition_deliver(op UUID,st NUMERIC,it UUID) RETURNS TABLE(character_id UUID,ordinal NUMERIC)
LANGUAGE plpgsql SECURITY DEFINER AS $$
#variable_conflict use_column
DECLARE c game_house_disposition_items%ROWTYPE;o game_house_dispositions%ROWTYPE;subject UUID;dest RECORD;
BEGIN
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op FOR UPDATE;
 IF NOT FOUND OR o.state<>2 OR o.next_step<>st THEN RAISE EXCEPTION 'disposition step not current' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_house_ownership h WHERE h.world_id=o.world_id AND h.house_key=o.house_key AND h.owner_character_id=o.owner_character_id AND h.ownership_revision=o.ownership_revision FOR SHARE)
 THEN RAISE EXCEPTION 'disposition native ownership changed' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.runtime_scope_kind=2 AND s.runtime_scope_world_id=o.world_id AND s.runtime_scope_house_key=o.house_key AND s.session_state IN(1,2)) OR
    EXISTS(SELECT 1 FROM game_house_scope_handoffs h WHERE h.world_id=o.world_id AND h.house_key=o.house_key AND h.state IN(1,2))
 THEN RAISE EXCEPTION 'disposition has canonical sessions or entrants' USING ERRCODE='23514'; END IF;
 SELECT * INTO c FROM game_house_disposition_items WHERE operation_id=op AND item_instance_id=it FOR UPDATE;
 IF NOT FOUND OR c.moved_step IS NOT NULL OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h
   JOIN game_item_house_reclaim_provenance p USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
   WHERE h.item_instance_id=it AND h.world_id=o.world_id AND h.house_key=o.house_key AND to_jsonb(h)=c.source_location AND to_jsonb(p)=c.source_provenance AND to_jsonb(i)=c.source_item)
 THEN RAISE EXCEPTION 'disposition native source/provenance changed' USING ERRCODE='23514'; END IF;
 subject:=(c.source_provenance->>'reclaim_subject_character_id')::uuid;
 UPDATE game_house_disposition_items SET moved_step=st,moving_xact_id=pg_current_xact_id() WHERE operation_id=op AND item_instance_id=it;
 DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=it;
 DELETE FROM game_item_house_interior_locations WHERE item_instance_id=it;
 SELECT * INTO dest FROM game_character_inbox_deliver(it,subject,o.world_id,'HOUSE_DISPOSITION',op::text);
 UPDATE game_house_disposition_items SET ordinal=dest.ordinal WHERE operation_id=op AND item_instance_id=it;
 RETURN QUERY SELECT dest.character_id,dest.ordinal;
END $$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_disposition_census_proven','game_house_disposition_operation_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_disposition_deliver(UUID,NUMERIC,UUID) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_disposition_deliver(UUID,NUMERIC,UUID) FROM PUBLIC;

-- A disposition is a job-to-Inbox custody move, not player House Take/Move.
-- Admit only the actual frozen location with completed, same-physical-transaction
-- native Inbox and immutable step evidence. The 0095 player shape arms stay intact.
CREATE FUNCTION game_house_disposition_native_removal_proven(source JSONB) RETURNS BOOLEAN
LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(
  SELECT 1 FROM game_house_dispositions o
   JOIN game_house_disposition_items c USING(operation_id)
   JOIN game_house_disposition_steps s ON s.operation_id=c.operation_id AND s.step=c.moved_step
   JOIN game_item_character_inbox_locations l ON l.item_instance_id=c.item_instance_id
   JOIN game_character_inbox_deliveries d ON d.item_instance_id=l.item_instance_id
    AND d.world_id=l.world_id AND d.character_id=l.character_id AND d.ordinal=l.ordinal
   JOIN game_item_instances i ON i.item_instance_id=c.item_instance_id
  WHERE o.world_id=(source->>'world_id')::uuid AND o.house_key=source->>'house_key'
   AND o.state<>4 AND o.state IN(2,3) AND c.item_instance_id=(source->>'item_instance_id')::uuid
   AND c.source_location=source AND c.moving_xact_id=pg_current_xact_id() AND c.ordinal=l.ordinal
   AND (c.source_location->>'world_id')::uuid=o.world_id
   AND c.source_location->>'house_key'=o.house_key
   AND l.character_id=(c.source_provenance->>'reclaim_subject_character_id')::uuid
   AND l.world_id=o.world_id AND d.cause_kind='HOUSE_DISPOSITION'
   AND d.cause_ref=o.operation_id::text AND d.created_xact_id=c.moving_xact_id
   AND s.created_xact_id=c.moving_xact_id AND to_jsonb(i)=c.source_item
   AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=c.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=c.item_instance_id)
 );
$$;
CREATE OR REPLACE FUNCTION game_house_location_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  IF game_house_disposition_native_removal_proven(to_jsonb(OLD)) THEN RETURN NULL;END IF;
 END IF;
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
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_disposition_native_removal_proven(JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_location_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE EXECUTE ON FUNCTION game_house_disposition_native_removal_proven(JSONB) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON game_house_dispositions,game_house_disposition_items,game_house_disposition_steps FROM oteryn_game_runtime,oteryn_game_control;
