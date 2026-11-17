-- Retire the bounded tree retained by a death Ground transfer, deepest first,
-- in the existing root transaction. Each item keeps its own frozen retirement
-- reservation, terminal receipt and immutable audit. Ordinary decay is unchanged.
-- Historical main entries and normalized nested edges retain their actual parent
-- and ordinal; no retirement flattens the graph or invents custody.
-- Equipped quivers use the exact game_item_loop_owner key/slot contract and
-- retain its at-most-20 ammunition leaves; they are not general nested roots.
CREATE TABLE game_player_death_retirement_edges (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 ground_root_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 death_transaction_id UUID NOT NULL REFERENCES game_player_death_item_records(transaction_id),
 world_id UUID NOT NULL,
 scope_kind SMALLINT NOT NULL,
 channel_id UUID,
 instance_id UUID,
 placement_ordinal NUMERIC(20,0) NOT NULL CHECK(placement_ordinal BETWEEN 1 AND 18446744073709551615),
 entry_kind SMALLINT NOT NULL CHECK(entry_kind IN(1,2)),
 depth SMALLINT NOT NULL CHECK(depth BETWEEN 1 AND 8),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(item_instance_id<>parent_item_instance_id AND item_instance_id<>ground_root_item_instance_id),
 CHECK((parent_item_instance_id=ground_root_item_instance_id)=(depth=1)),
 CHECK((scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
    OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL)),
 UNIQUE(ground_root_item_instance_id,parent_item_instance_id,placement_ordinal)
);
CREATE INDEX game_player_death_retirement_edges_parent
 ON game_player_death_retirement_edges(parent_item_instance_id);
CREATE INDEX game_player_death_retirement_edges_death
 ON game_player_death_retirement_edges(death_transaction_id);
REVOKE ALL ON game_player_death_retirement_edges FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_player_death_retirement_edges TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_player_death_retirement_edges_stamp BEFORE INSERT ON game_player_death_retirement_edges
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_player_death_retirement_edges_immutable BEFORE UPDATE OR DELETE ON game_player_death_retirement_edges
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_death_retirement_edges_no_truncate BEFORE TRUNCATE ON game_player_death_retirement_edges
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- BAGS placement ordinals are monotonic u64 identities, not container counts.
-- Preserve the 30-entry corpse-decay ceiling established by migration 0242.
-- The separate 16-item generated loot-plan bound does not bound corpse custody.
DO $migration$
DECLARE t TEXT; c RECORD; n INT;
BEGIN
 FOREACH t IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
  n:=0;
  FOR c IN SELECT conname FROM pg_constraint
   WHERE conrelid=to_regclass(t) AND contype='c'
    AND conkey=ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid=to_regclass(t) AND attname='placement_ordinal')]::smallint[]
  LOOP
   EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',t,c.conname);
   n:=n+1;
  END LOOP;
  IF n<>1 THEN RAISE EXCEPTION 'unexpected retirement ordinal constraint on %',t; END IF;
  EXECUTE format('ALTER TABLE %I ADD CONSTRAINT game_retire_placement_ordinal CHECK (
   placement_ordinal BETWEEN 1 AND 18446744073709551615
   AND (cause_kind<>''CORPSE_DECAY'' OR placement_ordinal<=30))',t);
 END LOOP;
END $migration$;

-- The native death-tree owner still locks/revalidates the graph, inserts each
-- exact reservation and visits children before their parents. Keep its four
-- writes in that order within one call per child; every installed trigger and
-- deferred live-authority proof still runs. The reservation supplies immutable
-- payload only, never current authority or a replacement for those proofs.
CREATE FUNCTION game_player_death_retire_entry(
 p_item UUID,p_generation NUMERIC,p_transaction_id UUID,p_parent UUID,p_entry_kind SMALLINT,
 p_event_type_id BIGINT,p_schema_revision BIGINT,p_retention_profile_id TEXT,p_retention_ms BIGINT
) RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SECURITY INVOKER AS $retire$
DECLARE v game_item_decay_retire_reservations%ROWTYPE; affected BIGINT; envelope_hash BYTEA;
BEGIN
 SELECT * INTO v FROM game_item_decay_retire_reservations
  WHERE item_instance_id=p_item AND fence_scope_ownership_generation=p_generation
   AND transaction_id=p_transaction_id;
 IF NOT FOUND OR v.cause_kind NOT IN('WORLD_RESET','ACTIVITY_RETIREMENT')
  OR v.placement_ordinal IS NULL OR v.ground_root_item_instance_id IS NULL
  OR v.ground_root_item_instance_id=p_item
  OR v.type2_schema_revision IS DISTINCT FROM p_schema_revision THEN
  RETURN FALSE;
 END IF;
 IF p_entry_kind=1 THEN
  DELETE FROM game_item_container_entries
   WHERE item_instance_id=p_item AND parent_item_instance_id=p_parent
    AND placement_ordinal=v.placement_ordinal;
  GET DIAGNOSTICS affected=ROW_COUNT;
 ELSIF p_entry_kind=2 THEN
  DELETE FROM game_item_nested_entries
   WHERE item_instance_id=p_item AND parent_item_instance_id=p_parent
    AND placement_ordinal=v.placement_ordinal;
  GET DIAGNOSTICS affected=ROW_COUNT;
 ELSE
  RETURN FALSE;
 END IF;
 IF affected<>1 THEN RETURN FALSE; END IF;
 UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=v.transaction_id
  WHERE item_instance_id=p_item AND lifecycle=1 AND quantity=v.quantity_before;
 GET DIAGNOSTICS affected=ROW_COUNT;
 IF affected<>1 THEN RETURN FALSE; END IF;
 envelope_hash:=sha256(v.envelope);
 INSERT INTO game_item_decay_retire_receipts
  (item_instance_id,corpse_item_instance_id,world_id,channel_id,
   fence_scope_ownership_generation,transaction_id,event_id,quantity_before,
   placement_ordinal,deadline,occurred_at,envelope_sha256,committed_at,cause_kind,
   reset_epoch,retirement_binding,target_digest,ground_root_item_instance_id,
   scope_kind,instance_id,activity_handoff_id,activity_ownership_generation)
 VALUES(p_item,NULL,v.world_id,v.channel_id,v.fence_scope_ownership_generation,
  v.transaction_id,v.event_id,v.quantity_before,v.placement_ordinal,NULL,v.occurred_at,
  envelope_hash,floor(extract(epoch FROM statement_timestamp())*1000)::bigint,
  v.cause_kind,v.reset_epoch,v.retirement_binding,v.target_digest,v.ground_root_item_instance_id,
  v.scope_kind,v.instance_id,v.activity_handoff_id,v.activity_ownership_generation);
 INSERT INTO game_item_audit_outbox
  (event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,
   schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,
   envelope,envelope_sha256,publication_state)
 VALUES(v.event_id,v.transaction_id,1,1,p_event_type_id,p_schema_revision,p_retention_profile_id,
  p_item,v.occurred_at,v.occurred_at+p_retention_ms,v.envelope,envelope_hash,1);
 RETURN TRUE;
END $retire$;
DO $migration$
BEGIN
 EXECUTE format('ALTER FUNCTION game_player_death_retire_entry(uuid,numeric,uuid,uuid,smallint,bigint,bigint,text,bigint) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_player_death_retire_entry(uuid,numeric,uuid,uuid,smallint,bigint,bigint,text,bigint)
 FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_player_death_retire_entry(uuid,numeric,uuid,uuid,smallint,bigint,bigint,text,bigint)
 TO oteryn_game_runtime;

-- One bounded transport call preserves the native owner's exact postorder.
-- Each child still gets its original reservation followed by the existing
-- ordered writer; no trigger, current-authority proof or terminal root check
-- is skipped. A child refusal aborts the batch before its caller can commit.
CREATE FUNCTION game_player_death_retire_entries(
 p_items UUID[],p_parents UUID[],p_transactions UUID[],p_events UUID[],
 p_quantities BIGINT[],p_ordinals TEXT[],p_envelopes BYTEA[],p_entry_kinds SMALLINT[],
 p_root UUID,p_generation NUMERIC,p_world UUID,p_channel UUID,p_scope_kind SMALLINT,p_instance UUID,
 p_occurred_at BIGINT,p_node UUID,p_registration_revision NUMERIC,p_schema_revision SMALLINT,
 p_cause_kind TEXT,p_reset_epoch NUMERIC,p_retirement_binding BYTEA,p_target_digest BYTEA,
 p_activity_handoff UUID,p_activity_generation NUMERIC,
 p_event_type_id BIGINT,p_retention_profile_id TEXT,p_retention_ms BIGINT
) RETURNS BIGINT
LANGUAGE plpgsql VOLATILE SECURITY INVOKER AS $batch$
DECLARE n INT:=cardinality(p_items); k INT;
BEGIN
 IF n IS NULL OR n>500
  OR cardinality(p_parents) IS DISTINCT FROM n
  OR cardinality(p_transactions) IS DISTINCT FROM n
  OR cardinality(p_events) IS DISTINCT FROM n
  OR cardinality(p_quantities) IS DISTINCT FROM n
  OR cardinality(p_ordinals) IS DISTINCT FROM n
  OR cardinality(p_envelopes) IS DISTINCT FROM n
  OR cardinality(p_entry_kinds) IS DISTINCT FROM n
  OR p_cause_kind IS NULL OR p_cause_kind NOT IN('WORLD_RESET','ACTIVITY_RETIREMENT') THEN
  RAISE EXCEPTION 'invalid death retirement batch' USING ERRCODE='23514';
 END IF;
 IF n=0 THEN RETURN 0; END IF;
 IF array_ndims(p_items) IS DISTINCT FROM 1 OR array_lower(p_items,1) IS DISTINCT FROM 1
  OR array_dims(p_parents) IS DISTINCT FROM array_dims(p_items)
  OR array_dims(p_transactions) IS DISTINCT FROM array_dims(p_items)
  OR array_dims(p_events) IS DISTINCT FROM array_dims(p_items)
  OR array_dims(p_quantities) IS DISTINCT FROM array_dims(p_items)
  OR array_dims(p_ordinals) IS DISTINCT FROM array_dims(p_items)
  OR array_dims(p_envelopes) IS DISTINCT FROM array_dims(p_items)
  OR array_dims(p_entry_kinds) IS DISTINCT FROM array_dims(p_items) THEN
  RAISE EXCEPTION 'invalid death retirement batch dimensions' USING ERRCODE='23514';
 END IF;
 IF array_position(p_items,NULL) IS NOT NULL OR array_position(p_parents,NULL) IS NOT NULL
  OR array_position(p_transactions,NULL) IS NOT NULL OR array_position(p_events,NULL) IS NOT NULL
  OR array_position(p_quantities,NULL) IS NOT NULL OR array_position(p_ordinals,NULL) IS NOT NULL
  OR array_position(p_envelopes,NULL) IS NOT NULL OR array_position(p_entry_kinds,NULL) IS NOT NULL THEN
  RAISE EXCEPTION 'null death retirement batch entry' USING ERRCODE='23514';
 END IF;
 FOR k IN 1..n LOOP
  INSERT INTO game_item_decay_retire_reservations
   (item_instance_id,fence_scope_ownership_generation,corpse_item_instance_id,world_id,channel_id,
    transaction_id,event_id,quantity_before,placement_ordinal,deadline,occurred_at,envelope,
    fence_holder_node_id,fence_holder_registration_revision,work_units_used,reserved_at,
    type2_schema_revision,cause_kind,reset_epoch,retirement_binding,target_digest,
    ground_root_item_instance_id,scope_kind,instance_id,activity_handoff_id,activity_ownership_generation)
  VALUES(p_items[k],p_generation,NULL,p_world,p_channel,p_transactions[k],p_events[k],p_quantities[k],
   p_ordinals[k]::numeric(20,0),NULL,p_occurred_at,p_envelopes[k],p_node,p_registration_revision,0,
   floor(extract(epoch FROM statement_timestamp())*1000)::bigint,p_schema_revision,p_cause_kind,
   p_reset_epoch,p_retirement_binding,p_target_digest,p_root,p_scope_kind,p_instance,
   p_activity_handoff,p_activity_generation);
  IF NOT game_player_death_retire_entry(p_items[k],p_generation,p_transactions[k],p_parents[k],
   p_entry_kinds[k],p_event_type_id,p_schema_revision,p_retention_profile_id,p_retention_ms) THEN
   RAISE EXCEPTION 'death retirement batch entry refused' USING ERRCODE='23514';
  END IF;
 END LOOP;
 RETURN n;
END $batch$;
DO $migration$
DECLARE f TEXT:='game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint)';
BEGIN
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',f);
 EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
END $migration$;

-- Capture only a current retirement reservation's actual edge. All ancestors
-- must still be live in the same World, with one parent and a bounded acyclic
-- path to the receipt-proven death Ground root. A raw DELETE gets no proof.
CREATE OR REPLACE FUNCTION game_player_death_reset_entry_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $capture$
DECLARE v game_item_decay_retire_reservations%ROWTYPE;
 g game_item_ground_locations%ROWTYPE; d game_player_death_item_records%ROWTYPE;
 current_item UUID; parents UUID[]; visited UUID[]; level INT; kind SMALLINT;
BEGIN
 SELECT * INTO v FROM game_item_decay_retire_reservations
  WHERE item_instance_id=OLD.item_instance_id
   AND cause_kind IN('WORLD_RESET','ACTIVITY_RETIREMENT')
   AND ground_root_item_instance_id<>item_instance_id
   AND placement_ordinal=OLD.placement_ordinal AND world_id=OLD.world_id
  ORDER BY fence_scope_ownership_generation DESC LIMIT 1;
 IF NOT FOUND THEN RETURN OLD; END IF;
 kind:=CASE TG_TABLE_NAME WHEN 'game_item_container_entries' THEN 1
                         WHEN 'game_item_nested_entries' THEN 2 ELSE 0 END;
 IF kind=0 THEN RAISE EXCEPTION 'unexpected death retirement source' USING ERRCODE='23514'; END IF;
 current_item:=OLD.parent_item_instance_id;
 visited:=ARRAY[OLD.item_instance_id];
 FOR level IN 1..8 LOOP
  IF current_item=ANY(visited) OR NOT EXISTS(SELECT 1 FROM game_item_instances
   WHERE item_instance_id=current_item AND world_id=OLD.world_id AND lifecycle=1 AND quantity>0) THEN
   RETURN OLD;
  END IF;
  visited:=visited||current_item;
  SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=current_item;
  IF FOUND THEN
   SELECT * INTO d FROM game_player_death_item_records
    WHERE item_instance_id=current_item AND operation=1 AND source_slot IN(0,6,9,10)
     AND world_id=g.world_id
     AND (scope_kind,channel_id,instance_id) IS NOT DISTINCT FROM (g.scope_kind,g.channel_id,g.instance_id)
     AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(transaction_id);
   IF NOT FOUND OR current_item<>v.ground_root_item_instance_id OR g.world_id<>OLD.world_id
    OR (g.scope_kind,g.channel_id,g.instance_id) IS DISTINCT FROM (v.scope_kind,v.channel_id,v.instance_id)
    OR (d.source_slot=0 AND level>7)
    OR (d.source_slot IN(6,10) AND (level>1 OR NOT EXISTS(SELECT 1 FROM game_item_instances qi
     WHERE qi.item_instance_id=current_item AND qi.definition_production_key IN(
      'oteryn:item.tibia.i35562','oteryn:item.tibia.i35848','oteryn:item.tibia.i35849','oteryn:item.tibia.i35524',
      'oteryn:item.tibia.i36666','oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644'))))
    OR NOT EXISTS(SELECT 1 FROM game_item_decay_retire_reservations root
     WHERE root.item_instance_id=current_item AND root.ground_root_item_instance_id=current_item
      AND root.fence_scope_ownership_generation=v.fence_scope_ownership_generation
      AND (root.world_id,root.scope_kind,root.channel_id,root.instance_id,root.cause_kind,
           root.reset_epoch,root.retirement_binding,root.target_digest,root.activity_handoff_id,root.activity_ownership_generation)
       IS NOT DISTINCT FROM
          (v.world_id,v.scope_kind,v.channel_id,v.instance_id,v.cause_kind,
           v.reset_epoch,v.retirement_binding,v.target_digest,v.activity_handoff_id,v.activity_ownership_generation)) THEN
    RETURN OLD;
   END IF;
   INSERT INTO game_player_death_retirement_edges
    (item_instance_id,parent_item_instance_id,ground_root_item_instance_id,death_transaction_id,
     world_id,scope_kind,channel_id,instance_id,placement_ordinal,entry_kind,depth)
    VALUES(OLD.item_instance_id,OLD.parent_item_instance_id,current_item,d.transaction_id,
     OLD.world_id,g.scope_kind,g.channel_id,g.instance_id,OLD.placement_ordinal,kind,level);
   RETURN OLD;
  END IF;
  SELECT array_agg(parent_item_instance_id) INTO parents FROM (
   SELECT parent_item_instance_id FROM game_item_container_entries
    WHERE item_instance_id=current_item AND world_id=OLD.world_id
   UNION ALL SELECT parent_item_instance_id FROM game_item_nested_entries
    WHERE item_instance_id=current_item AND world_id=OLD.world_id
  ) edges;
  IF cardinality(parents) IS DISTINCT FROM 1 THEN RETURN OLD; END IF;
  current_item:=parents[1];
 END LOOP;
 RETURN OLD;
END $capture$;
CREATE TRIGGER game_player_death_reset_nested_capture BEFORE DELETE ON game_item_nested_entries
 FOR EACH ROW EXECUTE FUNCTION game_player_death_reset_entry_capture();

-- This is only the location alternative. The installed World/Instance proofs
-- still check their live authority, drain/epoch or completed activity, exact
-- reservation/event, conserved item, absence of all custody and immutable audit.
-- Requiring the immediate parent's and root's same-transaction terminal receipt
-- makes partial subtree removal insufficient.
CREATE FUNCTION game_player_death_retirement_entry_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql VOLATILE SECURITY DEFINER AS $proof$
 -- Select the one current edge before joining its shared root. Without this
 -- boundary, the planner can scan every sibling through the root index for
 -- each deferred callback, despite the edge's primary-key Item binding.
 WITH edge AS MATERIALIZED (
  SELECT * FROM game_player_death_retirement_edges
  WHERE item_instance_id=p_item AND created_xact_id=pg_current_xact_id()
 )
 SELECT EXISTS(
  SELECT 1 FROM edge e
  JOIN game_item_decay_retire_receipts r ON r.item_instance_id=e.item_instance_id
  JOIN game_item_decay_retire_receipts parent ON parent.item_instance_id=e.parent_item_instance_id
  JOIN game_item_decay_retire_receipts root ON root.item_instance_id=e.ground_root_item_instance_id
  JOIN game_item_instances root_item ON root_item.item_instance_id=root.item_instance_id
  JOIN game_item_ground_removal_evidence g ON g.item_instance_id=root.item_instance_id
  JOIN game_player_death_item_records d ON d.transaction_id=e.death_transaction_id
  WHERE e.item_instance_id=p_item AND e.created_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND parent.created_xact_id=pg_current_xact_id()
   AND root.created_xact_id=pg_current_xact_id() AND g.created_xact_id=pg_current_xact_id()
   AND r.cause_kind IN('WORLD_RESET','ACTIVITY_RETIREMENT')
   AND e.ground_root_item_instance_id=r.ground_root_item_instance_id
   AND e.placement_ordinal=r.placement_ordinal AND e.world_id=r.world_id
   AND (e.scope_kind,e.channel_id,e.instance_id) IS NOT DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id)
   AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.world_id,r.scope_kind,r.channel_id,r.instance_id)
   AND d.item_instance_id=root.item_instance_id AND d.operation=1 AND d.source_slot IN(0,6,9,10)
   AND (d.world_id,d.scope_kind,d.channel_id,d.instance_id) IS NOT DISTINCT FROM (r.world_id,r.scope_kind,r.channel_id,r.instance_id)
   AND e.depth<=CASE d.source_slot WHEN 9 THEN 8 WHEN 0 THEN 7 ELSE 1 END
   AND (d.source_slot NOT IN(6,10) OR root_item.definition_production_key IN(
    'oteryn:item.tibia.i35562','oteryn:item.tibia.i35848','oteryn:item.tibia.i35849','oteryn:item.tibia.i35524',
    'oteryn:item.tibia.i36666','oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644'))
   AND root.ground_root_item_instance_id=root.item_instance_id AND root.placement_ordinal IS NULL
   AND parent.ground_root_item_instance_id=root.item_instance_id
   AND (root.world_id,root.scope_kind,root.channel_id,root.instance_id,root.cause_kind,
        root.reset_epoch,root.retirement_binding,root.target_digest,root.activity_handoff_id,root.activity_ownership_generation,
        root.fence_scope_ownership_generation)
    IS NOT DISTINCT FROM
       (r.world_id,r.scope_kind,r.channel_id,r.instance_id,r.cause_kind,
        r.reset_epoch,r.retirement_binding,r.target_digest,r.activity_handoff_id,r.activity_ownership_generation,
        r.fence_scope_ownership_generation)
   AND (parent.world_id,parent.scope_kind,parent.channel_id,parent.instance_id,parent.cause_kind,
        parent.reset_epoch,parent.retirement_binding,parent.target_digest,parent.activity_handoff_id,parent.activity_ownership_generation,
        parent.fence_scope_ownership_generation)
    IS NOT DISTINCT FROM
       (r.world_id,r.scope_kind,r.channel_id,r.instance_id,r.cause_kind,
        r.reset_epoch,r.retirement_binding,r.target_digest,r.activity_handoff_id,r.activity_ownership_generation,
        r.fence_scope_ownership_generation)
 )
$proof$;

-- Every entry requires the root's same-transaction receipt above. Its mandatory
-- deferred consistency proof checks the complete tree's bounds, without a cache
-- or a whole-tree scan for each descendant. Ordinary roots have no such edges.
CREATE FUNCTION game_player_death_retirement_root_bounds_proven(p_root UUID) RETURNS BOOLEAN
LANGUAGE sql VOLATILE SECURITY DEFINER AS $bounds$
 SELECT coalesce(sum(parent_entries),0)<=coalesce(min(descendants_limit),0)
  AND coalesce(max(parent_entries),0)<=20
 FROM (
  SELECT e.parent_item_instance_id,count(*) AS parent_entries,
   min(CASE d.source_slot WHEN 9 THEN 500 WHEN 0 THEN 499 ELSE 20 END) AS descendants_limit
  FROM game_player_death_retirement_edges e
  JOIN game_player_death_item_records d ON d.transaction_id=e.death_transaction_id
  WHERE e.ground_root_item_instance_id=p_root AND e.created_xact_id=pg_current_xact_id()
  GROUP BY e.parent_item_instance_id
 ) parents
$bounds$;

-- Insert the new location alternative into the latest installed proof bodies;
-- every conservation, authority and terminal predicate outside it is retained.
DO $migration$
DECLARE f TEXT; body TEXT; amended TEXT; at_begin INT; old_arm TEXT; new_arm TEXT;
 receipt_lookup TEXT; root_gate TEXT;
BEGIN
 old_arm:='OR (r.placement_ordinal IS NOT NULL AND (';
 new_arm:='OR (r.placement_ordinal IS NOT NULL AND NOT game_player_death_retirement_entry_proven(p_item) AND (';
 receipt_lookup:='IF NOT FOUND THEN RETURN FALSE; END IF;';
 root_gate:=$root_gate$
 IF r.placement_ordinal IS NULL THEN
  IF NOT game_player_death_retirement_root_bounds_proven(p_item) THEN
   RAISE EXCEPTION 'death Ground retirement tree exceeds admitted bounds' USING ERRCODE='23514';
  END IF;
 END IF;
$root_gate$;
 FOREACH f IN ARRAY ARRAY['game_world_reset_item_proven(uuid)','game_activity_retirement_item_proven(uuid)'] LOOP
  body:=pg_get_functiondef(to_regprocedure(f));
  IF (length(body)-length(replace(body,old_arm,'')))/length(old_arm)<>1 THEN
   RAISE EXCEPTION 'missing exact retirement entry proof in %',f;
  END IF;
  amended:=replace(body,old_arm,new_arm);
  IF (length(amended)-length(replace(amended,receipt_lookup,'')))/length(receipt_lookup)<>1 THEN
   RAISE EXCEPTION 'missing exact retirement receipt lookup in %',f;
  END IF;
  EXECUTE replace(amended,receipt_lookup,receipt_lookup||root_gate);
 END LOOP;
 -- The UPDATE callback otherwise walks every later operation-family arm
 -- before reaching the existing retirement proof. Recognize only this exact
 -- terminal UPDATE, then run that same full proof now. Earlier UPDATE events
 -- retain their own transaction binding and fall through to the installed body.
 -- A current operation-family collision must keep its original ordering even
 -- if its constraint was already checked IMMEDIATE. These indexed lookups are
 -- a conservative superset of the earlier effectful arms, without their JSON
 -- expansion or repeated completion calls. MAIL is bound by item, not by tx.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 IF position('game_world_reset_item_proven(NEW.item_instance_id)' IN body)=0
  OR position('game_activity_retirement_item_proven(NEW.item_instance_id)' IN body)=0 THEN
  RAISE EXCEPTION 'missing installed retirement Item proof';
 END IF;
 at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at_begin=0 THEN RAISE EXCEPTION 'missing retirement Item guard entry'; END IF;
 amended:=overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND OLD.quantity>0
  AND NEW.lifecycle=2 AND NEW.quantity=0
  AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
  AND NEW.state_revision=OLD.state_revision+1
  AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
      =(to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) THEN
  IF EXISTS(SELECT 1 FROM game_item_decay_retire_receipts r
   WHERE r.item_instance_id=NEW.item_instance_id AND r.world_id=NEW.world_id
    AND r.transaction_id=NEW.last_transaction_id AND r.quantity_before=OLD.quantity
    AND r.cause_kind IN('WORLD_RESET','ACTIVITY_RETIREMENT')
    AND r.created_xact_id=pg_current_xact_id()) THEN
   IF NOT EXISTS(
    SELECT 1 FROM game_mail_operations r
     WHERE r.root_item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_atelier_item_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_bazaar_item_transitions r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_carried_step_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_character_stash_item_lines r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_coin_exchange_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_floor_assembly_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_floor_transform_expiry_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_floor_transform_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_forge_operations r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_house_disposition_wrap_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_house_furniture_destroy_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_house_furniture_wrap_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_house_timed_decoration_expiries r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_item_consume_transform_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_item_craft_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_item_imbuement_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_item_marriage_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_item_write_once_transforms r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_market_item_lines r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_player_death_item_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
   ) THEN
    IF game_world_reset_item_proven(NEW.item_instance_id)
     OR game_activity_retirement_item_proven(NEW.item_instance_id) THEN RETURN NULL; END IF;
   END IF;
  END IF;
 END IF;
$arm$ from at_begin for 5);
 EXECUTE amended;
 FOREACH f IN ARRAY ARRAY['game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef(to_regprocedure(f||'()'));
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at_begin=0 THEN RAISE EXCEPTION 'missing retirement custody guard %',f; END IF;
  amended:=overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_player_death_retirement_edges e
  WHERE e.item_instance_id=OLD.item_instance_id AND e.parent_item_instance_id=OLD.parent_item_instance_id
   AND e.world_id=OLD.world_id AND e.placement_ordinal=OLD.placement_ordinal
   AND e.entry_kind=CASE TG_TABLE_NAME WHEN 'game_item_container_entries' THEN 1 ELSE 2 END
   AND e.created_xact_id=pg_current_xact_id())
  AND (game_world_reset_item_proven(OLD.item_instance_id)
       OR game_activity_retirement_item_proven(OLD.item_instance_id)) THEN RETURN NULL; END IF;
$arm$ from at_begin for 5);
  EXECUTE amended;
 END LOOP;
 FOREACH f IN ARRAY ARRAY['game_player_death_reset_entry_capture()','game_player_death_retirement_entry_proven(uuid)',
  'game_player_death_retirement_root_bounds_proven(uuid)',
  'game_world_reset_item_proven(uuid)','game_activity_retirement_item_proven(uuid)',
  'game_item_instance_change_proven()',
  'game_item_container_entry_removal_proven()','game_item_nested_entry_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SECURITY DEFINER SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $migration$;
