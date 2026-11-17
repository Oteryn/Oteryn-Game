-- Retire bounded sibling cohorts without a separate executor turn per Item.
-- Actual-edge capture, Item guards, receipt proofs and audit proofs still run
-- for every row. A depth finishes (including its receipts) before its parents.
-- Keep the existing input bounds, reservations, function identity and grants.
DO $migration$
DECLARE
 signature TEXT := 'game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint)';
 installed pg_proc%ROWTYPE;
 successor pg_proc%ROWTYPE;
 definition TEXT;
 body TEXT;
 old_declaration TEXT := 'DECLARE n INT:=cardinality(p_items); k INT;';
 old_loop TEXT := $old$ FOR k IN 1..n LOOP
  IF NOT game_player_death_retire_entry(p_items[k],p_generation,p_transactions[k],p_parents[k],
   p_entry_kinds[k],p_event_type_id,p_schema_revision,p_retention_profile_id,p_retention_ms) THEN
   RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
  END IF;
 END LOOP;$old$;
 new_loop TEXT := $new$
 -- The topology only schedules writes. The unchanged BEFORE DELETE capture
 -- independently proves each actual current edge and its live ancestor path.
 WITH RECURSIVE supplied AS (
  SELECT position,p_items[position] AS item,p_parents[position] AS parent
  FROM generate_series(1,n) AS batch(position)
 ), tree(item,depth,path) AS (
  SELECT p_root,0,ARRAY[p_root]
  UNION ALL
  SELECT s.item,t.depth+1,t.path||s.item
  FROM tree t JOIN supplied s ON s.parent=t.item
  WHERE t.depth<8 AND NOT s.item=ANY(t.path)
 )
 SELECT array_agg(t.depth ORDER BY s.position),max(t.depth)
 INTO depths,max_depth FROM supplied s JOIN tree t ON t.item=s.item;
 IF cardinality(depths) IS DISTINCT FROM n OR array_position(depths,NULL) IS NOT NULL
  OR max_depth NOT BETWEEN 1 AND 8 THEN
  RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
 END IF;

 -- Re-read the inserted reservations, retaining the single-entry writer's
 -- admission conditions before any custody mutation.
 SELECT count(*) INTO affected
 FROM generate_series(1,n) AS batch(position)
 JOIN game_item_decay_retire_reservations r ON r.item_instance_id=p_items[batch.position]
  AND r.fence_scope_ownership_generation=p_generation
  AND r.transaction_id=p_transactions[batch.position]
 WHERE r.cause_kind IN('WORLD_RESET','ACTIVITY_RETIREMENT')
  AND r.placement_ordinal IS NOT NULL AND r.ground_root_item_instance_id IS NOT NULL
  AND r.ground_root_item_instance_id<>r.item_instance_id
  AND r.type2_schema_revision IS NOT DISTINCT FROM p_schema_revision;
 IF affected<>n THEN
  RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
 END IF;

 FOR level IN REVERSE max_depth..1 LOOP
  SELECT count(*) INTO level_count FROM unnest(depths) AS d(depth) WHERE d.depth=level;
  removed:=0;
  DELETE FROM game_item_container_entries e
  USING generate_series(1,n) AS batch(position),game_item_decay_retire_reservations r
  WHERE depths[batch.position]=level AND p_entry_kinds[batch.position]=1
   AND r.item_instance_id=p_items[batch.position]
   AND r.fence_scope_ownership_generation=p_generation
   AND r.transaction_id=p_transactions[batch.position]
   AND e.item_instance_id=r.item_instance_id AND e.parent_item_instance_id=p_parents[batch.position]
   AND e.placement_ordinal=r.placement_ordinal AND e.world_id=r.world_id;
  GET DIAGNOSTICS affected=ROW_COUNT;
  removed:=removed+affected;
  DELETE FROM game_item_nested_entries e
  USING generate_series(1,n) AS batch(position),game_item_decay_retire_reservations r
  WHERE depths[batch.position]=level AND p_entry_kinds[batch.position]=2
   AND r.item_instance_id=p_items[batch.position]
   AND r.fence_scope_ownership_generation=p_generation
   AND r.transaction_id=p_transactions[batch.position]
   AND e.item_instance_id=r.item_instance_id AND e.parent_item_instance_id=p_parents[batch.position]
   AND e.placement_ordinal=r.placement_ordinal AND e.world_id=r.world_id;
  GET DIAGNOSTICS affected=ROW_COUNT;
  removed:=removed+affected;
  IF removed<>level_count THEN
   RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
  END IF;

  UPDATE game_item_instances i SET quantity=0,lifecycle=2,last_transaction_id=r.transaction_id
  FROM generate_series(1,n) AS batch(position),game_item_decay_retire_reservations r
  WHERE depths[batch.position]=level AND r.item_instance_id=p_items[batch.position]
   AND r.fence_scope_ownership_generation=p_generation
   AND r.transaction_id=p_transactions[batch.position]
   AND i.item_instance_id=r.item_instance_id AND i.lifecycle=1 AND i.quantity=r.quantity_before;
  GET DIAGNOSTICS affected=ROW_COUNT;
  IF affected<>level_count THEN
   RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
  END IF;

  INSERT INTO game_item_decay_retire_receipts
   (item_instance_id,corpse_item_instance_id,world_id,channel_id,
    fence_scope_ownership_generation,transaction_id,event_id,quantity_before,
    placement_ordinal,deadline,occurred_at,envelope_sha256,committed_at,cause_kind,
    reset_epoch,retirement_binding,target_digest,ground_root_item_instance_id,
    scope_kind,instance_id,activity_handoff_id,activity_ownership_generation)
  SELECT r.item_instance_id,NULL,r.world_id,r.channel_id,r.fence_scope_ownership_generation,
   r.transaction_id,r.event_id,r.quantity_before,r.placement_ordinal,NULL,r.occurred_at,
   sha256(r.envelope),floor(extract(epoch FROM statement_timestamp())*1000)::bigint,
   r.cause_kind,r.reset_epoch,r.retirement_binding,r.target_digest,r.ground_root_item_instance_id,
   r.scope_kind,r.instance_id,r.activity_handoff_id,r.activity_ownership_generation
  FROM generate_series(1,n) AS batch(position)
  JOIN game_item_decay_retire_reservations r ON r.item_instance_id=p_items[batch.position]
   AND r.fence_scope_ownership_generation=p_generation
   AND r.transaction_id=p_transactions[batch.position]
  WHERE depths[batch.position]=level ORDER BY batch.position;
  GET DIAGNOSTICS affected=ROW_COUNT;
  IF affected<>level_count THEN
   RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
  END IF;

  INSERT INTO game_item_audit_outbox
   (event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,
    schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,
    envelope,envelope_sha256,publication_state)
  SELECT r.event_id,r.transaction_id,1,1,p_event_type_id,p_schema_revision,p_retention_profile_id,
   r.item_instance_id,r.occurred_at,r.occurred_at+p_retention_ms,r.envelope,d.envelope_sha256,1
  FROM generate_series(1,n) AS batch(position)
  JOIN game_item_decay_retire_reservations r ON r.item_instance_id=p_items[batch.position]
   AND r.fence_scope_ownership_generation=p_generation
   AND r.transaction_id=p_transactions[batch.position]
  JOIN game_item_decay_retire_receipts d ON d.item_instance_id=r.item_instance_id
   AND d.fence_scope_ownership_generation=r.fence_scope_ownership_generation
   AND d.transaction_id=r.transaction_id AND d.created_xact_id=pg_current_xact_id()
  WHERE depths[batch.position]=level ORDER BY batch.position;
  GET DIAGNOSTICS affected=ROW_COUNT;
  IF affected<>level_count THEN
   RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
  END IF;
 END LOOP;$new$;
BEGIN
 SELECT * INTO STRICT installed FROM pg_proc WHERE oid=signature::regprocedure;
 definition:=pg_get_functiondef(installed.oid);
 body:=installed.prosrc;
 IF (length(body)-length(replace(body,old_declaration,'')))/length(old_declaration)<>1
  OR (length(body)-length(replace(body,old_loop,'')))/length(old_loop)<>1
  OR (length(definition)-length(replace(definition,body,'')))/length(body)<>1 THEN
  RAISE EXCEPTION 'unexpected bounded death retirement writer';
 END IF;
 body:=replace(body,old_declaration,
  'DECLARE n INT:=cardinality(p_items); depths INT[]; max_depth INT; level INT; level_count BIGINT; affected BIGINT; removed BIGINT;');
 body:=replace(body,old_loop,new_loop);
 EXECUTE replace(definition,installed.prosrc,body);
 SELECT * INTO STRICT successor FROM pg_proc WHERE oid=installed.oid;
 IF successor.prosrc IS DISTINCT FROM body OR successor.proowner<>installed.proowner
  OR successor.prosecdef IS DISTINCT FROM installed.prosecdef
  OR successor.proconfig IS DISTINCT FROM installed.proconfig
  OR successor.proacl IS DISTINCT FROM installed.proacl THEN
  RAISE EXCEPTION 'bounded death retirement writer metadata changed';
 END IF;
END $migration$;
