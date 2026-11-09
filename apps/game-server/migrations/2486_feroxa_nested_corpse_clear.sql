-- M52: retain real corpse ancestry while retiring nested loot leaves first.
-- Existing operation6, source OLD-custody captures and500-line cap remain authoritative.
CREATE OR REPLACE FUNCTION game_spell_item_capture_custody() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l game_spell_item_lines%ROWTYPE; r game_spell_item_receipts%ROWTYPE;
BEGIN
 SELECT * INTO l FROM game_spell_item_lines WHERE transaction_id=NEW.last_transaction_id AND item_instance_id=NEW.item_instance_id AND operation_kind IN(5,6) AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN NEW; END IF;
 SELECT * INTO r FROM game_spell_item_receipts WHERE transaction_id=l.transaction_id AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND OR OLD.lifecycle<>1 OR OLD.world_id<>r.world_id OR OLD.state_revision<>l.state_revision_before
   OR OLD.quantity<>l.quantity_before OR NEW.quantity<>l.quantity_after OR (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)<>(l.definition_family,l.definition_production_key,l.definition_revision) THEN
  RAISE EXCEPTION 'spell consumption captures exact actual OLD item state only' USING ERRCODE='23514';
 END IF;
 IF l.operation_kind=5 THEN
  IF l.source_equipment_slot IS NOT NULL THEN
   IF NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s JOIN game_character_equipment_state es USING(character_id)
     WHERE s.item_instance_id=OLD.item_instance_id AND s.character_id=r.character_id AND s.world_id=r.world_id
       AND s.slot=l.source_equipment_slot AND es.revision=l.source_equipment_revision) THEN
    RAISE EXCEPTION 'spell consumption source equipment custody changed' USING ERRCODE='23514';
   END IF;
  ELSIF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal) AND NOT game_quiver_entry(decode(replace(r.character_id::text,'-',''),'hex'),l.source_parent_item_instance_id,OLD.item_instance_id,l.source_placement_ordinal,r.world_id) THEN
   RAISE EXCEPTION 'spell consumption source inventory custody changed' USING ERRCODE='23514';
  END IF;
 ELSE
  IF NOT ((l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))
     OR (NOT l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))
     OR (r.cause_kind=2 AND NOT l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_nested_entries e
       WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id
         AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))) THEN
   RAISE EXCEPTION 'contained spell retirement source custody changed' USING ERRCODE='23514';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
      WHERE g.item_instance_id=l.source_corpse_root AND i.lifecycle=1 AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
       AND (g.world_id,g.channel_id)=(r.world_id,r.channel_id)
       AND (g.runtime_scope_ownership_generation=r.ownership_generation
         OR (r.cause_kind=2 AND g.scope_kind=1 AND g.instance_id IS NULL
           AND g.runtime_scope_ownership_generation<r.ownership_generation))
       AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)=(l.spatial_position,l.map_revision,l.content_revision,l.placement_context)) THEN
   RAISE EXCEPTION 'contained retirement root lacks actual current corpse placement' USING ERRCODE='23514';
  END IF;
 END IF;
 INSERT INTO game_spell_item_source_custody(transaction_id,item_instance_id,quantity_before,state_revision_before,source_parent,source_ordinal,source_slot)
 VALUES(l.transaction_id,OLD.item_instance_id,OLD.quantity,OLD.state_revision,l.source_parent_item_instance_id,l.source_placement_ordinal,l.source_equipment_slot);
 RETURN NEW;
END $$;



CREATE OR REPLACE FUNCTION game_item_nested_entry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_lines l
        JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
        JOIN game_spell_item_source_custody c ON c.transaction_id=l.transaction_id AND c.item_instance_id=i.item_instance_id
        WHERE (l.operation_kind=5 OR (l.operation_kind=6 AND r.cause_kind=2 AND NOT l.source_entry_is_corpse)) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
          AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
          AND c.source_parent=OLD.parent_item_instance_id AND c.source_ordinal=OLD.placement_ordinal
          AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=l.transaction_id AND i.state_revision=l.state_revision_before+1
          AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF TG_OP = 'INSERT' AND game_item_loop_line_current(NEW.item_instance_id) THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'DELETE' AND game_item_loop_line_current(OLD.item_instance_id) THEN
        RETURN NULL;
    END IF;
    -- BAGS-USE-1: the source entry spent to zero, in this same physical transaction.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_use_records u
          JOIN game_item_instances i ON i.item_instance_id = u.source_item_instance_id
          CROSS JOIN LATERAL game_item_loop_owner(OLD.parent_item_instance_id) o
         WHERE u.source_item_instance_id = OLD.item_instance_id
           AND u.world_id = OLD.world_id
           AND u.source_quantity_after = 0
           AND u.source_parent_item_instance_id = OLD.parent_item_instance_id
           AND u.source_placement_ordinal = OLD.placement_ordinal
           AND o.owner_character = u.character_id AND o.depth BETWEEN 1 AND 7
           AND i.last_transaction_id = u.transaction_id AND i.lifecycle = 2 AND i.quantity = 0
           AND u.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'nested container custody changes only by an item loop move' USING ERRCODE = '23514';
END;
$$;


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
  WITH RECURSIVE chain(root,item,parent,ordinal,is_corpse,depth,path) AS (
 SELECT e.parent_item_instance_id,e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,true,1,
   ARRAY[e.parent_item_instance_id,e.item_instance_id]
 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=ANY(ids)
 UNION ALL
 SELECT walk.root,e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,false,walk.depth+1,walk.path||e.item_instance_id
 FROM chain walk JOIN game_item_nested_entries e ON e.parent_item_instance_id=walk.item
 WHERE walk.depth<8 AND NOT e.item_instance_id=ANY(walk.path)
), costs AS (
    SELECT roots.item,1+count(e.item) AS work
    FROM unnest(ids) AS roots(item)
    LEFT JOIN chain e ON e.root=roots.item
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
  WITH RECURSIVE chain(root,item,parent,ordinal,is_corpse,depth,path) AS (
 SELECT e.parent_item_instance_id,e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,true,1,
   ARRAY[e.parent_item_instance_id,e.item_instance_id]
 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=ANY(ids)
 UNION ALL
 SELECT walk.root,e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,false,walk.depth+1,walk.path||e.item_instance_id
 FROM chain walk JOIN game_item_nested_entries e ON e.parent_item_instance_id=walk.item
 WHERE walk.depth<8 AND NOT e.item_instance_id=ANY(walk.path)
) SELECT array_agg(item_instance_id),jsonb_agg(to_jsonb(q)) INTO child_ids,children FROM (
 SELECT i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,
   i.quantity,i.state_revision,NULL::bigint AS stack_ordinal,g.spatial_position,g.corpse_ref,
   g.map_revision,g.content_revision,g.native_room_placement_context,
   walk.parent AS source_parent_item_instance_id,walk.ordinal AS source_placement_ordinal,
   walk.root AS source_corpse_root,walk.is_corpse AS source_entry_is_corpse,walk.depth
 FROM chain walk JOIN game_item_instances i ON i.item_instance_id=walk.item
 JOIN game_item_ground_locations g ON g.item_instance_id=walk.root
 ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i,g) q;
  IF cardinality(ids)+COALESCE(cardinality(child_ids),0)>500 THEN
    RAISE EXCEPTION 'Feroxa clear exceeds native500-line batch bound' USING ERRCODE='23514';
  END IF;
  IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=ANY(child_ids))
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=ANY(child_ids))
  THEN RAISE EXCEPTION 'Feroxa clear requires nested contained-item retirement' USING ERRCODE='23514'; END IF;
  IF EXISTS(SELECT 1 FROM game_item_nested_entries
    WHERE parent_item_instance_id=ANY(child_ids) AND NOT item_instance_id=ANY(child_ids)) THEN
   RAISE EXCEPTION 'Feroxa corpse tree exceeds native depth/batch bound' USING ERRCODE='23514';
  END IF;
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
    'source-9f5a72c-r1',p_content,sha256(intent),intent,jsonb_array_length(removals),observed,2,p_firing,p_phase,f.reset_epoch);
  -- Children must retire while the actual corpse Ground root is still live.
  -- Existing deferred operation6 guards also prove its same-transaction root line.
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
      r.quantity,0,r.state_revision,f.world_id,p_channel,r.spatial_position,r.map_revision,r.content_revision,
      r.native_room_placement_context,p_content,false,false,false,
      r.source_parent_item_instance_id,r.source_placement_ordinal,r.source_corpse_root,r.source_entry_is_corpse);
   UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=txid WHERE item_instance_id=r.item_instance_id;
   IF r.source_entry_is_corpse THEN
    DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=r.item_instance_id;
   ELSE
    DELETE FROM game_item_nested_entries WHERE item_instance_id=r.item_instance_id;
   END IF;
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

DO $$ DECLARE n text; BEGIN
 FOREACH n IN ARRAY ARRAY['game_spell_item_capture_custody','game_item_nested_entry_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
END $$;
