-- Current native-map owners may adopt existing script fields without reminting.
CREATE TABLE game_elemental_energy_field_adoptions (
 source_transaction_id UUID NOT NULL REFERENCES game_elemental_energy_field_receipts(transaction_id),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 holder_node_id UUID NOT NULL,
 holder_registration_revision NUMERIC(20,0) NOT NULL CHECK(holder_registration_revision>0),
 ground_before JSONB NOT NULL CHECK(jsonb_typeof(ground_before)='object' AND octet_length(ground_before::text)<=8192),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(source_transaction_id,ownership_generation)
);
CREATE FUNCTION game_elemental_energy_adoption_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_elemental_energy_field_receipts%ROWTYPE; g game_item_ground_locations%ROWTYPE;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT r FROM game_elemental_energy_field_receipts WHERE transaction_id=NEW.source_transaction_id;
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(r.world_id),'hex')||encode(uuid_send(r.channel_id),'hex'),33));
 PERFORM 1 FROM game_item_instances WHERE item_instance_id=r.item_instance_id FOR UPDATE;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id;
 IF NEW.ownership_generation<=r.scope_generation OR NOT EXISTS(
   SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards a USING(scope_key)
   WHERE s.world_id=r.world_id AND s.channel_id=r.channel_id AND s.state=1 AND s.ownership_generation=NEW.ownership_generation
     AND s.holder_node_id=NEW.holder_node_id AND s.holder_registration_revision=NEW.holder_registration_revision
     AND a.ready AND a.ownership_generation=s.ownership_generation)
 THEN RAISE EXCEPTION 'energy field adoption requires the current higher scope owner' USING ERRCODE='23514'; END IF;
 IF g.item_instance_id IS NULL OR NEW.ground_before IS DISTINCT FROM to_jsonb(g)
   OR g.runtime_scope_ownership_generation>=NEW.ownership_generation
   OR NOT game_elemental_energy_field_source(r.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=r.world_id AND c.channel_id=r.channel_id
      AND c.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=r.world_id AND channel_id=r.channel_id)
      AND r.content_revision='sha256:'||encode(c.server_artifact_digest,'hex') AND r.placement_context=c.frame_binding_digest)
 THEN RAISE EXCEPTION 'energy field adoption requires its exact live compatible predecessor' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_elemental_energy_adoption_apply() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE changed BIGINT;
BEGIN
 UPDATE game_item_ground_locations g SET runtime_scope_ownership_generation=NEW.ownership_generation
   FROM game_elemental_energy_field_receipts r
   WHERE r.transaction_id=NEW.source_transaction_id AND g.item_instance_id=r.item_instance_id
     AND to_jsonb(g)=NEW.ground_before AND NEW.created_xact_id=pg_current_xact_id();
 GET DIAGNOSTICS changed=ROW_COUNT;
 IF changed<>1 THEN RAISE EXCEPTION 'energy Ground changed during scope adoption' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_elemental_energy_adoption_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
   SELECT 1 FROM game_item_ground_locations g JOIN game_elemental_energy_field_receipts r USING(item_instance_id)
   WHERE r.transaction_id=NEW.source_transaction_id AND g.runtime_scope_ownership_generation=NEW.ownership_generation
     AND (to_jsonb(g)-'runtime_scope_ownership_generation')=(NEW.ground_before-'runtime_scope_ownership_generation'))
 THEN RAISE EXCEPTION 'energy scope adoption requires its exact Ground successor' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_elemental_energy_adoption_stamp BEFORE INSERT ON game_elemental_energy_field_adoptions FOR EACH ROW EXECUTE FUNCTION game_elemental_energy_adoption_stamp();
CREATE TRIGGER game_elemental_energy_adoption_apply AFTER INSERT ON game_elemental_energy_field_adoptions FOR EACH ROW EXECUTE FUNCTION game_elemental_energy_adoption_apply();
CREATE CONSTRAINT TRIGGER game_elemental_energy_adoption_proven AFTER INSERT ON game_elemental_energy_field_adoptions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_energy_adoption_proven();
CREATE TRIGGER game_elemental_energy_adoption_immutable BEFORE UPDATE OR DELETE ON game_elemental_energy_field_adoptions FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_elemental_energy_adoption_no_truncate BEFORE TRUNCATE ON game_elemental_energy_field_adoptions EXECUTE FUNCTION game_item_reject_truncate();
-- Extend both existing Ground UPDATE guard chains, retaining every predecessor.
DO $migration$
DECLARE name TEXT; body TEXT; at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_native_map_ground_rebind','game_item_ground_owner_lock'] LOOP
   body:=pg_get_functiondef((name||'()')::regprocedure);
   at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing energy handoff predecessor %',name; END IF;
   EXECUTE overlay(body PLACING E'BEGIN\n'||$arm$
     IF TG_OP='UPDATE'
       AND (to_jsonb(NEW)-'runtime_scope_ownership_generation')=(to_jsonb(OLD)-'runtime_scope_ownership_generation')
       AND NEW.runtime_scope_ownership_generation>OLD.runtime_scope_ownership_generation
       AND EXISTS(SELECT 1 FROM game_elemental_energy_field_adoptions a JOIN game_elemental_energy_field_receipts r ON r.transaction_id=a.source_transaction_id
         WHERE r.item_instance_id=OLD.item_instance_id AND a.ownership_generation=NEW.runtime_scope_ownership_generation
           AND a.ground_before=to_jsonb(OLD) AND a.created_xact_id=pg_current_xact_id())
     THEN RETURN NEW; END IF;
   $arm$ FROM at FOR 5);
 END LOOP;
 body:=pg_get_functiondef('game_elemental_energy_field_source(uuid)'::regprocedure);
 IF position('g.runtime_scope_ownership_generation,g.spatial_position' IN body)=0 THEN
   RAISE EXCEPTION 'missing energy source generation predecessor'; END IF;
 -- Change only the historical-generation equality; every current Item/pin check stays.
 body:=replace(body,'g.world_id,g.channel_id,g.runtime_scope_ownership_generation,g.spatial_position','g.world_id,g.channel_id,g.spatial_position');
 body:=replace(body,'r.world_id,r.channel_id,r.scope_generation,r.spatial_position','r.world_id,r.channel_id,r.spatial_position');
 body:=replace(body,'AND g.scope_kind=1',
   'AND (g.runtime_scope_ownership_generation=r.scope_generation OR EXISTS(SELECT 1 FROM game_elemental_energy_field_adoptions a WHERE a.source_transaction_id=r.transaction_id AND a.ownership_generation=g.runtime_scope_ownership_generation)) AND g.scope_kind=1');
 EXECUTE body;
END $migration$;
DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_elemental_energy_adoption_stamp()','game_elemental_energy_adoption_apply()','game_elemental_energy_adoption_proven()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
REVOKE ALL ON game_elemental_energy_field_adoptions FROM PUBLIC;
GRANT SELECT,INSERT ON game_elemental_energy_field_adoptions TO oteryn_game_runtime;
GRANT SELECT ON game_elemental_energy_field_adoptions TO oteryn_game_control;
