-- Source-bound, bounded AddItem recipes. The ordinary move is proven first in
-- the same physical transaction; this receipt proves its complete successor.
CREATE TABLE game_floor_assembly_recipes (
 recipe TEXT PRIMARY KEY,
 action_id INTEGER NOT NULL UNIQUE CHECK(action_id BETWEEN 1 AND 65535),
 input_keys TEXT[] NOT NULL CHECK(cardinality(input_keys) BETWEEN 1 AND 7),
 input_counts INTEGER[] NOT NULL CHECK(cardinality(input_counts)=cardinality(input_keys)
   AND 0<ALL(input_counts) AND 100>=ALL(input_counts)),
 output_key TEXT NOT NULL,
 output_count INTEGER NOT NULL CHECK(output_count BETWEEN 1 AND 100)
);
INSERT INTO game_floor_assembly_recipes VALUES(
 'oteryn:floor-assembly.ancient-helmet',40040,
 ARRAY['oteryn:item.tibia.i3222','oteryn:item.tibia.i3223','oteryn:item.tibia.i3224',
 'oteryn:item.tibia.i3225','oteryn:item.tibia.i3226','oteryn:item.tibia.i3227','oteryn:item.tibia.i3228'],
 ARRAY[1,1,1,1,1,1,1],'oteryn:item.tibia.i3229',1);
CREATE TRIGGER game_floor_recipe_immutable BEFORE UPDATE OR DELETE ON game_floor_assembly_recipes
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_floor_recipe_no_truncate BEFORE TRUNCATE ON game_floor_assembly_recipes
 EXECUTE FUNCTION game_item_reject_truncate();

CREATE TABLE game_floor_assembly_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 move_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
 recipe TEXT NOT NULL REFERENCES game_floor_assembly_recipes,
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs) BETWEEN 1 AND 7
   AND octet_length(inputs::text)<=32768),
 inputs_intent BYTEA NOT NULL CHECK(octet_length(inputs_intent) BETWEEN 1 AND 32768
   AND convert_from(inputs_intent,'UTF8')::jsonb=inputs),
 inputs_sha256 BYTEA NOT NULL CHECK(octet_length(inputs_sha256)=32 AND inputs_sha256=sha256(inputs_intent)),
 output_item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances DEFERRABLE INITIALLY DEFERRED,
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_floor_assembly_recipe_idx ON game_floor_assembly_receipts(recipe);

CREATE FUNCTION game_floor_assembly_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_floor_assembly_recipes%ROWTYPE; m game_item_loop_move_receipts%ROWTYPE;
 i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE; operand JSONB; n INTEGER:=0;
 first_ground JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT p FROM game_floor_assembly_recipes WHERE recipe=NEW.recipe;
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=NEW.move_transaction_id;
 IF m.created_xact_id<>pg_current_xact_id()
   OR NOT game_world_reset_map_epoch_current(m.world_id,NEW.reset_epoch)
   OR jsonb_array_length(NEW.inputs)<>cardinality(p.input_keys)
   OR (SELECT count(DISTINCT v->'item'->>'item_instance_id') FROM jsonb_array_elements(NEW.inputs) v)<>cardinality(p.input_keys)
   OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=NEW.output_item_instance_id)
 THEN RAISE EXCEPTION 'floor assembly requires a current unique move and complete inputs' USING ERRCODE='23514'; END IF;
 first_ground:=NEW.inputs->0->'ground';
 IF first_ground->>'map_revision'<>'sha256:'||encode(substring(NEW.placement FROM 1 FOR 32),'hex')
   OR decode(substring(first_ground->>'spatial_position' FROM 3),'hex')<>
       int4send(get_byte(NEW.placement,34)*256+get_byte(NEW.placement,35))
       ||int4send(get_byte(NEW.placement,36)*256+get_byte(NEW.placement,37))
       ||int2send((-get_byte(NEW.placement,38))::smallint)
   OR NOT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=m.world_id AND a.channel_id=m.channel_id
      AND a.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=m.world_id AND channel_id=m.channel_id)
      AND first_ground->>'content_revision'='sha256:'||encode(a.server_artifact_digest,'hex')
      AND first_ground->>'native_room_placement_context'='\x'||encode(a.frame_binding_digest,'hex'))
   OR NOT EXISTS(SELECT 1 FROM game_item_loop_move_lines l WHERE l.transaction_id=m.transaction_id
      AND l.to_kind=3 AND l.definition_family='Item' AND l.definition_production_key=ANY(p.input_keys)
      AND l.to_spatial_position=decode(substring(first_ground->>'spatial_position' FROM 3),'hex'))
 THEN RAISE EXCEPTION 'floor assembly requires the exact current drop tile and content' USING ERRCODE='23514'; END IF;
 FOR operand IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
   n:=n+1;
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(operand->'item'->>'item_instance_id')::uuid FOR UPDATE;
   SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id;
   IF i.item_instance_id IS NULL OR g.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM operand->'item'
     OR to_jsonb(g) IS DISTINCT FROM operand->'ground' OR i.lifecycle<>1 OR i.world_id<>m.world_id
     OR i.definition_family<>'Item' OR i.definition_revision_ref<>'definition-r1'
     OR i.definition_production_key<>p.input_keys[n] OR i.quantity<>p.input_counts[n]
     OR g.world_id<>m.world_id OR g.channel_id<>m.channel_id OR g.runtime_scope_ownership_generation<>m.scope_generation
     OR NOT game_item_loop_drop_ref(g.corpse_ref) OR g.stack_ordinal IS NULL
     OR (to_jsonb(g)-ARRAY['item_instance_id','corpse_ref','stack_ordinal']) IS DISTINCT FROM
        (first_ground-ARRAY['item_instance_id','corpse_ref','stack_ordinal'])
     OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'floor assembly requires exact plain physical operands' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;

CREATE FUNCTION game_floor_assembly_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_floor_assembly_receipts%ROWTYPE; p game_floor_assembly_recipes%ROWTYPE;
 m game_item_loop_move_receipts%ROWTYPE; i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE; operand JSONB;
BEGIN
 SELECT * INTO r FROM game_floor_assembly_receipts WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
 THEN RAISE EXCEPTION 'floor assembly has no current receipt' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT p FROM game_floor_assembly_recipes WHERE recipe=r.recipe;
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=r.move_transaction_id;
 FOR operand IN SELECT value FROM jsonb_array_elements(r.inputs) LOOP
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(operand->'item'->>'item_instance_id')::uuid;
   IF i.item_instance_id IS NULL OR i.lifecycle<>2 OR i.quantity<>0 OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
     OR i.state_revision<>(operand->'item'->>'state_revision')::numeric+1
     OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
        ((operand->'item')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
     OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'floor assembly input did not retire exactly once' USING ERRCODE='23514'; END IF;
 END LOOP;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.output_item_instance_id;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=r.output_item_instance_id;
 IF i.item_instance_id IS NULL OR i.world_id<>m.world_id OR i.definition_family<>'Item'
   OR i.definition_production_key<>p.output_key OR i.definition_revision_ref<>'definition-r1'
   OR i.quantity<>p.output_count OR i.lifecycle<>1 OR i.state_revision<>1
   OR i.minted_transaction_id<>r.transaction_id OR i.last_transaction_id IS NOT NULL
   OR g.item_instance_id IS NULL OR g.corpse_ref<>convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id)
   OR (to_jsonb(g)-ARRAY['item_instance_id','corpse_ref','stack_ordinal']) IS DISTINCT FROM ((r.inputs->0->'ground')-ARRAY['item_instance_id','corpse_ref','stack_ordinal'])
   OR g.stack_ordinal IS NULL OR g.stack_ordinal<=(SELECT max((v->'ground'->>'stack_ordinal')::bigint) FROM jsonb_array_elements(r.inputs) v)
   OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>1
   OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>jsonb_array_length(r.inputs)
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id AND a.event_id=r.event_id
      AND a.item_instance_id=r.output_item_instance_id AND a.envelope_sha256=r.envelope_sha256
      AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1 AND a.transaction_count=1
      AND a.occurred_at=r.occurred_at AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'floor assembly requires its unique fresh Ground output and audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_floor_assembly_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_floor_assembly_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_floor_assembly_admit BEFORE INSERT ON game_floor_assembly_receipts
 FOR EACH ROW EXECUTE FUNCTION game_floor_assembly_admit();
CREATE CONSTRAINT TRIGGER game_floor_assembly_consistent AFTER INSERT ON game_floor_assembly_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_floor_assembly_consistent();
CREATE TRIGGER game_floor_assembly_immutable BEFORE UPDATE OR DELETE ON game_floor_assembly_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_floor_assembly_no_truncate BEFORE TRUNCATE ON game_floor_assembly_receipts
 EXECUTE FUNCTION game_item_reject_truncate();

-- Add a complete proof arm; keep every original guard and receipt path intact.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
   'game_item_mint_consistency_guard','game_item_ground_insertion_guard','game_item_ground_removal_proven'] LOOP
   body:=pg_get_functiondef((n||'()')::regprocedure);
   at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing floor assembly predecessor guard %',n; END IF;
   IF n='game_item_instance_guard' THEN
     arm:=$arm$
     IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_floor_assembly_receipts r,LATERAL jsonb_array_elements(r.inputs) v
        WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=to_jsonb(OLD)) THEN
       IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
       THEN RAISE EXCEPTION 'floor assembly retires its whole exact input' USING ERRCODE='23514'; END IF;
       NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
     END IF;
     $arm$;
   ELSIF n='game_item_instance_change_proven' THEN
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_floor_assembly_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=to_jsonb(OLD)) THEN
       PERFORM game_floor_assembly_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
   ELSIF n='game_item_ground_removal_proven' THEN
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_floor_assembly_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.created_xact_id=pg_current_xact_id() AND v->'ground'=to_jsonb(OLD)) THEN
       PERFORM game_floor_assembly_complete((SELECT r.transaction_id FROM game_floor_assembly_receipts r,
          LATERAL jsonb_array_elements(r.inputs) v WHERE r.created_xact_id=pg_current_xact_id() AND v->'ground'=to_jsonb(OLD)));
       RETURN NULL;
     END IF;
     $arm$;
   ELSE
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_floor_assembly_receipts r WHERE r.output_item_instance_id=NEW.item_instance_id
       AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM game_floor_assembly_complete((SELECT transaction_id FROM game_floor_assembly_receipts
          WHERE output_item_instance_id=NEW.item_instance_id)); RETURN NULL;
     END IF;
     $arm$;
   END IF;
   EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_floor_assembly_admit()','game_floor_assembly_complete(uuid)','game_floor_assembly_consistent()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON game_floor_assembly_recipes,game_floor_assembly_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_floor_assembly_admit(),game_floor_assembly_complete(UUID),game_floor_assembly_consistent() FROM PUBLIC;
GRANT SELECT ON game_floor_assembly_recipes TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_floor_assembly_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_floor_assembly_receipts TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_floor_assembly_complete(UUID) TO oteryn_game_runtime;
