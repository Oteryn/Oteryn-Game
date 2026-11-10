-- M52: crowded corpse groups must commit bounded progress instead of
-- rejecting the entire phase forever. Existing fences, custody guards, audit
-- schema and Item500-line limit are unchanged; only root batch selection changes.
CREATE OR REPLACE FUNCTION game_feroxa_clear_ground_batch(
 p_firing uuid,p_world uuid,p_channel uuid,p_generation numeric,p_phase smallint,p_now bigint,
 p_map_revision text,p_content_revision text,p_frame bytea,p_content bytea)
RETURNS boolean LANGUAGE plpgsql STRICT AS $$
DECLARE f record; r record; c record; ids uuid[]; txid uuid; eventid uuid;
 intent bytea; observed bigint; ordinal integer:=0; removals jsonb; finished boolean;
 children jsonb; child_ids uuid[];
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
  -- Reserve one ledger line per root and per actual direct corpse child.
  -- The scope/parent locks above retain the same physical read set through COMMIT.
  -- Select a deterministic prefix, so a crowded room progresses without raising
  -- the native500-line limit or retiring a root before all its children.
  WITH costs AS (
    SELECT roots.item,1+count(e.item_instance_id) AS work
    FROM unnest(ids) AS roots(item)
    LEFT JOIN game_item_corpse_container_entries e ON e.parent_item_instance_id=roots.item
    GROUP BY roots.item
  ), prefix AS (
    SELECT item,sum(work) OVER(ORDER BY item ROWS UNBOUNDED PRECEDING) AS work FROM costs
  ) SELECT array_agg(item ORDER BY item) INTO ids FROM prefix WHERE work<=500;
  IF ids IS NULL THEN
    RAISE EXCEPTION 'Feroxa root exceeds native500-line batch bound' USING ERRCODE='23514';
  END IF;
  SELECT jsonb_agg(evidence.body ORDER BY evidence.body->>'item_instance_id') INTO removals
    FROM jsonb_array_elements(removals) AS evidence(body)
    WHERE (evidence.body->>'item_instance_id')::uuid=ANY(ids);
 END IF;
 IF ids IS NOT NULL THEN
  -- Direct corpse loot uses the existing physical operation6 and OLD-custody
  -- capture. Other container shapes remain closed until their own path is proven.
  IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=ANY(ids))
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=ANY(ids))
  THEN RAISE EXCEPTION 'Feroxa clear requires non-corpse contained-item retirement' USING ERRCODE='23514'; END IF;
  SELECT array_agg(item_instance_id),jsonb_agg(to_jsonb(q)) INTO child_ids,children FROM (
    SELECT i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,
      i.quantity,i.state_revision,NULL::bigint AS stack_ordinal,g.spatial_position,g.corpse_ref,
      g.map_revision,g.content_revision,g.native_room_placement_context,
      e.parent_item_instance_id AS source_parent_item_instance_id,
      e.placement_ordinal AS source_placement_ordinal,
      e.parent_item_instance_id AS source_corpse_root,true AS source_entry_is_corpse
    FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id)
      JOIN game_item_ground_locations g ON g.item_instance_id=e.parent_item_instance_id
    WHERE e.parent_item_instance_id=ANY(ids)
    ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF e,i) q;
  IF cardinality(ids)+COALESCE(cardinality(child_ids),0)>500 THEN
    RAISE EXCEPTION 'Feroxa clear exceeds native500-line batch bound' USING ERRCODE='23514';
  END IF;
  IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=ANY(child_ids))
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=ANY(child_ids))
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=ANY(child_ids))
  THEN RAISE EXCEPTION 'Feroxa clear requires nested contained-item retirement' USING ERRCODE='23514'; END IF;
  removals:=removals||COALESCE(children,'[]'::jsonb);
  txid:=uuidv7(); eventid:=uuidv7();
  observed:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
  intent:=convert_to(jsonb_build_object('schema','OTERYN_FEROXA_MAP_CLEAR/v2','firing',p_firing,
    'world',f.world_id,'channel',p_channel,'generation',p_generation::text,'epoch',f.reset_epoch::text,
    'phase',p_phase::text,'removals',removals)::text,'UTF8');
  INSERT INTO game_spell_item_receipts(transaction_id,event_id,world_id,channel_id,ownership_generation,
    spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,operation_count,
    occurred_at_unix_ms,cause_kind,feroxa_firing_id,feroxa_phase,feroxa_reset_epoch)
   VALUES(txid,eventid,f.world_id,p_channel,p_generation,'WorldEvent','oteryn:world-event.feroxa.map-clear',
    'source-9f5a72c-r1',p_content,sha256(intent),intent,jsonb_array_length(removals),observed,5,p_firing,p_phase,f.reset_epoch);
  -- Children must retire while the actual corpse Ground root is still live.
  -- Existing deferred operation6 guards also prove its same-transaction root line.
  FOR r IN SELECT i.*,e.parent_item_instance_id,e.placement_ordinal,
      g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context
    FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id)
      JOIN game_item_ground_locations g ON g.item_instance_id=e.parent_item_instance_id
    WHERE i.item_instance_id=ANY(child_ids) ORDER BY i.item_instance_id LOOP
   ordinal:=ordinal+1;
   INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,
    definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,
    world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,
    content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid,
    source_parent_item_instance_id,source_placement_ordinal,source_corpse_root,source_entry_is_corpse)
    VALUES(txid,ordinal,6,r.item_instance_id,r.definition_family,r.definition_production_key,r.definition_revision_ref,
      r.quantity,0,r.state_revision,f.world_id,p_channel,r.spatial_position,r.map_revision,r.content_revision,
      r.native_room_placement_context,p_content,false,false,false,
      r.parent_item_instance_id,r.placement_ordinal,r.parent_item_instance_id,true);
   UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=txid WHERE item_instance_id=r.item_instance_id;
   DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=r.item_instance_id;
  END LOOP;
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
