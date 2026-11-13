-- M52: a genuine World-map phase cause on the existing physical Item ledger.
-- No Player command, spell cost, Creature placement or Ability is fabricated.
-- Source classes 2..4 already belong to Encounter/Creature/Global callbacks.
-- Feroxa has its own discriminator and retains every preceding source proof.
ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_receipts_cause_kind_check;
ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_receipts_cause_kind_check
 CHECK(cause_kind IN(0,1,2,3,4,5));
ALTER TABLE game_spell_item_receipts
 ADD COLUMN feroxa_firing_id uuid REFERENCES game_world_raid_firings(firing_id),
 ADD COLUMN feroxa_phase smallint,
 ADD COLUMN feroxa_reset_epoch numeric(20,0);
DO $$ DECLARE previous text; name text; arm text; BEGIN
 arm:=$predicate$(cause_kind=5 AND spell_family='WorldEvent'
  AND spell_production_key='oteryn:world-event.feroxa.map-clear' AND spell_revision='source-9f5a72c-r1'
  AND runtime_scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL AND swing_ordinal IS NULL
  AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
  AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
  AND num_nonnulls(creature_placement,creature_actor_generation,creature_definition_key,creature_definition_revision,
      creature_occurrence,creature_map_digest,creature_frame_digest,creature_source_body_digest,creature_cast_digest)=0
  AND feroxa_firing_id IS NOT NULL AND feroxa_phase IN(1,2)
  AND feroxa_reset_epoch BETWEEN 1 AND 18446744073709551615)$predicate$;
 FOREACH name IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=name;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK (((cause_kind<>5 AND (%s) AND num_nonnulls(feroxa_firing_id,feroxa_phase,feroxa_reset_epoch)=0) OR (%s)) IS TRUE)',name,previous,arm);
 END LOOP;
END $$;
CREATE TABLE game_world_feroxa_ground_clears (
 firing_id uuid NOT NULL,
 channel_id uuid NOT NULL,
 phase smallint NOT NULL CHECK(phase IN(1,2)),
 complete boolean NOT NULL DEFAULT false,
 last_transaction_id uuid REFERENCES game_spell_item_receipts(transaction_id),
 PRIMARY KEY(firing_id,channel_id,phase),
 FOREIGN KEY(firing_id,channel_id) REFERENCES game_world_raid_targets(firing_id,channel_id)
);
REVOKE ALL ON game_world_feroxa_ground_clears FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_world_feroxa_ground_clears TO oteryn_game_runtime;
GRANT SELECT ON game_world_feroxa_ground_clears TO oteryn_game_control;

-- Closed native Ground address codec: signed big-endian x:i32,y:i32,floor:i16.
-- The Lua clear rectangle is smaller than the loaded35x35 map. Never widen it.
CREATE FUNCTION game_feroxa_clear_position(p bytea) RETURNS boolean LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT CASE WHEN octet_length(p)=10 THEN
  get_byte(p,0)=0 AND get_byte(p,1)=0 AND get_byte(p,4)=0 AND get_byte(p,5)=0
  AND get_byte(p,2)*256+get_byte(p,3) BETWEEN 33415 AND 33445
  AND get_byte(p,6)*256+get_byte(p,7) BETWEEN 31522 AND 31554
  AND get_byte(p,8)=0 AND get_byte(p,9)=11 ELSE false END
$$;

CREATE FUNCTION game_feroxa_ground_cause_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p jsonb; BEGIN
 IF NEW.cause_kind<>5 THEN RETURN NULL; END IF;
 p:=convert_from(NEW.intent,'UTF8')::jsonb;
 IF p->>'schema' IS DISTINCT FROM 'OTERYN_FEROXA_MAP_CLEAR/v1'
  OR p->>'firing' IS DISTINCT FROM NEW.feroxa_firing_id::text
  OR p->>'phase' IS DISTINCT FROM NEW.feroxa_phase::text
  OR p->>'world' IS DISTINCT FROM NEW.world_id::text
  OR p->>'channel' IS DISTINCT FROM NEW.channel_id::text
  OR p->>'generation' IS DISTINCT FROM NEW.ownership_generation::text
  OR p->>'epoch' IS DISTINCT FROM NEW.feroxa_reset_epoch::text
  OR NEW.operation_count NOT BETWEEN 1 AND 64
  OR jsonb_typeof(p->'removals') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p->'removals') IS DISTINCT FROM NEW.operation_count
  OR NOT EXISTS(SELECT 1 FROM game_world_feroxa_ground_clears c WHERE c.firing_id=NEW.feroxa_firing_id
    AND c.channel_id=NEW.channel_id AND c.phase=NEW.feroxa_phase AND c.last_transaction_id=NEW.transaction_id)
  OR NOT EXISTS(SELECT 1 FROM game_world_raid_firings f
    JOIN game_world_raid_targets t USING(firing_id)
    JOIN game_runtime_scope_assignments a ON a.world_id=f.world_id AND a.channel_id=t.channel_id AND a.scope_kind=1 AND a.state=1
    JOIN game_durability_admission_runtime_guards g USING(scope_key)
    JOIN game_world_reset_active active ON active.world_id=f.world_id AND active.reset_epoch=f.reset_epoch
    WHERE f.firing_id=NEW.feroxa_firing_id AND f.world_id=NEW.world_id AND f.reset_epoch=NEW.feroxa_reset_epoch
      AND f.raid_key='raid:edron.feroxa' AND f.state IN(1,2)
      AND t.channel_id=NEW.channel_id AND t.owner_generation=NEW.ownership_generation
      AND a.ownership_generation=NEW.ownership_generation AND g.ready AND g.ownership_generation=NEW.ownership_generation)
  OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
    AND (l.operation_kind<>2 OR NOT game_feroxa_clear_position(l.spatial_position)
      OR l.destination_parent_item_instance_id IS NOT NULL OR l.quantity_after<>0
      OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p->'removals') AS evidence(body)
        WHERE evidence.body->>'item_instance_id'=l.item_instance_id::text
          AND evidence.body->>'definition_family'=l.definition_family
          AND evidence.body->>'definition_production_key'=l.definition_production_key
          AND evidence.body->>'definition_revision_ref'=l.definition_revision
          AND evidence.body->>'quantity'=l.quantity_before::text AND evidence.body->>'state_revision'=l.state_revision_before::text
          AND evidence.body->>'stack_ordinal'=l.source_stack_ordinal::text
          AND evidence.body->'spatial_position'=to_jsonb(l.spatial_position)
          AND evidence.body->>'map_revision'=l.map_revision AND evidence.body->>'content_revision'=l.content_revision
          AND evidence.body->'native_room_placement_context'=to_jsonb(l.placement_context))))
 THEN RAISE EXCEPTION 'Feroxa clear must prove its exact World phase and Ground-only audit' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_feroxa_ground_cause_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_feroxa_ground_cause_proven();

-- Caller retains the physical Scope lock and independently proved node/World
-- authority. The SQL owner also checks current reset, assignment, target and run.
-- A complete phase is a permanent replay barrier: later dropped items survive.
CREATE FUNCTION game_feroxa_clear_ground_batch(
 p_firing uuid,p_world uuid,p_channel uuid,p_generation numeric,p_phase smallint,p_now bigint,
 p_map_revision text,p_content_revision text,p_frame bytea,p_content bytea)
RETURNS boolean LANGUAGE plpgsql STRICT AS $$
DECLARE f record; r record; c record; ids uuid[]; txid uuid; eventid uuid;
 intent bytea; observed bigint; ordinal integer:=0; removals jsonb; finished boolean;
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended(replace(p_world::text,'-','')||replace(p_channel::text,'-',''),33));
 SELECT * INTO f FROM game_world_raid_firings WHERE firing_id=p_firing FOR UPDATE;
 IF NOT FOUND OR f.world_id<>p_world OR f.raid_key<>'raid:edron.feroxa' OR f.state NOT IN(1,2)
  OR p_now<f.fired_at_ms OR p_phase IS DISTINCT FROM
    (CASE WHEN p_now-f.fired_at_ms>=3600000 THEN 2 WHEN p_now-f.fired_at_ms>=1800000 THEN 1 ELSE 0 END)
  OR NOT game_world_reset_map_epoch_current(f.world_id,f.reset_epoch)
  OR NOT EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
      WHERE a.world_id=f.world_id AND a.reset_epoch=f.reset_epoch AND e.target_digest=f.bundle_digest)
  OR NOT EXISTS(SELECT 1 FROM game_world_raid_targets t JOIN game_runtime_scope_assignments a
      ON a.world_id=f.world_id AND a.channel_id=t.channel_id AND a.scope_kind=1 AND a.state=1
      JOIN game_durability_admission_runtime_guards g USING(scope_key)
      WHERE t.firing_id=p_firing AND t.channel_id=p_channel AND t.owner_generation=p_generation
        AND a.ownership_generation=p_generation AND g.ready AND g.ownership_generation=p_generation)
  OR EXISTS(SELECT 1 FROM game_world_raid_channel_runs q WHERE q.firing_id=p_firing AND q.channel_id=p_channel
      AND (q.owner_generation<>p_generation OR q.state NOT IN(1,2)))
  OR octet_length(p_frame)<>32 OR octet_length(p_content)<>32
  OR p_content_revision IS DISTINCT FROM 'sha256:'||encode(p_content,'hex')
 THEN RAISE EXCEPTION 'Feroxa clear requires current due World/Channel phase' USING ERRCODE='23514'; END IF;
 INSERT INTO game_world_feroxa_ground_clears(firing_id,channel_id,phase) VALUES(p_firing,p_channel,p_phase) ON CONFLICT DO NOTHING;
 SELECT * INTO c FROM game_world_feroxa_ground_clears WHERE firing_id=p_firing AND channel_id=p_channel AND phase=p_phase FOR UPDATE;
 IF c.complete THEN RETURN true; END IF;
 IF EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
   WHERE g.world_id=f.world_id AND g.channel_id=p_channel AND g.scope_kind=1 AND g.instance_id IS NULL
    AND i.lifecycle=1 AND game_feroxa_clear_position(g.spatial_position)
    AND (g.map_revision<>p_map_revision OR g.content_revision<>p_content_revision
      OR g.native_room_placement_context<>p_frame OR g.runtime_scope_ownership_generation>p_generation))
 THEN RAISE EXCEPTION 'Feroxa clear refuses foreign Ground frame' USING ERRCODE='23514'; END IF;
 SELECT array_agg(item_instance_id),jsonb_agg(to_jsonb(q)) INTO ids,removals FROM (
  SELECT i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,
    i.quantity,i.state_revision,g.stack_ordinal,g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,
    g.native_room_placement_context
  FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
  WHERE g.world_id=f.world_id AND g.channel_id=p_channel AND g.scope_kind=1 AND g.instance_id IS NULL
    AND i.lifecycle=1 AND game_feroxa_clear_position(g.spatial_position)
  ORDER BY i.item_instance_id LIMIT 64 FOR UPDATE OF i,g) q;
 IF ids IS NOT NULL THEN
  -- Nested custody is not discarded without its own audited retirement lines.
  IF EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=ANY(ids))
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=ANY(ids))
  THEN RAISE EXCEPTION 'Feroxa clear requires contained-item retirement' USING ERRCODE='23514'; END IF;
  txid:=uuidv7(); eventid:=uuidv7();
  observed:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
  intent:=convert_to(jsonb_build_object('schema','OTERYN_FEROXA_MAP_CLEAR/v1','firing',p_firing,
    'world',f.world_id,'channel',p_channel,'generation',p_generation::text,'epoch',f.reset_epoch::text,
    'phase',p_phase::text,'removals',removals)::text,'UTF8');
  INSERT INTO game_spell_item_receipts(transaction_id,event_id,world_id,channel_id,ownership_generation,
    spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,operation_count,
    occurred_at_unix_ms,cause_kind,feroxa_firing_id,feroxa_phase,feroxa_reset_epoch)
   VALUES(txid,eventid,f.world_id,p_channel,p_generation,'WorldEvent','oteryn:world-event.feroxa.map-clear',
    'source-9f5a72c-r1',p_content,sha256(intent),intent,cardinality(ids),observed,5,p_firing,p_phase,f.reset_epoch);
  FOR r IN SELECT i.*,g.stack_ordinal,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context
    FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=ANY(ids) ORDER BY i.item_instance_id LOOP
   ordinal:=ordinal+1;
   INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,
    definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,
    world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,source_stack_ordinal,
    content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid)
    VALUES(txid,ordinal,2,r.item_instance_id,r.definition_family,r.definition_production_key,r.definition_revision_ref,
      r.quantity,0,r.state_revision,f.world_id,p_channel,r.spatial_position,r.map_revision,r.content_revision,
      r.native_room_placement_context,r.stack_ordinal,p_content,false,false,false);
   UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=txid WHERE item_instance_id=r.item_instance_id;
   DELETE FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id;
  END LOOP;
  INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256)
    VALUES(eventid,txid,observed,observed+7776000000,intent,sha256(intent));
 END IF;
 SELECT NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
   WHERE g.world_id=f.world_id AND g.channel_id=p_channel AND g.scope_kind=1 AND g.instance_id IS NULL
    AND i.lifecycle=1 AND game_feroxa_clear_position(g.spatial_position)) INTO finished;
 UPDATE game_world_feroxa_ground_clears SET complete=finished,last_transaction_id=COALESCE(txid,last_transaction_id)
  WHERE firing_id=p_firing AND channel_id=p_channel AND phase=p_phase;
 RETURN finished;
END $$;
REVOKE ALL ON FUNCTION game_feroxa_clear_ground_batch(uuid,uuid,uuid,numeric,smallint,bigint,text,text,bytea,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_feroxa_clear_ground_batch(uuid,uuid,uuid,numeric,smallint,bigint,text,text,bytea,bytea) TO oteryn_game_runtime;
