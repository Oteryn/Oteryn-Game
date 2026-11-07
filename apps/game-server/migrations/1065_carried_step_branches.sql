-- A physical STEP branch has no invented quest track. Its immutable receipt
-- proves either complete carried absence or one exact carried debit.
CREATE TABLE game_carried_step_recipes (
 recipe TEXT PRIMARY KEY,
 unique_id INTEGER NOT NULL UNIQUE CHECK(unique_id BETWEEN 1 AND 65535),
 input_key TEXT NOT NULL,
 input_count INTEGER NOT NULL CHECK(input_count BETWEEN 1 AND 100),
 success_position JSONB NOT NULL CHECK(jsonb_typeof(success_position)='array' AND jsonb_array_length(success_position)=3),
 missing_position JSONB NOT NULL CHECK(jsonb_typeof(missing_position)='array' AND jsonb_array_length(missing_position)=3)
);
INSERT INTO game_carried_step_recipes VALUES
 ('oteryn:quest-callback/ancient-tombs-key-3950',3950,'oteryn:item.tibia.i3237',1,'[33182,32714,-14]','[33231,32705,-8]'),
 ('oteryn:quest-callback/ancient-tombs-key-3951',3951,'oteryn:item.tibia.i3238',1,'[33174,32934,-15]','[33282,32744,-8]'),
 ('oteryn:quest-callback/ancient-tombs-key-3952',3952,'oteryn:item.tibia.i3240',1,'[33126,32591,-15]','[33250,32832,-8]');

-- The existing carried reader omits an over-budget tree. Absence needs the
-- stronger positive census proof, so an omitted tree can never mean no key.
CREATE FUNCTION game_carried_step_census_complete(actor UUID, world UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 WITH RECURSIVE tree(item,depth,path,cycle) AS (
  SELECT s.item_instance_id,0,ARRAY[s.item_instance_id],false FROM game_item_container_slots s
   WHERE s.character_id=actor AND s.world_id=world
  UNION ALL
  SELECT e.item,t.depth+1,t.path||e.item,e.item=ANY(t.path) FROM tree t JOIN (
   SELECT item_instance_id AS item,parent_item_instance_id AS parent FROM game_item_container_entries
    WHERE character_id=actor AND world_id=world
   UNION ALL
   SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries WHERE world_id=world
  ) e ON e.parent=t.item WHERE t.depth<9 AND NOT t.cycle
 ), bounded AS (SELECT * FROM tree LIMIT 501)
 SELECT count(*)<=500 AND NOT coalesce(bool_or(depth>8 OR cycle),false) FROM bounded
$$;

CREATE TABLE game_carried_step_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 recipe TEXT NOT NULL REFERENCES game_carried_step_recipes,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 game_session_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 character_lease_generation NUMERIC(20,0) NOT NULL CHECK(character_lease_generation>0),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>0),
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 object_revision NUMERIC(20,0) NOT NULL CHECK(object_revision>=0),
 context JSONB NOT NULL CHECK(jsonb_typeof(context)='object' AND octet_length(context::text)<=4096),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 8192),
 intent_sha256 BYTEA NOT NULL CHECK(intent_sha256=sha256(intent)),
 item_envelope_sha256 BYTEA CHECK(item_envelope_sha256 IS NULL OR octet_length(item_envelope_sha256)=32),
 input_before JSONB,
 location_before JSONB,
 location_kind TEXT CHECK(location_kind IN('container','nested')),
 occurred_at_unix_ms BIGINT NOT NULL CHECK(occurred_at_unix_ms>0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,cause_ordinal),
 CHECK(num_nonnulls(input_before,location_before,location_kind,item_envelope_sha256) IN(0,4)),
 CHECK(input_before IS NULL OR (jsonb_typeof(input_before)='object' AND octet_length(input_before::text)<=16384)),
 CHECK(location_before IS NULL OR (jsonb_typeof(location_before)='object' AND octet_length(location_before::text)<=16384))
);
CREATE INDEX game_carried_step_character_idx ON game_carried_step_receipts(character_id);
CREATE TABLE game_carried_step_audit (
 event_id UUID PRIMARY KEY REFERENCES game_carried_step_receipts(event_id),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_carried_step_receipts,
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 32768),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_carried_step_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_carried_step_recipes%ROWTYPE; i game_item_instances%ROWTYPE; v_intent JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 v_intent:=convert_from(NEW.intent,'UTF8')::jsonb;
 IF v_intent IS DISTINCT FROM jsonb_build_object('recipe',NEW.recipe,'placement',encode(NEW.placement,'hex'),
    'object_revision',NEW.object_revision,'context',NEW.context,'character',NEW.character_id::text)
  OR NOT (NEW.context ?& ARRAY['map_revision','content_revision','placement_context','reset_epoch','actor_placement','movement_revision'])
  OR jsonb_typeof(NEW.context->'reset_epoch') IS DISTINCT FROM 'number'
  OR jsonb_typeof(NEW.context->'movement_revision') IS DISTINCT FROM 'number'
  OR (NEW.context->>'movement_revision')::numeric<1
 THEN RAISE EXCEPTION 'carried STEP exact source intent required' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT p FROM game_carried_step_recipes WHERE recipe=NEW.recipe;
 IF NOT game_carried_step_census_complete(NEW.character_id,NEW.world_id)
  OR NEW.occurred_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)
  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
   JOIN game_durability_admission_character_guards g USING(character_id)
   JOIN game_character_roots c USING(character_id)
   WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id AND s.session_state=2
    AND s.current_generation=NEW.connection_generation AND s.character_lease_generation=NEW.character_lease_generation
    AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=NEW.world_id AND s.runtime_scope_channel_id=NEW.channel_id
    AND s.scope_ownership_generation=NEW.ownership_generation AND g.eligible AND g.holder_game_session_id=s.game_session_id
    AND g.lease_generation=s.character_lease_generation AND c.world_id=NEW.world_id AND c.lifecycle=1)
  OR NOT game_world_reset_map_epoch_current(NEW.world_id,(NEW.context->>'reset_epoch')::numeric)
  OR NEW.context->>'map_revision' IS NULL OR NEW.context->>'content_revision' IS NULL
  OR NOT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.channel_id
    AND 'sha256:'||encode(a.server_artifact_digest,'hex')=NEW.context->>'content_revision'
    AND (SELECT jsonb_agg(get_byte(a.frame_binding_digest,n) ORDER BY n) FROM generate_series(0,octet_length(a.frame_binding_digest)-1) n)=NEW.context->'placement_context'
    AND a.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=NEW.world_id AND channel_id=NEW.channel_id))
 THEN RAISE EXCEPTION 'carried STEP requires a complete census and current source authority' USING ERRCODE='23514'; END IF;
 IF NEW.input_before IS NULL THEN
  IF EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c
    JOIN game_item_instances owned ON owned.item_instance_id=c.item WHERE owned.lifecycle=1
     AND owned.definition_family='Item' AND owned.definition_production_key=p.input_key
     AND owned.definition_revision_ref='definition-r1' AND owned.quantity>=p.input_count)
  THEN RAISE EXCEPTION 'carried STEP cannot claim an absent exact input' USING ERRCODE='23514'; END IF;
 ELSE
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(NEW.input_before->>'item_instance_id')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM NEW.input_before OR i.world_id<>NEW.world_id
   OR i.lifecycle<>1 OR i.definition_family<>'Item' OR i.definition_production_key<>p.input_key
   OR i.definition_revision_ref<>'definition-r1' OR i.quantity<p.input_count
   OR NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c
     WHERE c.item=i.item_instance_id AND c.location=NEW.location_before AND c.kind=NEW.location_kind)
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
  THEN RAISE EXCEPTION 'carried STEP needs the exact plain carried input' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_carried_step_complete(p_transaction UUID) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE r game_carried_step_receipts%ROWTYPE; p game_carried_step_recipes%ROWTYPE;
 i game_item_instances%ROWTYPE; audit JSONB;
BEGIN
 SELECT * INTO STRICT r FROM game_carried_step_receipts WHERE transaction_id=p_transaction;
 SELECT * INTO STRICT p FROM game_carried_step_recipes WHERE recipe=r.recipe;
 IF r.created_xact_id<>pg_current_xact_id()
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>0
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.input_before IS NULL THEN 0 ELSE 1 END)
  OR NOT EXISTS(SELECT 1 FROM game_carried_step_audit a WHERE a.transaction_id=r.transaction_id AND a.event_id=r.event_id
   AND a.created_xact_id=pg_current_xact_id() AND a.envelope_sha256=sha256(a.envelope))
 THEN RAISE EXCEPTION 'carried STEP requires atomic branch audit and exact Item balance' USING ERRCODE='23514'; END IF;
 SELECT convert_from(envelope,'UTF8')::jsonb INTO STRICT audit FROM game_carried_step_audit WHERE transaction_id=r.transaction_id;
 IF audit IS DISTINCT FROM jsonb_build_object('receipt',to_jsonb(r),'recipe',to_jsonb(p))
 THEN RAISE EXCEPTION 'carried STEP audit must retain the complete branch proof' USING ERRCODE='23514'; END IF;
 IF r.input_before IS NOT NULL THEN
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(r.input_before->>'item_instance_id')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity<>(r.input_before->>'quantity')::bigint-p.input_count
   OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END) OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
   OR i.state_revision<>(r.input_before->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
      (r.input_before-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
   OR (i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
                       OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
   OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(r.character_id,r.world_id) c
      WHERE c.item=i.item_instance_id AND c.location=r.location_before AND c.kind=r.location_kind))
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id AND a.event_id=r.event_id
      AND a.item_instance_id=i.item_instance_id AND a.envelope_sha256=r.item_envelope_sha256 AND a.created_xact_id=pg_current_xact_id()
      AND a.occurred_at=r.occurred_at_unix_ms AND a.transaction_ordinal=1 AND a.transaction_count=1)
  THEN RAISE EXCEPTION 'carried STEP must debit only its exact original carried identity' USING ERRCODE='23514'; END IF;
 END IF;
END $$;
CREATE FUNCTION game_carried_step_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_carried_step_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_carried_step_admit BEFORE INSERT ON game_carried_step_receipts FOR EACH ROW EXECUTE FUNCTION game_carried_step_admit();
CREATE CONSTRAINT TRIGGER game_carried_step_consistent AFTER INSERT ON game_carried_step_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_carried_step_consistent();

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing carried STEP predecessor %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_carried_step_receipts r
      WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND r.input_before=to_jsonb(OLD)) THEN
     IF NOT EXISTS(SELECT 1 FROM game_carried_step_receipts r JOIN game_carried_step_recipes p USING(recipe)
       WHERE r.transaction_id=NEW.last_transaction_id AND NEW.quantity=OLD.quantity-p.input_count
        AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
      OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'carried STEP changes only the exact declared debit' USING ERRCODE='23514'; END IF;
     NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
    END IF;
   $arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_carried_step_receipts r WHERE r.transaction_id=NEW.last_transaction_id
      AND r.created_xact_id=pg_current_xact_id() AND r.input_before->>'item_instance_id'=NEW.item_instance_id::text) THEN
     PERFORM game_carried_step_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
   $arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_carried_step_receipts r JOIN game_item_instances i
      ON i.item_instance_id=(r.input_before->>'item_instance_id')::uuid
      WHERE r.created_xact_id=pg_current_xact_id() AND r.location_before=to_jsonb(OLD)
       AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id) THEN
     RETURN NULL;
    END IF;
   $arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
END $migration$;

DO $migration$
DECLARE t TEXT; f TEXT;
BEGIN
 FOREACH t IN ARRAY ARRAY['game_carried_step_recipes','game_carried_step_receipts','game_carried_step_audit'] LOOP
  EXECUTE format('CREATE TRIGGER carried_step_immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',t);
  EXECUTE format('CREATE TRIGGER carried_step_no_truncate BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_item_reject_truncate()',t);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC',t);
  EXECUTE format('GRANT SELECT ON %I TO oteryn_game_runtime,oteryn_game_control',t);
 END LOOP;
 GRANT INSERT ON game_carried_step_receipts,game_carried_step_audit TO oteryn_game_runtime;
 FOREACH f IN ARRAY ARRAY['game_carried_step_census_complete(uuid,uuid)','game_carried_step_admit()',
   'game_carried_step_complete(uuid)','game_carried_step_consistent()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $migration$;
