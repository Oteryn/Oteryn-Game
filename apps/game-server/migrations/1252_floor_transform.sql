-- A source AddItem consequence transforms an existing Item and retains its
-- original deadline per cycle. No prior Item identity or timer history is reset.
CREATE TABLE game_floor_transform_recipes (
 recipe TEXT PRIMARY KEY,
 action_id INTEGER NOT NULL CHECK(action_id BETWEEN 1 AND 65535),
 input_key TEXT NOT NULL,
 input_count INTEGER NOT NULL CHECK(input_count BETWEEN 1 AND 100),
 target_key TEXT NOT NULL CHECK(target_key<>input_key),
 output_key TEXT NOT NULL CHECK(output_key<>target_key),
 duration_millis BIGINT NOT NULL CHECK(duration_millis BETWEEN 1 AND 4294967295),
 decay_key TEXT NOT NULL,
 UNIQUE(action_id,input_key)
);
INSERT INTO game_floor_transform_recipes VALUES(
 'oteryn:floor-transform.ancient-helmet-ruby',40040,'oteryn:item.tibia.i3030',1,
 'oteryn:item.tibia.i3229','oteryn:item.tibia.i3230',1800000,'oteryn:item.tibia.i3229');
CREATE TRIGGER game_floor_transform_recipe_immutable BEFORE UPDATE OR DELETE ON game_floor_transform_recipes
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_floor_transform_recipe_no_truncate BEFORE TRUNCATE ON game_floor_transform_recipes
 EXECUTE FUNCTION game_item_reject_truncate();
CREATE TABLE game_floor_transform_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 move_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
 recipe TEXT NOT NULL REFERENCES game_floor_transform_recipes,
 input_before JSONB NOT NULL CHECK(jsonb_typeof(input_before)='object' AND octet_length(input_before::text)<=16384),
 target_before JSONB NOT NULL CHECK(jsonb_typeof(target_before)='object' AND octet_length(target_before::text)<=16384),
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 expires_at_unix_ms BIGINT NOT NULL CHECK(expires_at_unix_ms>occurred_at),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_floor_transform_recipe_idx ON game_floor_transform_receipts(recipe);
CREATE INDEX game_floor_transform_deadline_idx ON game_floor_transform_receipts(expires_at_unix_ms);
CREATE INDEX game_floor_transform_item_idx ON game_floor_transform_receipts(((target_before->'item'->>'item_instance_id')::uuid));
CREATE VIEW game_floor_transform_deadlines AS
 SELECT (r.target_before->'item'->>'item_instance_id')::uuid AS item_instance_id,
 r.transaction_id AS source_transaction_id,p.duration_millis,r.expires_at_unix_ms,
 p.decay_key AS target_production_key,r.content_digest
 FROM game_floor_transform_receipts r JOIN game_floor_transform_recipes p USING(recipe);

CREATE FUNCTION game_floor_transform_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_floor_transform_recipes%ROWTYPE; m game_item_loop_move_receipts%ROWTYPE;
 operand JSONB; i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE; first_ground JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT p FROM game_floor_transform_recipes WHERE recipe=NEW.recipe;
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=NEW.move_transaction_id;
 first_ground:=NEW.target_before->'ground';
 IF m.created_xact_id<>pg_current_xact_id() OR NOT game_world_reset_map_epoch_current(m.world_id,NEW.reset_epoch)
   OR NEW.expires_at_unix_ms<>NEW.occurred_at+p.duration_millis
   OR (NEW.input_before->'item'->>'item_instance_id')=(NEW.target_before->'item'->>'item_instance_id')
   OR first_ground->>'map_revision'<>'sha256:'||encode(substring(NEW.placement FROM 1 FOR 32),'hex')
   OR decode(substring(first_ground->>'spatial_position' FROM 3),'hex')<>
       int4send(get_byte(NEW.placement,34)*256+get_byte(NEW.placement,35))
       ||int4send(get_byte(NEW.placement,36)*256+get_byte(NEW.placement,37))
       ||int2send((-get_byte(NEW.placement,38))::smallint)
   OR NOT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=m.world_id AND a.channel_id=m.channel_id
      AND a.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=m.world_id AND channel_id=m.channel_id)
      AND NEW.content_digest=a.server_artifact_digest
      AND first_ground->>'content_revision'='sha256:'||encode(a.server_artifact_digest,'hex')
      AND first_ground->>'native_room_placement_context'='\x'||encode(a.frame_binding_digest,'hex'))
   OR NOT EXISTS(SELECT 1 FROM game_item_loop_move_lines l WHERE l.transaction_id=m.transaction_id AND l.to_kind=3
      AND l.item_instance_id=(NEW.input_before->'item'->>'item_instance_id')::uuid
      AND l.definition_family='Item' AND l.definition_production_key=p.input_key
      AND l.to_spatial_position=decode(substring(first_ground->>'spatial_position' FROM 3),'hex'))
 THEN RAISE EXCEPTION 'floor transform requires exact current drop, tile, content and deadline' USING ERRCODE='23514'; END IF;
 FOREACH operand IN ARRAY ARRAY[NEW.input_before,NEW.target_before] LOOP
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(operand->'item'->>'item_instance_id')::uuid FOR UPDATE;
   SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id;
   IF i.item_instance_id IS NULL OR g.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM operand->'item'
     OR to_jsonb(g) IS DISTINCT FROM operand->'ground' OR i.lifecycle<>1 OR i.world_id<>m.world_id
     OR i.definition_family<>'Item' OR i.definition_revision_ref<>'definition-r1'
     OR g.world_id<>m.world_id OR g.channel_id<>m.channel_id OR g.runtime_scope_ownership_generation<>m.scope_generation
     OR NOT game_item_loop_drop_ref(g.corpse_ref) OR g.stack_ordinal IS NULL
     OR (to_jsonb(g)-ARRAY['item_instance_id','corpse_ref','stack_ordinal']) IS DISTINCT FROM
        (first_ground-ARRAY['item_instance_id','corpse_ref','stack_ordinal'])
     OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules s WHERE s.item_instance_id=i.item_instance_id
        AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts d WHERE d.item_instance_id=s.item_instance_id))
     OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'floor transform requires exact unencumbered physical operands' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NEW.input_before->'item'->>'definition_production_key'<>p.input_key
   OR (NEW.input_before->'item'->>'quantity')::bigint<p.input_count
   OR NEW.target_before->'item'->>'definition_production_key'<>p.target_key
   OR (NEW.target_before->'item'->>'quantity')::bigint<>1
 THEN RAISE EXCEPTION 'floor transform operands differ from exact recipe' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_floor_transform_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_floor_transform_receipts%ROWTYPE; p game_floor_transform_recipes%ROWTYPE;
 i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE; before_item JSONB; remaining BIGINT;
BEGIN
 SELECT * INTO r FROM game_floor_transform_receipts WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
 THEN RAISE EXCEPTION 'floor transform has no current receipt' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT p FROM game_floor_transform_recipes WHERE recipe=r.recipe;
 before_item:=r.input_before->'item';
 remaining:=(before_item->>'quantity')::bigint-p.input_count;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(before_item->>'item_instance_id')::uuid;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id;
 IF i.item_instance_id IS NULL OR i.quantity<>remaining OR i.lifecycle<>(CASE WHEN remaining=0 THEN 2 ELSE 1 END)
   OR i.last_transaction_id IS DISTINCT FROM r.transaction_id OR i.state_revision<>(before_item->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
      (before_item-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
   OR (remaining=0 AND g.item_instance_id IS NOT NULL)
   OR (remaining>0 AND to_jsonb(g) IS DISTINCT FROM r.input_before->'ground')
 THEN RAISE EXCEPTION 'floor transform must consume only the exact input quantity' USING ERRCODE='23514'; END IF;
 before_item:=r.target_before->'item';
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(before_item->>'item_instance_id')::uuid;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id;
 IF i.item_instance_id IS NULL OR i.definition_production_key<>p.output_key
   OR i.last_transaction_id IS DISTINCT FROM r.transaction_id OR i.state_revision<>(before_item->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
      (before_item-ARRAY['definition_production_key','last_transaction_id','state_revision'])
   OR to_jsonb(g) IS DISTINCT FROM r.target_before->'ground'
   OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>0
   OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>2
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id AND a.event_id=r.event_id
      AND a.item_instance_id=i.item_instance_id AND a.envelope_sha256=r.envelope_sha256
      AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1 AND a.transaction_count=1
      AND a.occurred_at=r.occurred_at AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'floor transform requires same identity, placement, deadline and atomic audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_floor_transform_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_floor_transform_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_floor_transform_admit BEFORE INSERT ON game_floor_transform_receipts
 FOR EACH ROW EXECUTE FUNCTION game_floor_transform_admit();
CREATE CONSTRAINT TRIGGER game_floor_transform_consistent AFTER INSERT ON game_floor_transform_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_floor_transform_consistent();
CREATE TRIGGER game_floor_transform_immutable BEFORE UPDATE OR DELETE ON game_floor_transform_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_floor_transform_no_truncate BEFORE TRUNCATE ON game_floor_transform_receipts
 EXECUTE FUNCTION game_item_reject_truncate();

-- Only this same-XID complete proof admits the two exact successor rows.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_ground_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing floor transform predecessor %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_floor_transform_receipts r
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND (r.input_before->'item'=to_jsonb(OLD) OR r.target_before->'item'=to_jsonb(OLD))) THEN
     IF EXISTS(SELECT 1 FROM game_floor_transform_receipts r JOIN game_floor_transform_recipes p USING(recipe)
       WHERE r.transaction_id=NEW.last_transaction_id AND r.input_before->'item'=to_jsonb(OLD)) THEN
      IF (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
      THEN RAISE EXCEPTION 'floor transform input changed unrelated state' USING ERRCODE='23514'; END IF;
     ELSE
      IF (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id'])
      THEN RAISE EXCEPTION 'floor transform target changed unrelated state' USING ERRCODE='23514'; END IF;
     END IF;
     NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
    END IF;
   $arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_floor_transform_receipts r WHERE r.transaction_id=NEW.last_transaction_id
      AND r.created_xact_id=pg_current_xact_id() AND (r.input_before->'item'=to_jsonb(OLD) OR r.target_before->'item'=to_jsonb(OLD))) THEN
     PERFORM game_floor_transform_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
   $arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_floor_transform_receipts r WHERE r.created_xact_id=pg_current_xact_id()
      AND r.input_before->'ground'=to_jsonb(OLD)) THEN
     PERFORM game_floor_transform_complete((SELECT transaction_id FROM game_floor_transform_receipts r
       WHERE r.created_xact_id=pg_current_xact_id() AND r.input_before->'ground'=to_jsonb(OLD))); RETURN NULL;
    END IF;
   $arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_floor_transform_admit()','game_floor_transform_complete(uuid)','game_floor_transform_consistent()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON game_floor_transform_recipes,game_floor_transform_receipts,game_floor_transform_deadlines FROM PUBLIC;
REVOKE ALL ON FUNCTION game_floor_transform_admit(),game_floor_transform_complete(UUID),game_floor_transform_consistent() FROM PUBLIC;
GRANT SELECT ON game_floor_transform_recipes,game_floor_transform_deadlines TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_floor_transform_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_floor_transform_receipts TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_floor_transform_complete(UUID) TO oteryn_game_runtime;
