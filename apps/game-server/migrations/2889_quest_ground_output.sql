-- BRAINOUT: a portable Ground output shares its input BURNs and quest receipt.
CREATE TABLE game_quest_ground_outputs (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances DEFERRABLE INITIALLY DEFERRED,
 character_id UUID NOT NULL, cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
 callback JSONB NOT NULL CHECK(jsonb_typeof(callback)='object' AND octet_length(callback::text)<=16384),
 context JSONB NOT NULL CHECK(jsonb_typeof(context)='object' AND octet_length(context::text)<=8192),
 world_id UUID NOT NULL, channel_id UUID NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 spatial_position BYTEA NOT NULL CHECK(octet_length(spatial_position)=10),
 corpse_ref BYTEA NOT NULL CHECK(game_item_loop_drop_ref(corpse_ref)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 65536),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
  REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key) DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_quest_ground_output_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE output JSONB; offering JSONB; input game_quest_callback_consumptions%ROWTYPE; proof JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 output:=NEW.callback->'effects'->'ground_output';
 IF jsonb_typeof(output) IS DISTINCT FROM 'object' OR NEW.callback->>'key' IS DISTINCT FROM NEW.transition_key
    OR NEW.callback->>'trigger' IS DISTINCT FROM 'use'
    OR jsonb_typeof(NEW.callback->'offerings') IS DISTINCT FROM 'array'
    OR jsonb_array_length(NEW.callback->'offerings') NOT BETWEEN 1 AND 8
    OR coalesce((output->>'count')::bigint,0) NOT BETWEEN 1 AND 100
    OR output->>'item' IS NULL OR output->>'item' !~ '^oteryn:item[.]tibia[.]i[0-9]+$'
    OR NEW.spatial_position IS DISTINCT FROM int4send((output->'position'->>'x')::int)
        ||int4send((output->'position'->>'y')::int)||int2send((output->'position'->>'floor')::smallint)
    OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key)
       WHERE s.world_id=NEW.world_id AND s.channel_id=NEW.channel_id AND s.state=1
         AND s.ownership_generation=NEW.scope_generation AND g.ready AND g.ownership_generation=s.ownership_generation)
 THEN RAISE EXCEPTION 'Ground output requires a current bounded source callback' USING ERRCODE='23514'; END IF;
 IF (SELECT count(*) FROM game_quest_callback_consumptions c WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)
       =(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key))<>jsonb_array_length(NEW.callback->'offerings')
 THEN RAISE EXCEPTION 'Ground output must account for every input' USING ERRCODE='23514'; END IF;
 FOR offering IN SELECT value FROM jsonb_array_elements(NEW.callback->'offerings') LOOP
   SELECT * INTO input FROM game_quest_callback_consumptions c
    WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
      AND c.location_kind='ground' AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id()
      AND c.before_item->>'definition_production_key'=offering->>'item'
      AND c.quantity=(offering->>'count')::int
      AND c.before_ground->>'spatial_position'='\x'||encode(int4send((offering->'position'->>'x')::int)
         ||int4send((offering->'position'->>'y')::int)||int2send((offering->'position'->>'floor')::smallint),'hex');
   IF input.item_instance_id IS NULL THEN RAISE EXCEPTION 'Ground output lacks its exact input' USING ERRCODE='23514'; END IF;
   proof:=convert_from(input.proof,'UTF8')::jsonb;
   IF proof->'callback' IS DISTINCT FROM NEW.callback OR proof->'context' IS DISTINCT FROM NEW.context
      OR input.before_ground->>'world_id' IS DISTINCT FROM NEW.world_id::text
      OR input.before_ground->>'channel_id' IS DISTINCT FROM NEW.channel_id::text
      OR (input.before_ground->>'runtime_scope_ownership_generation')::numeric IS DISTINCT FROM NEW.scope_generation
      OR input.before_ground->>'map_revision' IS DISTINCT FROM NEW.context->>'map_revision'
      OR input.before_ground->>'content_revision' IS DISTINCT FROM NEW.context->>'content_revision'
   THEN RAISE EXCEPTION 'Ground output input provenance differs' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE FUNCTION game_quest_ground_output_complete(p_item UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_quest_ground_outputs%ROWTYPE; i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE; output JSONB;
BEGIN
 SELECT * INTO STRICT r FROM game_quest_ground_outputs WHERE item_instance_id=p_item;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id;
 output:=r.callback->'effects'->'ground_output';
 IF r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL OR g.item_instance_id IS NULL
   OR (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,i.state_revision)
      IS DISTINCT FROM (r.world_id,'Item'::text,output->>'item','definition-r1'::text,(output->>'count')::bigint,1::smallint,1::numeric)
   OR i.minted_transaction_id IS DISTINCT FROM r.transaction_id OR i.last_transaction_id IS NOT NULL
   OR (g.world_id,g.channel_id,g.runtime_scope_ownership_generation,g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision)
      IS DISTINCT FROM (r.world_id,r.channel_id,r.scope_generation,r.spatial_position,r.corpse_ref,r.context->>'map_revision',r.context->>'content_revision')
   OR to_jsonb(ARRAY(SELECT get_byte(g.native_room_placement_context,n) FROM generate_series(0,octet_length(g.native_room_placement_context)-1) n)) IS DISTINCT FROM r.context->'placement_context'
   OR g.scope_kind<>1 OR g.instance_id IS NOT NULL OR g.stack_ordinal IS NULL
   OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q WHERE (q.character_id,q.cause_id,q.cause_ordinal,q.transition_key)
       =(r.character_id,r.cause_id,r.cause_ordinal,r.transition_key) AND q.cause_kind='use' AND q.created_xact_id=pg_current_xact_id())
   OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>1
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.transaction_id=r.transaction_id AND o.event_id=r.event_id
       AND o.item_instance_id=r.item_instance_id AND o.envelope=r.envelope AND o.envelope_sha256=sha256(r.envelope)
       AND o.occurred_at=r.occurred_at AND o.created_xact_id=pg_current_xact_id() AND o.transaction_ordinal=1 AND o.transaction_count=1
       AND o.publication_state=1 AND o.published_at IS NULL)
 THEN RAISE EXCEPTION 'incomplete atomic quest Ground output' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_quest_ground_output_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_quest_ground_output_complete(NEW.item_instance_id); RETURN NULL; END $$;
CREATE TRIGGER game_quest_ground_output_admit BEFORE INSERT ON game_quest_ground_outputs FOR EACH ROW EXECUTE FUNCTION game_quest_ground_output_admit();
CREATE CONSTRAINT TRIGGER game_quest_ground_output_proven AFTER INSERT ON game_quest_ground_outputs DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_ground_output_proven();
CREATE TRIGGER game_quest_ground_output_immutable BEFORE UPDATE OR DELETE ON game_quest_ground_outputs FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_ground_output_no_truncate BEFORE TRUNCATE ON game_quest_ground_outputs EXECUTE FUNCTION game_item_reject_truncate();
-- Preserve every predecessor arm in the established MINT and Ground guard chains.
DO $migration$
DECLARE name TEXT; body TEXT; at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_mint_consistency_guard','game_item_ground_insertion_guard'] LOOP
   body:=pg_get_functiondef((name||'()')::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing Ground output predecessor %',name; END IF;
 EXECUTE overlay(body PLACING E'BEGIN\n'||$arm$
     IF EXISTS(SELECT 1 FROM game_quest_ground_outputs r WHERE r.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM game_quest_ground_output_complete(NEW.item_instance_id); RETURN NULL;
     END IF;
   $arm$ FROM at FOR 5);
 END LOOP;
END $migration$;
-- A ground-output input proof cannot commit without the corresponding output.
DO $migration$
DECLARE body TEXT; at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing quest consumption predecessor'; END IF;
 EXECUTE overlay(body PLACING E'BEGIN\n'||$arm$
   IF convert_from(NEW.proof,'UTF8')::jsonb->'callback'->'effects'->'ground_output' IS NOT NULL THEN
     IF NOT EXISTS(SELECT 1 FROM game_quest_ground_outputs r WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)
       =(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key) AND r.created_xact_id=pg_current_xact_id())
     THEN RAISE EXCEPTION 'quest Ground inputs require their atomic output' USING ERRCODE='23514'; END IF;
     PERFORM game_quest_ground_output_complete((SELECT item_instance_id FROM game_quest_ground_outputs r
       WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)));
   END IF;
 $arm$ FROM at FOR 5);
END $migration$;
DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_quest_ground_output_admit()','game_quest_ground_output_complete(uuid)','game_quest_ground_output_proven()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
REVOKE ALL ON game_quest_ground_outputs FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_ground_outputs TO oteryn_game_runtime;
GRANT SELECT ON game_quest_ground_outputs TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_quest_ground_output_complete(UUID) TO oteryn_game_runtime;
