-- M49: energy soil creates one persistent Ground field above the unchanged altar.
CREATE TABLE game_elemental_energy_field_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances DEFERRABLE INITIALLY DEFERRED,
 craft_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_craft_records DEFERRABLE INITIALLY DEFERRED,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
 world_id UUID NOT NULL, channel_id UUID NOT NULL, reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 spatial_position BYTEA NOT NULL CHECK(octet_length(spatial_position)=10),
 map_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
 placement_context BYTEA NOT NULL CHECK(octet_length(placement_context)=32),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 65536),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(transaction_id<>craft_transaction_id)
);
CREATE FUNCTION game_elemental_energy_field_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE c game_item_craft_records%ROWTYPE; r game_terrain_tool_receipts%ROWTYPE;
 t game_terrain_tool_transforms%ROWTYPE; a game_content_activations%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_item_craft_records WHERE transaction_id=NEW.craft_transaction_id;
 SELECT * INTO r FROM game_terrain_tool_receipts WHERE game_session_id=c.game_session_id AND command_id=c.command_id;
 SELECT * INTO t FROM game_terrain_tool_transforms WHERE (world_id,channel_id,placement,reset_epoch)=(r.world_id,r.channel_id,r.placement,r.reset_epoch);
 SELECT * INTO a FROM game_content_activations WHERE world_id=r.world_id AND channel_id=r.channel_id ORDER BY activation_sequence DESC LIMIT 1;
 IF c.transaction_id IS NULL OR c.created_xact_id<>pg_current_xact_id()
   OR c.recipe<>'oteryn:elemental_energy_soil.crystal' OR NOT c.consume_source OR c.outputs<>'[]'::jsonb
   OR c.before_item->>'definition_production_key'<>'oteryn:item.tibia.i945'
   OR r.game_session_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
   OR (r.world_id,r.character_id,r.tool_item_instance_id,r.intent) IS DISTINCT FROM (c.world_id,c.character_id,c.source_item_instance_id,c.intent)
   OR r.transformed_item_id<>7737 OR t.state_item_id<>7737 OR cardinality(t.decay_states)<>0 OR t.decay_at_unix_ms IS NOT NULL
   OR a.activation_sequence IS NULL OR t.content_generation<>a.server_artifact_digest
   OR r.reset_epoch<>coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=r.world_id),0)
   OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key)
       WHERE s.world_id=r.world_id AND s.channel_id=r.channel_id AND s.state=1 AND s.ownership_generation=NEW.scope_generation
       AND g.ready AND g.ownership_generation=s.ownership_generation)
 THEN RAISE EXCEPTION 'energy field requires its fresh soil craft and unchanged native altar' USING ERRCODE='23514'; END IF;
 NEW.world_id:=r.world_id; NEW.channel_id:=r.channel_id; NEW.reset_epoch:=r.reset_epoch;
 NEW.spatial_position:=int4send(get_byte(r.placement,34)*256+get_byte(r.placement,35))
   ||int4send(get_byte(r.placement,36)*256+get_byte(r.placement,37))||int2send(get_byte(r.placement,38)::smallint);
 NEW.map_revision:='sha256:'||encode(sha256(convert_to(t.map_revision,'UTF8')),'hex');
 NEW.content_revision:='sha256:'||encode(t.content_generation,'hex'); NEW.placement_context:=a.frame_binding_digest;
 NEW.occurred_at:=c.occurred_at; NEW.created_xact_id:=pg_current_xact_id(); NEW.envelope_sha256:=sha256(NEW.envelope);
 -- Match the existing native Item scope lock and count actual Ground roots.
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(NEW.world_id),'hex')||encode(uuid_send(NEW.channel_id),'hex'),33));
 IF (SELECT count(*) FROM (SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
       WHERE g.world_id=NEW.world_id AND g.channel_id=NEW.channel_id AND g.scope_kind=1 AND g.instance_id IS NULL
       AND g.spatial_position=NEW.spatial_position AND i.lifecycle=1 LIMIT 64) bounded)>=64
   OR EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
       WHERE g.world_id=NEW.world_id AND g.channel_id=NEW.channel_id AND g.scope_kind=1 AND g.instance_id IS NULL
       AND g.spatial_position=NEW.spatial_position AND i.lifecycle=1
       AND (g.map_revision,g.content_revision,g.native_room_placement_context) IS DISTINCT FROM (NEW.map_revision,NEW.content_revision,NEW.placement_context))
 THEN RAISE EXCEPTION 'energy field Ground cell is full or unreconciled' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_elemental_energy_field_complete(p_craft UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_elemental_energy_field_receipts%ROWTYPE; i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_elemental_energy_field_receipts WHERE craft_transaction_id=p_craft;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL OR g.item_instance_id IS NULL
   OR (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,i.state_revision)
       IS DISTINCT FROM (r.world_id,'Item'::text,'oteryn:item.tibia.i2126'::text,'definition-r1'::text,1::bigint,1::smallint,1::numeric)
   OR i.minted_transaction_id<>r.transaction_id OR i.last_transaction_id IS NOT NULL
   OR (g.world_id,g.channel_id,g.runtime_scope_ownership_generation,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
       IS DISTINCT FROM (r.world_id,r.channel_id,r.scope_generation,r.spatial_position,r.map_revision,r.content_revision,r.placement_context)
   OR g.scope_kind<>1 OR g.instance_id IS NOT NULL OR g.stack_ordinal IS NULL
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=i.item_instance_id)
   OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>1
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.transaction_id=r.transaction_id AND o.event_id=r.event_id
       AND o.item_instance_id=r.item_instance_id AND o.envelope=r.envelope AND o.envelope_sha256=r.envelope_sha256
       AND o.occurred_at=r.occurred_at AND o.created_xact_id=pg_current_xact_id() AND o.transaction_ordinal=1 AND o.transaction_count=1
       AND o.publication_state=1 AND o.published_at IS NULL)
 THEN RAISE EXCEPTION 'energy soil must mint exactly its persistent field and own audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_elemental_energy_field_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_item_craft_complete(NEW.craft_transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_elemental_energy_field_admit BEFORE INSERT ON game_elemental_energy_field_receipts FOR EACH ROW EXECUTE FUNCTION game_elemental_energy_field_admit();
CREATE CONSTRAINT TRIGGER game_elemental_energy_field_consistent AFTER INSERT ON game_elemental_energy_field_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_energy_field_consistent();
CREATE TRIGGER game_elemental_energy_field_immutable BEFORE UPDATE OR DELETE ON game_elemental_energy_field_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_elemental_energy_field_no_truncate BEFORE TRUNCATE ON game_elemental_energy_field_receipts EXECUTE FUNCTION game_item_reject_truncate();
DO $migration$
DECLARE definition TEXT; body TEXT; at INTEGER; name TEXT; arm TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
 EXECUTE 'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ('||substring(definition FROM 7)
   ||' OR COALESCE(consume_source AND recipe=''oteryn:elemental_energy_soil.crystal'' AND outputs=''[]''::jsonb AND before_item->>''definition_production_key''=''oteryn:item.tibia.i945'',FALSE))';
 FOREACH name IN ARRAY ARRAY['game_item_craft_admit','game_item_craft_complete','game_item_mint_consistency_guard','game_item_ground_insertion_guard'] LOOP
   body:=pg_get_functiondef((name||CASE WHEN name='game_item_craft_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
   at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing energy field predecessor %',name; END IF;
   IF name='game_item_craft_admit' THEN arm:=$arm$
     IF NEW.recipe='oteryn:elemental_energy_soil.crystal' AND (
       NOT NEW.consume_source OR NEW.outputs<>'[]'::jsonb OR NEW.before_item->>'definition_production_key'<>'oteryn:item.tibia.i945'
       OR NOT EXISTS(SELECT 1 FROM game_terrain_tool_receipts r JOIN game_terrain_tool_transforms t
          ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)=(r.world_id,r.channel_id,r.placement,r.reset_epoch)
          WHERE (r.game_session_id,r.command_id,r.character_id,r.world_id,r.tool_item_instance_id,r.intent)
             =(NEW.game_session_id,NEW.command_id,NEW.character_id,NEW.world_id,NEW.source_item_instance_id,NEW.intent)
          AND r.created_xact_id=pg_current_xact_id() AND r.transformed_item_id=7737 AND t.state_item_id=7737
          AND cardinality(t.decay_states)=0 AND t.decay_at_unix_ms IS NULL))
     THEN RAISE EXCEPTION 'energy soil requires a fresh unchanged altar receipt' USING ERRCODE='23514'; END IF;
   $arm$;
   ELSIF name='game_item_craft_complete' THEN arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_item_craft_records WHERE transaction_id=p_transaction AND recipe='oteryn:elemental_energy_soil.crystal')
     THEN PERFORM game_elemental_energy_field_complete(p_transaction); END IF;
   $arm$;
   ELSE arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_energy_field_receipts r WHERE r.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM game_item_craft_complete((SELECT craft_transaction_id FROM game_elemental_energy_field_receipts WHERE item_instance_id=NEW.item_instance_id)); RETURN NULL;
     END IF;
   $arm$; END IF;
   EXECUTE overlay(body PLACING E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
END $migration$;
CREATE FUNCTION game_elemental_energy_field_source(p_item UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_elemental_energy_field_receipts r JOIN game_item_instances i USING(item_instance_id)
   JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.item_instance_id=p_item
   AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL AND i.minted_transaction_id=r.transaction_id
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=('Item','oteryn:item.tibia.i2126','definition-r1')
   AND (g.world_id,g.channel_id,g.runtime_scope_ownership_generation,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
     =(r.world_id,r.channel_id,r.scope_generation,r.spatial_position,r.map_revision,r.content_revision,r.placement_context)
   AND g.scope_kind=1 AND g.instance_id IS NULL
   AND r.reset_epoch=coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=r.world_id),0))
$$;
DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_elemental_energy_field_admit()','game_elemental_energy_field_complete(uuid)',
  'game_elemental_energy_field_consistent()','game_elemental_energy_field_source(uuid)'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
REVOKE ALL ON game_elemental_energy_field_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_elemental_energy_field_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_elemental_energy_field_receipts TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_elemental_energy_field_complete(UUID),game_elemental_energy_field_source(UUID) TO oteryn_game_runtime;
