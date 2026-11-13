-- Crystal 201189f movement_death_ring.lua: UID2244 consumes a newly dropped6299
-- only while at least one of the three source trees remains. No item is produced.
CREATE TABLE game_dreamer_ring_offerings (
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
 trees_before INTEGER[] NOT NULL CHECK(cardinality(trees_before)=3
  AND array_position(trees_before,NULL) IS NULL AND trees_before<@ARRAY[0,3636] AND 3636=ANY(trees_before)),
 map_revision TEXT NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_dreamer_ring_item_idx ON game_dreamer_ring_offerings(item_instance_id);
CREATE INDEX game_dreamer_ring_character_idx ON game_dreamer_ring_offerings(character_id);

CREATE FUNCTION game_dreamer_ring_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
   JOIN game_item_loop_move_lines l USING(item_instance_id)
   JOIN game_item_loop_move_receipts r USING(transaction_id)
   WHERE i.item_instance_id=NEW.item_instance_id AND to_jsonb(i)=NEW.before_item AND to_jsonb(g)=NEW.before_ground
    AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
    AND i.definition_production_key='oteryn:item.tibia.i6299' AND i.definition_revision_ref='definition-r1'
    AND r.transaction_id=NEW.move_transaction_id AND r.created_xact_id=pg_current_xact_id()
    AND r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id AND r.character_id=NEW.character_id
    AND r.world_id=NEW.world_id AND r.channel_id=NEW.channel_id AND r.intent=NEW.intent
    AND l.to_kind=3 AND l.quantity_after=1 AND l.to_spatial_position=g.spatial_position
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
     AND c.server_artifact_digest=NEW.content_digest AND c.frame_binding_digest=g.native_room_placement_context)
    AND i.last_transaction_id=r.transaction_id AND i.state_revision=l.state_revision_before+1)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id)
 THEN RAISE EXCEPTION 'Dreamer offering requires its exact newly dropped inactive ring' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_dreamer_ring_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_dreamer_ring_offerings%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_dreamer_ring_offerings WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL
  OR i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id<>r.transaction_id
  OR i.state_revision<>(r.before_item->>'state_revision')::numeric+1
  OR (to_jsonb(i)-ARRAY['quantity','lifecycle','state_revision','last_transaction_id']) IS DISTINCT FROM
     (r.before_item-ARRAY['quantity','lifecycle','state_revision','last_transaction_id'])
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=r.event_id AND a.transaction_id=r.transaction_id
   AND a.item_instance_id=r.item_instance_id AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
   AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1 AND a.transaction_count=1
   AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'Dreamer offering requires its exact debit, custody removal and audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_dreamer_ring_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_dreamer_ring_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_dreamer_ring_admit BEFORE INSERT ON game_dreamer_ring_offerings
 FOR EACH ROW EXECUTE FUNCTION game_dreamer_ring_admit();
CREATE CONSTRAINT TRIGGER game_dreamer_ring_consistent AFTER INSERT ON game_dreamer_ring_offerings
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_dreamer_ring_consistent();
CREATE TRIGGER game_dreamer_ring_immutable BEFORE UPDATE OR DELETE ON game_dreamer_ring_offerings
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_dreamer_ring_no_truncate BEFORE TRUNCATE ON game_dreamer_ring_offerings
 EXECUTE FUNCTION game_item_reject_truncate();

-- Preserve every original guard arm and admit only this exact one-ring successor.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 -- Item instance retirement admission is composed after accepted audit guards by migration 2815.
 FOREACH n IN ARRAY ARRAY['game_item_instance_change_proven','game_item_ground_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing Dreamer offering predecessor guard %',n; END IF;
  IF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r WHERE r.transaction_id=NEW.last_transaction_id
     AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
     PERFORM game_dreamer_ring_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r WHERE r.item_instance_id=OLD.item_instance_id
     AND r.created_xact_id=pg_current_xact_id() AND r.before_ground=to_jsonb(OLD)) THEN
     PERFORM game_dreamer_ring_complete((SELECT transaction_id FROM game_dreamer_ring_offerings
      WHERE item_instance_id=OLD.item_instance_id AND created_xact_id=pg_current_xact_id())); RETURN NULL;
    END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_dreamer_ring_admit()','game_dreamer_ring_complete(uuid)','game_dreamer_ring_consistent()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON game_dreamer_ring_offerings FROM PUBLIC;
REVOKE ALL ON FUNCTION game_dreamer_ring_admit(),game_dreamer_ring_complete(UUID),game_dreamer_ring_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_dreamer_ring_offerings TO oteryn_game_runtime;
GRANT SELECT ON game_dreamer_ring_offerings TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_dreamer_ring_complete(UUID) TO oteryn_game_runtime;
