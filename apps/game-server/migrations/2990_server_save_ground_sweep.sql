-- A Berlin save occurrence is bound to the epoch that is active at publication.
-- Channel ownership generations fence writes, but are not part of the cohort key.
CREATE TABLE game_server_save_occurrences (
 occurrence_id uuid PRIMARY KEY DEFAULT uuidv7(),
 world_id uuid NOT NULL,
 reset_epoch numeric(20,0) NOT NULL,
 scheduled_at timestamptz NOT NULL,
 freeze_until timestamptz NOT NULL CHECK (freeze_until > scheduled_at),
 map_revision text NOT NULL,
 server_digest bytea NOT NULL CHECK (octet_length(server_digest)=32),
 frame_digest bytea NOT NULL CHECK (octet_length(frame_digest)=32),
 cleanup_enabled boolean NOT NULL DEFAULT false,
 clean_protection_zones boolean NOT NULL DEFAULT false,
 UNIQUE (world_id, scheduled_at),
 FOREIGN KEY (world_id,reset_epoch) REFERENCES game_world_reset_epochs(world_id,reset_epoch)
);
CREATE INDEX game_server_save_epoch ON game_server_save_occurrences(world_id,reset_epoch,scheduled_at DESC);
CREATE TRIGGER game_server_save_occurrence_immutable BEFORE UPDATE OR DELETE
 ON game_server_save_occurrences FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE TABLE game_server_save_channels (
 occurrence_id uuid NOT NULL REFERENCES game_server_save_occurrences,
 channel_id uuid NOT NULL,
 state text NOT NULL CHECK (state IN ('frozen','complete','missed')),
 PRIMARY KEY (occurrence_id,channel_id)
);
CREATE TABLE game_server_save_ground (
 occurrence_id uuid NOT NULL,
 channel_id uuid NOT NULL,
 item_id uuid NOT NULL,
 evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence)='object'),
 disposition text NOT NULL CHECK (disposition IN ('pending','preserved','unknown','skipped','retired')),
 reason text,
 transaction_id uuid UNIQUE REFERENCES game_spell_item_receipts,
 FOREIGN KEY (occurrence_id,channel_id) REFERENCES game_server_save_channels,
 CHECK ((disposition='retired')=(transaction_id IS NOT NULL)),
 CHECK ((disposition IN ('preserved','unknown','skipped'))=(reason IS NOT NULL)),
 PRIMARY KEY (occurrence_id,channel_id,item_id)
);
CREATE INDEX game_server_save_ground_pending ON game_server_save_ground(occurrence_id,channel_id,item_id)
 WHERE disposition='pending';

REVOKE ALL ON game_server_save_occurrences,game_server_save_channels,game_server_save_ground FROM PUBLIC;
GRANT SELECT ON game_server_save_occurrences TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_server_save_channels,game_server_save_ground TO oteryn_game_runtime;
GRANT SELECT ON game_server_save_occurrences,game_server_save_channels,game_server_save_ground TO oteryn_game_control;

-- The save owner uses database time and a one-minute admission window.
-- The last Berlin slot is recovered after restart even if its window expired.
CREATE FUNCTION game_server_save_publish_due(p_world uuid,p_epoch numeric,p_cleanup boolean,p_protection boolean)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 observed timestamptz;
 scheduled timestamptz;
BEGIN
 IF NOT game_world_reset_map_epoch_current(p_world,p_epoch) THEN RETURN; END IF;
 observed:=clock_timestamp();
 scheduled:=(date_trunc('day',(observed AT TIME ZONE 'Europe/Berlin')-interval '6 hours')
             +interval '6 hours') AT TIME ZONE 'Europe/Berlin';
 INSERT INTO game_server_save_occurrences
   (world_id,reset_epoch,scheduled_at,freeze_until,map_revision,server_digest,frame_digest,cleanup_enabled,clean_protection_zones)
 SELECT e.world_id,e.reset_epoch,scheduled,scheduled+interval '1 minute',e.map_revision,e.server_digest,e.frame_digest,p_cleanup,p_protection
 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING (world_id,reset_epoch)
 WHERE a.world_id=p_world AND a.reset_epoch=p_epoch AND e.calendar='TIBIA_SERVER_SAVE_BERLIN_06'
   AND e.activated_at<=scheduled
 ON CONFLICT (world_id,scheduled_at) DO NOTHING;
END $$;

-- Pin the current Channel issuance and the positive House exclusion against writers.
CREATE FUNCTION game_server_save_pin(p_world uuid,p_channel uuid,p_epoch numeric,p_map text,p_server bytea,p_frame bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT game_world_reset_map_epoch_current(p_world,p_epoch) THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:'||p_world::text||':'||p_channel::text,0));
 LOCK TABLE game_house_tiles IN SHARE MODE;
 RETURN EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
    WHERE a.world_id=p_world AND a.reset_epoch=p_epoch
     AND (e.map_revision,e.server_digest,e.frame_digest)=(p_map,p_server,p_frame))
   AND EXISTS(SELECT 1 FROM (
      SELECT server_artifact_digest,frame_binding_digest FROM game_content_activations
      WHERE world_id=p_world AND channel_id=p_channel ORDER BY activation_sequence DESC LIMIT 1
    ) current_pin WHERE (server_artifact_digest,frame_binding_digest)=(p_server,p_frame));
END $$;

CREATE FUNCTION game_server_save_lock_root(p_item uuid,p_evidence jsonb)
RETURNS boolean LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING (item_instance_id,world_id)
   WHERE g.item_instance_id=p_item AND to_jsonb(i)=p_evidence->'item' AND to_jsonb(g)=p_evidence->'ground'
   FOR UPDATE OF i,g)
$$;

DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY['game_server_save_publish_due(uuid,numeric,boolean,boolean)'::regprocedure,
                           'game_server_save_pin(uuid,uuid,numeric,text,bytea,bytea)'::regprocedure,
                           'game_server_save_lock_root(uuid,jsonb)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;

-- Both server-owned causes use Feroxa's child-first physical Item writer.
-- The caller supplies the locked rows and its own typed receipt and progress proof.
CREATE FUNCTION game_retire_ground_tree_batch(txid uuid,ids uuid[],children jsonb,retained_root boolean)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE receipt game_spell_item_receipts%ROWTYPE; r record; ordinal integer:=0;
BEGIN
 SELECT * INTO STRICT receipt FROM game_spell_item_receipts
 WHERE transaction_id=txid AND created_xact_id=pg_current_xact_id() AND cause_kind IN (5,7);
  -- Children must retire while the actual Ground root is still live.
  -- Deferred operation6 guards prove a retired root line or this retained live root.
  FOR r IN SELECT * FROM jsonb_to_recordset(children) AS child(
   item_instance_id uuid,definition_family text,definition_production_key text,definition_revision_ref text,
   quantity bigint,state_revision numeric,spatial_position bytea,map_revision text,content_revision text,
   native_room_placement_context bytea,source_parent_item_instance_id uuid,source_placement_ordinal numeric,
   source_corpse_root uuid,source_entry_is_corpse boolean,depth integer)
   ORDER BY depth DESC,item_instance_id LOOP
   ordinal:=ordinal+1;
   INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,
    definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,
    world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,
    content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid,
    source_parent_item_instance_id,source_placement_ordinal,source_corpse_root,source_entry_is_corpse)
    VALUES(txid,ordinal,6,r.item_instance_id,r.definition_family,r.definition_production_key,r.definition_revision_ref,
      r.quantity,0,r.state_revision,receipt.world_id,receipt.channel_id,r.spatial_position,r.map_revision,r.content_revision,
      r.native_room_placement_context,receipt.catalog_digest,false,false,false,
      r.source_parent_item_instance_id,r.source_placement_ordinal,r.source_corpse_root,r.source_entry_is_corpse);
   UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=txid WHERE item_instance_id=r.item_instance_id;
   IF r.source_entry_is_corpse THEN
    DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=r.item_instance_id;
   ELSE
    DELETE FROM game_item_nested_entries WHERE item_instance_id=r.item_instance_id;
   END IF;
  END LOOP;
  FOR r IN SELECT i.*,g.stack_ordinal,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context
    FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=ANY(ids) AND NOT retained_root ORDER BY i.item_instance_id LOOP
   ordinal:=ordinal+1;
   INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,
    definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,
    world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,source_stack_ordinal,
    content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid)
    VALUES(txid,ordinal,2,r.item_instance_id,r.definition_family,r.definition_production_key,r.definition_revision_ref,
      r.quantity,0,r.state_revision,receipt.world_id,receipt.channel_id,r.spatial_position,r.map_revision,r.content_revision,
      r.native_room_placement_context,r.stack_ordinal,receipt.catalog_digest,false,false,false);
   UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=txid WHERE item_instance_id=r.item_instance_id;
   DELETE FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id;
  END LOOP;
  INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256)
    VALUES(receipt.event_id,txid,receipt.occurred_at_unix_ms,receipt.occurred_at_unix_ms+7776000000,receipt.intent,receipt.binding);
END $$;
REVOKE ALL ON FUNCTION game_retire_ground_tree_batch(uuid,uuid[],jsonb,boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_retire_ground_tree_batch(uuid,uuid[],jsonb,boolean) TO oteryn_game_runtime;

-- Replace only the installed physical writer block. Feroxa retains its phase,
-- work bound, large-tree behavior, selection and audit format.
DO $migration$
DECLARE definition text; source text; start_at integer; end_at integer;
BEGIN
 SELECT pg_get_functiondef(oid),prosrc INTO STRICT definition,source FROM pg_proc
 WHERE oid='game_feroxa_clear_ground_batch(uuid,uuid,uuid,numeric,smallint,bigint,text,text,bytea,bytea)'::regprocedure;
 start_at:=strpos(source,'  -- Children must retire while');
 end_at:=strpos(source,E'\n END IF;\n SELECT NOT EXISTS');
 IF start_at=0 OR end_at<=start_at THEN RAISE EXCEPTION 'Feroxa Ground tree writer changed'; END IF;
 EXECUTE replace(definition,source,
   overlay(source placing '  PERFORM game_retire_ground_tree_batch(txid,ids,children,retained_root);'
    from start_at for end_at-start_at));
END $migration$;

ALTER TABLE game_spell_item_receipts
 ADD COLUMN server_save_occurrence_id uuid REFERENCES game_server_save_occurrences,
 ADD CONSTRAINT game_spell_item_save_cause_id CHECK ((cause_kind=7)=(server_save_occurrence_id IS NOT NULL)),
 DROP CONSTRAINT game_spell_item_receipts_cause_kind_check,
 ADD CONSTRAINT game_spell_item_receipts_cause_kind_check CHECK (cause_kind BETWEEN 0 AND 7);
CREATE INDEX game_spell_item_save_publication ON game_spell_item_receipts(world_id,channel_id,transaction_id DESC)
 WHERE cause_kind=7;
CREATE INDEX game_spell_item_save_occurrence ON game_spell_item_receipts(server_save_occurrence_id)
 WHERE server_save_occurrence_id IS NOT NULL;

DO $migration$
DECLARE name text; prior text; definition text; patched text; f regprocedure;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
  WHERE conrelid='game_spell_item_receipts'::regclass AND conname=name;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR (
    cause_kind=7 AND spell_family=''WorldEvent'' AND spell_production_key=''oteryn:world-event.server-save.ground-sweep''
    AND spell_revision=''definition-r1'' AND runtime_scope_kind=1 AND channel_id IS NOT NULL
    AND operation_count BETWEEN 1 AND 256 AND
    num_nonnulls(instance_id,game_session_id,command_id,character_id,cost,cost_binding,caster_lease_generation,
      caster_placement_digest,swing_ordinal,creature_placement,creature_actor_generation,
      creature_definition_key,creature_definition_revision,creature_occurrence,creature_map_digest,
      creature_frame_digest,creature_source_body_digest,creature_cast_digest,
      feroxa_firing_id,feroxa_phase,feroxa_reset_epoch)=0)) IS TRUE)',name,prior);
 END LOOP;
 -- Both typed owners may retire ordinary nested Ground custody. The
 -- Feroxa-only retained-root proof stays Feroxa-only.
 FOREACH f IN ARRAY ARRAY['game_spell_item_capture_custody()'::regprocedure,
                           'game_spell_item_line_proven()'::regprocedure,
                           'game_item_nested_entry_proven()'::regprocedure] LOOP
  definition:=pg_get_functiondef(f);
  patched:=regexp_replace(definition,'(r|event)\.cause_kind[[:space:]]*=[[:space:]]*5','\1.cause_kind IN (5,7)','g');
  IF patched=definition THEN RAISE EXCEPTION 'missing Ground-tree custody proof in %',f; END IF;
  EXECUTE patched;
 END LOOP;
END $migration$;

CREATE FUNCTION game_server_save_retirement_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p jsonb;
BEGIN
 IF NEW.cause_kind<>7 THEN RETURN NULL; END IF;
 p:=convert_from(NEW.intent,'UTF8')::jsonb;
 IF NOT EXISTS(
   SELECT 1 FROM game_server_save_ground g JOIN game_server_save_occurrences s USING (occurrence_id)
   JOIN game_spell_item_lines root ON root.transaction_id=NEW.transaction_id AND root.item_instance_id=g.item_id
   WHERE g.occurrence_id=NEW.server_save_occurrence_id AND g.channel_id=NEW.channel_id
    AND s.world_id=NEW.world_id AND s.cleanup_enabled AND s.server_digest=NEW.catalog_digest
    AND g.transaction_id=NEW.transaction_id AND g.disposition='retired'
    AND game_world_reset_map_epoch_current(s.world_id,s.reset_epoch)
    AND root.operation_kind=2 AND root.placement_context=s.frame_digest
    AND p=jsonb_build_object('schema','OTERYN_SERVER_SAVE_GROUND/v1','occurrence',s.occurrence_id,
      'world',s.world_id,'channel',g.channel_id,'epoch',s.reset_epoch,'scheduled_at',s.scheduled_at,
      'generation',NEW.ownership_generation,'item',g.item_id,'evidence',g.evidence)
    AND root.state_revision_before=(g.evidence->'item'->>'state_revision')::numeric
    AND root.source_stack_ordinal=(g.evidence->'ground'->>'stack_ordinal')::bigint
    AND root.spatial_position=(g.evidence->'ground'->>'spatial_position')::bytea
    AND root.map_revision=g.evidence->'ground'->>'map_revision'
    AND root.content_revision=g.evidence->'ground'->>'content_revision'
    AND NEW.operation_count=1+jsonb_array_length(g.evidence->'tree')
    AND NOT EXISTS(SELECT 1 FROM jsonb_to_recordset(g.evidence->'tree') AS t(
       item_id uuid,item_revision numeric,parent_id uuid, ordinal numeric,corpse boolean)
     WHERE NOT EXISTS(SELECT 1 FROM game_spell_item_lines child
      WHERE child.transaction_id=NEW.transaction_id AND child.operation_kind=6 AND child.item_instance_id=t.item_id
       AND child.source_corpse_root=g.item_id AND child.state_revision_before=t.item_revision
       AND child.source_parent_item_instance_id=t.parent_id AND child.source_placement_ordinal=t.ordinal
       AND child.source_entry_is_corpse=t.corpse))
 ) THEN RAISE EXCEPTION 'server-save retirement requires its exact settled cohort and audit evidence' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_server_save_retirement_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_server_save_retirement_proven();
REVOKE ALL ON FUNCTION game_server_save_retirement_proven() FROM PUBLIC;

-- All SQL reads here run under the caller's scoped Item and current-pin locks.
-- Match the frozen placement and exact child edges again before opening a receipt.
CREATE FUNCTION game_server_save_retire_tree(p_occurrence uuid,p_channel uuid,p_item uuid,p_generation numeric)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE
 s game_server_save_occurrences%ROWTYPE; evidence jsonb; children jsonb;
 txid uuid:=uuidv7(); eventid uuid:=uuidv7();
 observed bigint; intent bytea;
BEGIN
 SELECT * INTO STRICT s FROM game_server_save_occurrences
 WHERE occurrence_id=p_occurrence AND cleanup_enabled;
 PERFORM pg_advisory_xact_lock(hashtextextended(replace(s.world_id::text,'-','')||replace(p_channel::text,'-',''),33));
 SELECT g.evidence INTO STRICT evidence FROM game_server_save_ground g
 JOIN game_server_save_channels c USING (occurrence_id,channel_id)
 WHERE g.occurrence_id=p_occurrence AND g.channel_id=p_channel AND g.item_id=p_item
   AND g.disposition='pending' AND c.state='frozen' FOR UPDATE OF g;
 IF NOT game_server_save_pin(s.world_id,p_channel,s.reset_epoch,s.map_revision,s.server_digest,s.frame_digest)
   OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
     JOIN game_durability_admission_runtime_guards guard USING(scope_key)
     WHERE a.world_id=s.world_id AND a.channel_id=p_channel AND a.scope_kind=1 AND a.state=1
       AND a.ownership_generation=p_generation AND guard.ready AND guard.ownership_generation=p_generation)
 THEN RAISE EXCEPTION 'server save pin or Channel owner is stale' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING (item_instance_id,world_id)
   WHERE g.item_instance_id=p_item AND g.world_id=s.world_id AND g.channel_id=p_channel
    AND g.scope_kind=1 AND g.instance_id IS NULL AND g.runtime_scope_ownership_generation<=p_generation
    AND g.content_revision='sha256:'||encode(s.server_digest,'hex') AND g.native_room_placement_context=s.frame_digest
    AND i.lifecycle=1 AND i.quantity>0 AND to_jsonb(i)=evidence->'item' AND to_jsonb(g)=evidence->'ground'
    AND NOT EXISTS(SELECT 1 FROM game_house_tiles h WHERE (h.world_id,h.spatial_position)=(g.world_id,g.spatial_position))
    AND NOT EXISTS(SELECT 1 FROM game_native_map_item_receipts m WHERE m.item_instance_id=p_item)
    AND NOT EXISTS(SELECT 1 FROM game_map_item_mint_receipts m WHERE m.item_instance_id=p_item))
 THEN RAISE EXCEPTION 'server save Ground placement changed' USING ERRCODE='23514'; END IF;

 SELECT coalesce(jsonb_agg(to_jsonb(child)), '[]'::jsonb) INTO children FROM (
  SELECT i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,
   i.quantity,i.state_revision,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,
   t.parent_id AS source_parent_item_instance_id,t.ordinal AS source_placement_ordinal,p_item AS source_corpse_root,
   t.corpse AS source_entry_is_corpse,t.depth
  FROM jsonb_to_recordset(evidence->'tree') AS t(
    item_id uuid,item_revision numeric,parent_id uuid,ordinal numeric,placed_transaction uuid,corpse boolean,depth integer)
  JOIN game_item_instances i ON i.item_instance_id=t.item_id AND i.world_id=s.world_id
     AND i.state_revision=t.item_revision AND i.lifecycle=1 AND i.quantity>0
  JOIN game_item_ground_locations g ON g.item_instance_id=p_item
  WHERE CASE WHEN t.corpse THEN EXISTS(SELECT 1 FROM game_item_corpse_container_entries e
     WHERE (e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
       =(t.item_id,s.world_id,t.parent_id,t.ordinal,t.placed_transaction))
    ELSE EXISTS(SELECT 1 FROM game_item_nested_entries e
     WHERE (e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
       =(t.item_id,s.world_id,t.parent_id,t.ordinal,t.placed_transaction)) END
  ORDER BY i.item_instance_id FOR UPDATE OF i
 ) child;
 IF jsonb_array_length(children)<>jsonb_array_length(evidence->'tree')
   OR jsonb_array_length(children)>=256
 THEN RAISE EXCEPTION 'server save requires the entire bounded Ground tree' USING ERRCODE='23514'; END IF;
 observed:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 intent:=convert_to(jsonb_build_object('schema','OTERYN_SERVER_SAVE_GROUND/v1','occurrence',s.occurrence_id,
      'world',s.world_id,'channel',p_channel,'epoch',s.reset_epoch,'scheduled_at',s.scheduled_at,
      'generation',p_generation,'item',p_item,'evidence',evidence)::text,'UTF8');
 INSERT INTO game_spell_item_receipts(transaction_id,event_id,world_id,channel_id,ownership_generation,
    spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,operation_count,
    occurred_at_unix_ms,cause_kind,server_save_occurrence_id)
 VALUES(txid,eventid,s.world_id,p_channel,p_generation,'WorldEvent','oteryn:world-event.server-save.ground-sweep',
    'definition-r1',s.server_digest,sha256(intent),intent,1+jsonb_array_length(children),observed,7,p_occurrence);
 PERFORM game_retire_ground_tree_batch(txid,ARRAY[p_item],children,false);
 UPDATE game_server_save_ground SET disposition='retired',transaction_id=txid
 WHERE occurrence_id=p_occurrence AND channel_id=p_channel AND item_id=p_item;
END $$;
REVOKE ALL ON FUNCTION game_server_save_retire_tree(uuid,uuid,uuid,numeric) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_server_save_retire_tree(uuid,uuid,uuid,numeric) TO oteryn_game_runtime;

DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY['game_retire_ground_tree_batch(uuid,uuid[],jsonb,boolean)'::regprocedure,
   'game_server_save_retirement_proven()'::regprocedure,
   'game_server_save_retire_tree(uuid,uuid,uuid,numeric)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 END LOOP;
END $$;
