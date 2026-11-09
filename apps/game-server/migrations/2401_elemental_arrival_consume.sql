-- M49: an AddItem successor shares the ordinary drop's physical transaction.
CREATE TABLE game_elemental_arrival_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 move_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
 corpse_item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances,
 altar_id INTEGER NOT NULL REFERENCES game_elemental_arrival_rules,
 corpse_before JSONB NOT NULL CHECK(jsonb_typeof(corpse_before)='object' AND octet_length(corpse_before::text)<=8192),
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 readset JSONB NOT NULL CHECK(jsonb_typeof(readset)='array' AND jsonb_array_length(readset)=8 AND octet_length(readset::text)<=8192),
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(transaction_id<>move_transaction_id)
);
CREATE TABLE game_elemental_summon_intents (
 transaction_id UUID PRIMARY KEY REFERENCES game_elemental_arrival_receipts,
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 altar_id INTEGER NOT NULL REFERENCES game_elemental_arrival_rules,
 creature_key TEXT NOT NULL,
 summon_position JSONB NOT NULL,
 map_revision TEXT NOT NULL,
 content_revision TEXT NOT NULL,
 placement_context BYTEA NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
-- A later proven death releases the lease; the intent itself remains immutable.
CREATE TABLE game_elemental_boss_leases (
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 altar_id INTEGER NOT NULL REFERENCES game_elemental_arrival_rules,
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_elemental_summon_intents,
 PRIMARY KEY(world_id,channel_id,reset_epoch,altar_id)
);
CREATE FUNCTION game_elemental_arrival_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE m game_item_loop_move_receipts%ROWTYPE; p game_elemental_arrival_rules%ROWTYPE;
 i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE; k NUMERIC:=0; n INTEGER;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=NEW.move_transaction_id;
 SELECT * INTO STRICT p FROM game_elemental_arrival_rules WHERE altar_id=NEW.altar_id;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.corpse_item_instance_id FOR UPDATE;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=NEW.corpse_item_instance_id;
 FOR n IN 32..39 LOOP k:=k*256+get_byte(NEW.placement,n); END LOOP;
 IF m.created_xact_id<>pg_current_xact_id() OR NOT game_world_reset_map_epoch_current(m.world_id,NEW.reset_epoch)
   OR NOT game_elemental_arrival_readset_valid(NEW.altar_id,k,NEW.readset)
   OR i.item_instance_id IS NULL OR g.item_instance_id IS NULL
   OR NEW.corpse_before IS DISTINCT FROM jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g))
   OR i.lifecycle<>1 OR i.quantity<>1 OR i.world_id<>m.world_id OR i.definition_family<>'Item'
   OR i.definition_production_key<>p.corpse_key OR i.definition_revision_ref<>'definition-r1'
   OR g.world_id<>m.world_id OR g.channel_id<>m.channel_id OR g.runtime_scope_ownership_generation<>m.scope_generation
   OR NOT game_item_loop_drop_ref(g.corpse_ref) OR g.stack_ordinal IS NULL
   OR g.map_revision<>'sha256:'||encode(substring(NEW.placement FROM 1 FOR 32),'hex')
   OR get_byte(NEW.placement,32)<>0 OR get_byte(NEW.placement,33)<>0 OR get_byte(NEW.placement,38)>15 OR get_byte(NEW.placement,39)>63
   OR g.spatial_position<>int4send(get_byte(NEW.placement,34)*256+get_byte(NEW.placement,35))
       ||int4send(get_byte(NEW.placement,36)*256+get_byte(NEW.placement,37))||int2send((-get_byte(NEW.placement,38))::smallint)
   OR NOT EXISTS(SELECT 1 FROM game_item_loop_move_lines l WHERE l.transaction_id=m.transaction_id
       AND l.item_instance_id=i.item_instance_id AND l.to_kind=3 AND l.quantity_after=1
       AND l.definition_family='Item' AND l.definition_production_key=p.corpse_key AND l.to_spatial_position=g.spatial_position)
   OR NOT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=m.world_id AND a.channel_id=m.channel_id
       AND a.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=m.world_id AND channel_id=m.channel_id)
       AND g.content_revision='sha256:'||encode(a.server_artifact_digest,'hex') AND g.native_room_placement_context=a.frame_binding_digest)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'elemental arrival requires the exact current whole corpse drop and canonical ring' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_elemental_intent_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_elemental_arrival_receipts%ROWTYPE; m game_item_loop_move_receipts%ROWTYPE; p game_elemental_arrival_rules%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_elemental_arrival_receipts WHERE transaction_id=NEW.transaction_id;
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=r.move_transaction_id;
 SELECT * INTO STRICT p FROM game_elemental_arrival_rules WHERE altar_id=r.altar_id;
 IF r.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'summon intent requires a current consumption receipt' USING ERRCODE='23514'; END IF;
 NEW.world_id:=m.world_id; NEW.channel_id:=m.channel_id; NEW.scope_generation:=m.scope_generation;
 NEW.reset_epoch:=r.reset_epoch; NEW.altar_id:=r.altar_id; NEW.creature_key:=p.creature_key; NEW.summon_position:=p.summon_position;
 NEW.map_revision:=r.corpse_before->'ground'->>'map_revision'; NEW.content_revision:=r.corpse_before->'ground'->>'content_revision';
 NEW.placement_context:=decode(substring(r.corpse_before->'ground'->>'native_room_placement_context' FROM 3),'hex');
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE FUNCTION game_elemental_arrival_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_elemental_arrival_receipts%ROWTYPE; i game_item_instances%ROWTYPE; b game_elemental_summon_intents%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_elemental_arrival_receipts WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'missing current elemental receipt' USING ERRCODE='23514'; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.corpse_item_instance_id;
 SELECT * INTO b FROM game_elemental_summon_intents WHERE transaction_id=p_transaction;
 IF i.lifecycle<>2 OR i.quantity<>0 OR i.last_transaction_id<>p_transaction
   OR i.state_revision<>(r.corpse_before->'item'->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
      ((r.corpse_before->'item')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
   OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
   OR b.transaction_id IS NULL OR b.created_xact_id<>pg_current_xact_id()
   OR NOT EXISTS(SELECT 1 FROM game_elemental_boss_leases l WHERE l.transaction_id=b.transaction_id AND l.world_id=b.world_id
       AND l.channel_id=b.channel_id AND l.reset_epoch=b.reset_epoch AND l.altar_id=b.altar_id)
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_transaction AND a.event_id=r.event_id
       AND a.item_instance_id=i.item_instance_id AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=pg_current_xact_id()
       AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.occurred_at=r.occurred_at AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'elemental arrival must retire its whole corpse with one summon lease and audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_elemental_arrival_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_elemental_arrival_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE FUNCTION game_elemental_lease_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE b game_elemental_summon_intents%ROWTYPE;
BEGIN
 SELECT * INTO STRICT b FROM game_elemental_summon_intents WHERE transaction_id=NEW.transaction_id;
 IF b.created_xact_id<>pg_current_xact_id() OR (NEW.world_id,NEW.channel_id,NEW.reset_epoch,NEW.altar_id) IS DISTINCT FROM
    (b.world_id,b.channel_id,b.reset_epoch,b.altar_id) THEN RAISE EXCEPTION 'elemental boss lease needs its current intent' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_elemental_arrival_admit BEFORE INSERT ON game_elemental_arrival_receipts FOR EACH ROW EXECUTE FUNCTION game_elemental_arrival_admit();
CREATE TRIGGER game_elemental_intent_admit BEFORE INSERT ON game_elemental_summon_intents FOR EACH ROW EXECUTE FUNCTION game_elemental_intent_admit();
CREATE TRIGGER game_elemental_lease_admit BEFORE INSERT ON game_elemental_boss_leases FOR EACH ROW EXECUTE FUNCTION game_elemental_lease_admit();
CREATE CONSTRAINT TRIGGER game_elemental_arrival_consistent AFTER INSERT ON game_elemental_arrival_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_arrival_consistent();
DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_elemental_arrival_receipts','game_elemental_summon_intents','game_elemental_boss_leases'] LOOP
   EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',name||'_immutable',name);
   EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_item_reject_truncate()',name||'_no_truncate',name);
 END LOOP;
END $migration$;
-- Extend only the retired-input proof arm; retain every predecessor path.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_ground_removal_proven'] LOOP
   body:=pg_get_functiondef((name||'()')::regprocedure);
   at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing elemental predecessor %',name; END IF;
   IF name='game_item_instance_guard' THEN
     arm:=$arm$
     IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_elemental_arrival_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.corpse_before->'item'=to_jsonb(OLD)) THEN
       IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']) THEN RAISE EXCEPTION 'elemental arrival retires its exact whole corpse' USING ERRCODE='23514'; END IF;
       NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
     END IF;
     $arm$;
   ELSIF name='game_item_instance_change_proven' THEN
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_arrival_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.corpse_before->'item'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_arrival_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
   ELSE
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_arrival_receipts r WHERE r.created_xact_id=pg_current_xact_id()
       AND r.corpse_before->'ground'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_arrival_complete((SELECT transaction_id FROM game_elemental_arrival_receipts
         WHERE created_xact_id=pg_current_xact_id() AND corpse_before->'ground'=to_jsonb(OLD))); RETURN NULL;
     END IF;
     $arm$;
   END IF;
   EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
END $migration$;

CREATE FUNCTION game_elemental_arrival_consume(p_move UUID,p_transaction UUID,p_event UUID,p_item UUID,
 p_state NUMERIC,p_altar INTEGER,p_placement BYTEA,p_readset JSONB,p_reset BIGINT,p_occurred BIGINT,
 p_envelope BYTEA,p_schema BIGINT,p_retention TEXT) RETURNS void LANGUAGE plpgsql AS $$
DECLARE before JSONB;
BEGIN
 SELECT jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) INTO before
   FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
   WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.state_revision=p_state FOR UPDATE OF i;
 IF before IS NULL THEN RAISE EXCEPTION 'elemental corpse revision is stale' USING ERRCODE='23514'; END IF;
 SET CONSTRAINTS ALL DEFERRED;
 INSERT INTO game_elemental_arrival_receipts(transaction_id,event_id,move_transaction_id,corpse_item_instance_id,altar_id,
   corpse_before,placement,readset,reset_epoch,occurred_at,envelope_sha256)
 VALUES(p_transaction,p_event,p_move,p_item,p_altar,before,p_placement,p_readset,p_reset,p_occurred,sha256(p_envelope));
 UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=p_transaction WHERE item_instance_id=p_item;
 DELETE FROM game_item_ground_locations WHERE item_instance_id=p_item;
 INSERT INTO game_elemental_summon_intents(transaction_id) VALUES(p_transaction);
 INSERT INTO game_elemental_boss_leases(world_id,channel_id,reset_epoch,altar_id,transaction_id)
   SELECT world_id,channel_id,reset_epoch,altar_id,transaction_id FROM game_elemental_summon_intents WHERE transaction_id=p_transaction;
 INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,
   retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state)
 VALUES(p_event,p_transaction,1,1,2,p_schema,p_retention,p_item,p_occurred,p_occurred+7776000000,p_envelope,sha256(p_envelope),1);
END $$;
DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_elemental_arrival_admit()','game_elemental_intent_admit()',
  'game_elemental_arrival_complete(uuid)','game_elemental_arrival_consistent()','game_elemental_lease_admit()',
  'game_elemental_arrival_consume(uuid,uuid,uuid,uuid,numeric,integer,bytea,jsonb,bigint,bigint,bytea,bigint,text)'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
REVOKE ALL ON game_elemental_arrival_receipts,game_elemental_summon_intents,game_elemental_boss_leases FROM PUBLIC;
GRANT SELECT,INSERT ON game_elemental_arrival_receipts,game_elemental_summon_intents,game_elemental_boss_leases TO oteryn_game_runtime;
GRANT SELECT ON game_elemental_arrival_receipts,game_elemental_summon_intents,game_elemental_boss_leases TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_elemental_arrival_complete(UUID),game_elemental_arrival_consume(UUID,UUID,UUID,UUID,NUMERIC,INTEGER,BYTEA,JSONB,BIGINT,BIGINT,BYTEA,BIGINT,TEXT) TO oteryn_game_runtime;
