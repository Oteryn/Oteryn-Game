-- Crystal 201189f8: the scarab coin dropped into a coal basin and its
-- optional flame occupant's arrival belong to the same item-loop transaction.
CREATE TABLE game_tomb_basin_offerings (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 move_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL, channel_id UUID NOT NULL,
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 before_item JSONB NOT NULL CHECK(jsonb_typeof(before_item)='object'),
 before_ground JSONB NOT NULL CHECK(jsonb_typeof(before_ground)='object'),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 4096),
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 object_revision BIGINT NOT NULL CHECK(object_revision>=0),
 basin_uid INTEGER NOT NULL CHECK(basin_uid BETWEEN 9033 AND 9040),
 occupant_id UUID REFERENCES game_character_roots(character_id),
 occupant_session_id UUID,
 CHECK((occupant_id IS NULL) = (occupant_session_id IS NULL)),
 map_revision TEXT NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_tomb_basin_item_idx ON game_tomb_basin_offerings(item_instance_id);
CREATE INDEX game_tomb_basin_character_idx ON game_tomb_basin_offerings(character_id);

CREATE FUNCTION game_tomb_basin_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
   JOIN game_item_loop_move_lines l USING(item_instance_id)
   JOIN game_item_loop_move_receipts r USING(transaction_id)
   WHERE i.item_instance_id=NEW.item_instance_id AND to_jsonb(i)=NEW.before_item AND to_jsonb(g)=NEW.before_ground
    AND i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item'
    AND i.definition_production_key='oteryn:item.tibia.i3042' AND i.definition_revision_ref='definition-r1'
    AND r.transaction_id=NEW.move_transaction_id AND r.created_xact_id=pg_current_xact_id()
    AND r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id AND r.character_id=NEW.character_id
    AND r.world_id=NEW.world_id AND r.channel_id=NEW.channel_id AND r.intent=NEW.intent
    AND l.to_kind=3 AND l.quantity_after=i.quantity AND l.to_spatial_position=g.spatial_position
    AND g.world_id=NEW.world_id AND g.channel_id=NEW.channel_id AND g.map_revision=NEW.map_revision
    AND g.content_revision='sha256:'||encode(NEW.content_digest,'hex')
    AND g.runtime_scope_ownership_generation=r.scope_generation
    AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id)
    AND g.spatial_position=int4send(get_byte(NEW.placement,34)*256+get_byte(NEW.placement,35))
     ||int4send(get_byte(NEW.placement,36)*256+get_byte(NEW.placement,37))
     ||int2send((-get_byte(NEW.placement,38))::smallint)
    AND game_world_reset_map_epoch_current(NEW.world_id,NEW.reset_epoch::numeric)
    AND EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=NEW.world_id AND c.channel_id=NEW.channel_id
     AND c.activation_sequence=(SELECT a.activation_sequence FROM game_content_activations a WHERE a.world_id=c.world_id AND a.channel_id=c.channel_id ORDER BY a.activation_sequence DESC LIMIT 1)
     AND c.server_artifact_digest=NEW.content_digest AND (c.frame_binding_digest=g.native_room_placement_context OR EXISTS(
      -- A bundle Channel's Ground row carries the bundle frame of the active
      -- reset map, while the activation retains the entry frame (2733 pattern).
      SELECT 1 FROM game_world_reset_active active JOIN game_world_reset_epochs epoch USING(world_id,reset_epoch)
      WHERE active.world_id=NEW.world_id AND active.reset_epoch=NEW.reset_epoch
       AND epoch.server_digest=c.server_artifact_digest AND epoch.client_digest=c.client_artifact_digest
       AND g.map_revision='sha256:'||encode(sha256(convert_to(epoch.map_revision,'UTF8')),'hex')
       AND game_elemental_placement_frame_matches(NEW.world_id,epoch.map_revision,g.native_room_placement_context,c.frame_binding_digest))))
    AND i.last_transaction_id=r.transaction_id AND i.state_revision=l.state_revision_before+1)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id)
 THEN RAISE EXCEPTION 'Tomb basin requires its exact newly dropped scarab coins' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_tomb_basin_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_tomb_basin_offerings%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_tomb_basin_offerings WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL
  OR i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id<>r.transaction_id
  OR i.state_revision<>(r.before_item->>'state_revision')::numeric+1
  OR (to_jsonb(i)-ARRAY['quantity','lifecycle','state_revision','last_transaction_id']) IS DISTINCT FROM
     (r.before_item-ARRAY['quantity','lifecycle','state_revision','last_transaction_id'])
  OR (r.occupant_id IS NOT NULL AND NOT EXISTS (
   SELECT 1 FROM game_character_pending_arrivals a
   JOIN (VALUES (9033,33093,32824,13),(9034,33299,32742,13),
      (9035,33080,32588,13),(9036,33246,32850,13),
      (9037,33271,32553,14),(9038,33234,32687,13),
      (9039,33130,32683,12),(9040,33158,32832,10)
   ) AS destination(uid,x,y,floor) ON destination.uid=r.basin_uid
   WHERE a.character_id=r.occupant_id AND a.occurrence_id=r.transaction_id
     AND a.world_id=r.world_id AND (a.tile_x,a.tile_y,a.tile_floor)=(destination.x,destination.y,destination.floor)
     AND a.xmin=pg_current_xact_id()::xid AND NOT a.source_npc_teleport
 )) OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=r.event_id AND a.transaction_id=r.transaction_id
   AND a.item_instance_id=r.item_instance_id AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
   AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1 AND a.transaction_count=1
   AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'Tomb basin requires its exact debit, custody removal, audit and arrival' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_tomb_basin_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_tomb_basin_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_tomb_basin_admit BEFORE INSERT ON game_tomb_basin_offerings
 FOR EACH ROW EXECUTE FUNCTION game_tomb_basin_admit();
CREATE CONSTRAINT TRIGGER game_tomb_basin_consistent AFTER INSERT ON game_tomb_basin_offerings
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_tomb_basin_consistent();
CREATE TRIGGER game_tomb_basin_immutable BEFORE UPDATE OR DELETE ON game_tomb_basin_offerings
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_tomb_basin_no_truncate BEFORE TRUNCATE ON game_tomb_basin_offerings
 EXECUTE FUNCTION game_item_reject_truncate();

-- Extend the three existing custody proofs with the exact basin receipt.
DO $migration$
DECLARE definition TEXT; anchor TEXT; arm TEXT; position INTEGER; signature TEXT;
BEGIN
 definition:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 anchor:=$anchor$        UNION ALL SELECT 1 FROM game_dreamer_ring_offerings WHERE transaction_id=NEW.last_transaction_id$anchor$;
 IF (length(definition)-length(replace(definition,anchor,'')))<>length(anchor)
 THEN RAISE EXCEPTION 'missing tomb basin terminal guard predecessor'; END IF;
 definition:=replace(definition,anchor,anchor||E'\n        UNION ALL SELECT 1 FROM game_tomb_basin_offerings WHERE transaction_id=NEW.last_transaction_id');
 anchor:=$anchor$    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r$anchor$;
 IF (length(definition)-length(replace(definition,anchor,'')))<>length(anchor)
 THEN RAISE EXCEPTION 'missing tomb basin instance guard predecessor'; END IF;
 arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_tomb_basin_offerings r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id
      AND r.created_xact_id=pg_current_xact_id() AND r.before_item=v_instance_guard_old_json) THEN
     IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'Tomb basin retires exactly the dropped scarab coins' USING ERRCODE='23514'; END IF;
     NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
    END IF;
$arm$;
 EXECUTE replace(definition,anchor,arm||anchor);

 definition:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 position:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF position=0 THEN RAISE EXCEPTION 'missing tomb basin instance proof predecessor'; END IF;
 arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_tomb_basin_offerings r WHERE r.transaction_id=NEW.last_transaction_id
     AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
     PERFORM game_tomb_basin_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
 EXECUTE overlay(definition placing E'BEGIN\n'||arm FROM position FOR 5);

 definition:=pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure);
 position:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF position=0 THEN RAISE EXCEPTION 'missing tomb basin ground guard predecessor'; END IF;
 arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_tomb_basin_offerings r WHERE r.item_instance_id=OLD.item_instance_id
     AND r.created_xact_id=pg_current_xact_id() AND r.before_ground=to_jsonb(OLD)) THEN
     PERFORM game_tomb_basin_complete((SELECT transaction_id FROM game_tomb_basin_offerings
      WHERE item_instance_id=OLD.item_instance_id AND created_xact_id=pg_current_xact_id())); RETURN NULL;
    END IF;
$arm$;
 EXECUTE overlay(definition placing E'BEGIN\n'||arm FROM position FOR 5);

 FOREACH signature IN ARRAY ARRAY['game_tomb_basin_admit()','game_tomb_basin_complete(uuid)','game_tomb_basin_consistent()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON game_tomb_basin_offerings FROM PUBLIC;
REVOKE ALL ON FUNCTION game_tomb_basin_admit(),game_tomb_basin_complete(UUID),game_tomb_basin_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_tomb_basin_offerings TO oteryn_game_runtime;
GRANT SELECT ON game_tomb_basin_offerings TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_tomb_basin_complete(UUID) TO oteryn_game_runtime;
