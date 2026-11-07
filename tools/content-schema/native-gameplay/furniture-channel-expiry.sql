
-- Ground clocks belong to the current Channel/node, independently of the
-- character that last moved or lit the decoration.
CREATE FUNCTION game_furniture_ground_clock_fence(p_node UUID,p_node_revision NUMERIC,p_world UUID,p_channel UUID,p_scope NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
  OR game_node_lock_current_registration(p_node,p_node_revision) IS DISTINCT FROM TRUE
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments runtime
   JOIN game_durability_admission_runtime_guards g ON g.scope_key=runtime.scope_key
   WHERE runtime.scope_kind=1 AND runtime.world_id=p_world AND runtime.channel_id=p_channel
    AND runtime.state=1 AND runtime.ownership_generation=p_scope
    AND runtime.holder_node_id=p_node AND runtime.holder_registration_revision=p_node_revision
    AND g.ready AND g.ownership_generation=p_scope)
 THEN RAISE EXCEPTION 'Ground furniture clock requires current Channel node and generation' USING ERRCODE='23514'; END IF;
END $$;
-- W18 carried furniture retains the original existing timed-state deadline
-- across House exit. This is a current-Channel adapter, never another clock.
CREATE FUNCTION game_furniture_carried_clock_fence(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_channel UUID)
 RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
  OR game_node_lock_current_registration(p_node,p_node_revision) IS DISTINCT FROM TRUE
  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
   JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id AND c.account_id=s.account_id
   JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
   WHERE s.game_session_id=p_session AND s.character_id=p_character AND s.world_id=p_world
    AND s.session_state IN(1,2) AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=p_world
    AND s.runtime_scope_channel_id=p_channel AND s.current_generation=p_connection
    AND s.character_lease_generation=p_lease AND s.scope_ownership_generation=p_scope
    AND c.eligible AND c.lease_generation=p_lease AND c.holder_game_session_id=p_session
    AND a.presence_character_id=p_character AND a.holder_game_session_id=p_session
    AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments runtime
     JOIN game_durability_admission_runtime_guards g ON g.scope_key=runtime.scope_key
     WHERE runtime.scope_kind=1 AND runtime.world_id=p_world AND runtime.channel_id=p_channel
      AND runtime.state=1 AND runtime.ownership_generation=p_scope
      AND runtime.holder_node_id=p_node AND runtime.holder_registration_revision=p_node_revision
      AND g.ready AND g.ownership_generation=p_scope))
 THEN RAISE EXCEPTION 'carried furniture timer requires current Channel, Character, account and node' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_furniture_carried_clock_source(p_character UUID,p_world UUID,p_item UUID)
 RETURNS TABLE(before_item JSONB,location JSONB,provenance JSONB)
 LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules schedule
   WHERE schedule.item_instance_id=p_item AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts done WHERE done.item_instance_id=p_item)
    AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries done WHERE done.item_instance_id=p_item))
 THEN RETURN; END IF;
 RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('custody_kind','container','custody',to_jsonb(e)),'null'::jsonb
  FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id)
  JOIN game_item_container_slots bag ON bag.character_id=e.character_id AND bag.world_id=e.world_id
   AND bag.item_instance_id=e.parent_item_instance_id
  WHERE i.item_instance_id=p_item AND i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1
   AND i.definition_family='Item' AND e.character_id=p_character FOR UPDATE OF i,e;
 IF FOUND THEN RETURN; END IF;
 RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('custody_kind','nested','custody',to_jsonb(e)),'null'::jsonb
  FROM game_item_instances i JOIN game_item_nested_entries e USING(item_instance_id,world_id),
  LATERAL game_item_loop_owner(e.parent_item_instance_id) owner
  WHERE i.item_instance_id=p_item AND i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1
   AND i.definition_family='Item' AND owner.owner_character=p_character AND owner.depth BETWEEN 0 AND 7
  FOR UPDATE OF i,e;
END $$;

-- The existing timed row has no House/session context. Retain that exact
-- authority and before/after snapshots in one immutable receipt for its expiry.
CREATE TABLE game_furniture_carried_decay_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_timed_state_writes(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 character_id UUID NULL REFERENCES game_character_roots,
 context JSONB NOT NULL,
 before_item JSONB NOT NULL,after_item JSONB NOT NULL,
 before_timer JSONB NOT NULL,after_timer JSONB NOT NULL,
 location JSONB NOT NULL,provenance JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_furniture_carried_decay_character ON game_furniture_carried_decay_receipts(character_id);
REVOKE ALL ON game_furniture_carried_decay_receipts FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_furniture_carried_decay_receipts TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_furniture_carried_decay_immutable BEFORE UPDATE OR DELETE ON game_furniture_carried_decay_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_furniture_carried_decay_no_truncate BEFORE TRUNCATE ON game_furniture_carried_decay_receipts
 EXECUTE FUNCTION game_item_reject_truncate();

-- Skip complete cosmetic cycles after an unoccupied House was overdue. The
-- next phase remains source-authored, so at most four regular ticks settle a
-- clock/tesseract regardless of how long the House was empty.
CREATE FUNCTION game_furniture_carried_decay_complete(p_tx UUID) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_furniture_carried_decay_receipts%ROWTYPE; w game_item_timed_state_writes%ROWTYPE;
 life game_house_furniture_lifetimes%ROWTYPE; next_life game_house_furniture_lifetimes%ROWTYPE;
 expected JSONB; source_id UUID; after_key TEXT; burned BOOLEAN; kind TEXT; cycle_ms BIGINT; skipped BIGINT;
BEGIN
 SELECT * INTO r FROM game_furniture_carried_decay_receipts WHERE transaction_id=p_tx;
 SELECT * INTO w FROM game_item_timed_state_writes WHERE transaction_id=p_tx;
 IF r.transaction_id IS NULL OR w.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
  OR w.created_xact_id<>pg_current_xact_id() OR w.cause<>4 OR w.holder_character_id IS DISTINCT FROM r.character_id
  OR w.committed_at<(r.before_timer->>'deadline_at')::bigint
  OR (r.before_timer->>'deadline_at') IS NULL OR w.charges_before IS NOT NULL OR w.charges_after IS NOT NULL
  OR w.full_charges IS NOT NULL THEN RETURN FALSE; END IF;
 IF r.context->>'kind'='ground' THEN
  IF r.character_id IS NOT NULL THEN RETURN FALSE; END IF;
  PERFORM game_furniture_ground_clock_fence((r.context->>'node')::uuid,(r.context->>'node_revision')::numeric,
    (r.context->>'world')::uuid,(r.context->>'channel')::uuid,(r.context->>'scope')::numeric);
 ELSE
 PERFORM game_furniture_carried_clock_fence((r.context->>'session')::uuid,r.character_id,
  (r.context->>'connection')::numeric,(r.context->>'lease')::numeric,(r.context->>'scope')::numeric,
  (r.context->>'node')::uuid,(r.context->>'node_revision')::numeric,
  (r.context->>'world')::uuid,(r.context->>'channel')::uuid);
 END IF;
 source_id:=(r.before_item->>'item_instance_id')::uuid;
 SELECT * INTO life FROM game_house_furniture_lifetimes
  WHERE 'oteryn:item.tibia.i'||item_id::text=r.before_item->>'definition_production_key';
 IF NOT FOUND OR life.duration_ms IS NULL OR source_id<>w.item_instance_id
  OR r.before_item->>'definition_family'<>'Item' OR r.before_item->>'lifecycle'<>'1'
  OR r.before_item->>'quantity'<>'1' OR r.before_timer->>'item_instance_id'<>source_id::text
  OR w.expected_revision<>(r.before_timer->>'state_revision')::numeric
  OR w.remaining_ms_before IS DISTINCT FROM (r.before_timer->>'remaining_ms')::bigint
  OR w.deadline_before IS DISTINCT FROM (r.before_timer->>'deadline_at')::bigint THEN RETURN FALSE; END IF;
 cycle_ms:=game_house_furniture_cycle_ms(life.item_id);
 skipped:=CASE WHEN cycle_ms IS NULL THEN 0 ELSE
  (((r.context->>'settled_at')::bigint-w.deadline_before)/cycle_ms)*cycle_ms END;
 IF (r.context->>'settled_at')::bigint<w.deadline_before
  OR (r.context->>'settled_at')::bigint>w.committed_at THEN RETURN FALSE; END IF;
 burned:=life.target_id IS NULL;
 after_key:=CASE WHEN burned THEN r.before_item->>'definition_production_key' ELSE 'oteryn:item.tibia.i'||life.target_id::text END;
 SELECT * INTO next_life FROM game_house_furniture_lifetimes WHERE item_id=life.target_id;
 expected:=r.before_item||jsonb_build_object('definition_production_key',after_key,'last_transaction_id',p_tx,
  'state_revision',(r.before_item->>'state_revision')::numeric+1,
  'quantity',CASE WHEN burned THEN 0 ELSE 1 END,'lifecycle',CASE WHEN burned THEN 2 ELSE 1 END);
 IF r.after_item<>expected OR w.definition_before_family<>'Item' OR w.definition_after_family<>'Item'
  OR w.definition_before_production_key<>r.before_item->>'definition_production_key'
  OR w.definition_before_revision_ref<>r.before_item->>'definition_revision_ref'
  OR w.definition_after_production_key<>after_key OR w.definition_after_revision_ref<>r.before_item->>'definition_revision_ref'
  OR w.remaining_ms_after IS DISTINCT FROM next_life.duration_ms
  OR w.deadline_after IS DISTINCT FROM w.deadline_before+next_life.duration_ms+skipped
  OR w.full_remaining_ms IS DISTINCT FROM next_life.duration_ms OR w.lit_continuous<>(next_life.duration_ms IS NOT NULL)
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=source_id AND to_jsonb(i)=r.after_item)
  THEN RETURN FALSE; END IF;
 expected:=r.before_timer||jsonb_build_object('charges',NULL,'remaining_ms',w.remaining_ms_after,
  'deadline_at',w.deadline_after,'state_revision',w.expected_revision+1);
 IF r.after_timer<>expected OR NOT EXISTS(SELECT 1 FROM game_item_timed_states t
  WHERE t.item_instance_id=source_id AND to_jsonb(t)=r.after_timer) THEN RETURN FALSE; END IF;
 kind:=r.location->>'custody_kind';
 IF burned THEN RETURN
  NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=source_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=source_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=source_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=source_id)
  AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=source_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=source_id);
 END IF;
 RETURN CASE kind
  WHEN 'ground' THEN r.context->>'kind'='ground' AND EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=source_id
    AND to_jsonb(g)=r.location->'custody' AND g.scope_kind=1
    AND g.world_id=(r.context->>'world')::uuid AND g.channel_id=(r.context->>'channel')::uuid
    AND g.runtime_scope_ownership_generation=(r.context->>'scope')::numeric)
  WHEN 'house' THEN FALSE AND EXISTS(SELECT 1 FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   WHERE h.item_instance_id=source_id AND to_jsonb(h)=r.location->'custody' AND to_jsonb(p)=r.provenance)
  WHEN 'container' THEN EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.item_instance_id=source_id AND to_jsonb(c)=r.location->'custody' AND c.character_id=r.character_id)
  WHEN 'nested' THEN EXISTS(SELECT 1 FROM game_item_nested_entries c,LATERAL game_item_loop_owner(c.parent_item_instance_id) owner
   WHERE c.item_instance_id=source_id AND to_jsonb(c)=r.location->'custody' AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)
  WHEN 'equipment' THEN EXISTS(SELECT 1 FROM game_character_equipment_slots c WHERE c.item_instance_id=source_id AND to_jsonb(c)=r.location->'custody' AND c.character_id=r.character_id)
  ELSE FALSE END;
END $$;
CREATE FUNCTION game_furniture_carried_decay_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_furniture_carried_decay_complete(NEW.transaction_id) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'incomplete fenced House decoration expiry' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_furniture_carried_decay_proven AFTER INSERT ON game_furniture_carried_decay_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_furniture_carried_decay_proven();

CREATE FUNCTION game_furniture_carried_decay_expire(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_channel UUID)
 RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c RECORD; src RECORD; t game_item_timed_states%ROWTYPE; w game_item_timed_state_writes%ROWTYPE;
 life game_house_furniture_lifetimes%ROWTYPE; next_life game_house_furniture_lifetimes%ROWTYPE;
 before_item JSONB;after_item JSONB;after_timer JSONB;tx UUID; total INTEGER:=0;burned BOOLEAN; settled BIGINT;cycle_ms BIGINT;skipped BIGINT;
BEGIN
 PERFORM game_furniture_carried_clock_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_channel);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 FOR c IN SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_timed_states timer USING(item_instance_id)
  JOIN game_house_furniture_lifetimes lifetime ON 'oteryn:item.tibia.i'||lifetime.item_id::text=i.definition_production_key
  LEFT JOIN LATERAL game_item_loop_owner(i.item_instance_id) owner ON TRUE
  WHERE i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1 AND lifetime.duration_ms IS NOT NULL
   AND timer.deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
   AND timer.charges IS NULL AND owner.owner_character=p_character
   AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
  ORDER BY i.item_instance_id LIMIT 16 FOR UPDATE OF i,timer LOOP
  SELECT * INTO src FROM game_furniture_carried_clock_source(p_character,p_world,c.item_instance_id);
  IF NOT FOUND THEN CONTINUE; END IF;
  before_item:=src.before_item;
  SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=c.item_instance_id FOR UPDATE;
  SELECT * INTO life FROM game_house_furniture_lifetimes WHERE 'oteryn:item.tibia.i'||item_id::text=before_item->>'definition_production_key';
  SELECT * INTO next_life FROM game_house_furniture_lifetimes WHERE item_id=life.target_id;
  IF t.deadline_at IS NULL OR t.deadline_at>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
   OR t.charges IS NOT NULL OR life.duration_ms IS NULL THEN CONTINUE; END IF;
  tx:=game_character_uuid_v7();burned:=life.target_id IS NULL;
  IF burned AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=c.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=c.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=c.item_instance_id)) THEN CONTINUE; END IF;
  settled:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  cycle_ms:=game_house_furniture_cycle_ms(life.item_id);
  skipped:=CASE WHEN cycle_ms IS NULL THEN 0 ELSE ((settled-t.deadline_at)/cycle_ms)*cycle_ms END;
  w:=NULL;w.item_instance_id:=c.item_instance_id;w.expected_revision:=t.state_revision;w.cause:=4;
  w.transaction_id:=tx;w.holder_character_id:=p_character;w.definition_before_family:='Item';w.definition_after_family:='Item';
  w.definition_before_production_key:=before_item->>'definition_production_key';w.definition_before_revision_ref:=before_item->>'definition_revision_ref';
  w.definition_after_production_key:=CASE WHEN burned THEN w.definition_before_production_key ELSE 'oteryn:item.tibia.i'||life.target_id::text END;
  w.definition_after_revision_ref:=w.definition_before_revision_ref;w.lit_continuous:=next_life.duration_ms IS NOT NULL;
  w.full_remaining_ms:=next_life.duration_ms;w.remaining_ms_before:=t.remaining_ms;w.deadline_before:=t.deadline_at;
  w.remaining_ms_after:=next_life.duration_ms;w.deadline_after:=t.deadline_at+next_life.duration_ms+skipped;
  after_item:=before_item||jsonb_build_object('definition_production_key',w.definition_after_production_key,
   'last_transaction_id',tx,'state_revision',(before_item->>'state_revision')::numeric+1,
   'quantity',CASE WHEN burned THEN 0 ELSE 1 END,'lifecycle',CASE WHEN burned THEN 2 ELSE 1 END);
  after_timer:=to_jsonb(t)||jsonb_build_object('charges',NULL,'remaining_ms',w.remaining_ms_after,
   'deadline_at',w.deadline_after,'state_revision',t.state_revision+1);
  INSERT INTO game_furniture_carried_decay_receipts(transaction_id,character_id,context,before_item,after_item,before_timer,after_timer,location,provenance)
   VALUES(tx,p_character,jsonb_build_object('session',p_session,'connection',p_connection,'lease',p_lease,'scope',p_scope,
    'node',p_node,'node_revision',p_node_revision,'world',p_world,'channel',p_channel,'settled_at',settled),before_item,after_item,to_jsonb(t),after_timer,src.location,src.provenance);
  INSERT INTO game_item_timed_state_writes SELECT(w).*;
  UPDATE game_item_instances SET definition_production_key=w.definition_after_production_key,last_transaction_id=tx,
   state_revision=state_revision+1,quantity=CASE WHEN burned THEN 0 ELSE 1 END,lifecycle=CASE WHEN burned THEN 2 ELSE 1 END
   WHERE item_instance_id=c.item_instance_id;
  UPDATE game_item_timed_states SET charges=NULL,remaining_ms=w.remaining_ms_after,deadline_at=w.deadline_after,state_revision=state_revision+1
   WHERE item_instance_id=c.item_instance_id;
  IF burned THEN
   DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_house_interior_locations WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_container_entries WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_nested_entries WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_character_equipment_slots WHERE item_instance_id=c.item_instance_id;
  END IF;
  total:=total+1;
 END LOOP;
 RETURN total;
END $$;

CREATE FUNCTION game_furniture_ground_decay_expire(p_node UUID,p_node_revision NUMERIC,p_world UUID,p_channel UUID,p_scope NUMERIC)
 RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c RECORD; src RECORD; t game_item_timed_states%ROWTYPE; w game_item_timed_state_writes%ROWTYPE;
 life game_house_furniture_lifetimes%ROWTYPE; next_life game_house_furniture_lifetimes%ROWTYPE;
 before_item JSONB;after_item JSONB;after_timer JSONB;tx UUID; total INTEGER:=0;burned BOOLEAN; settled BIGINT;cycle_ms BIGINT;skipped BIGINT;
BEGIN
 PERFORM game_furniture_ground_clock_fence(p_node,p_node_revision,p_world,p_channel,p_scope);
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(p_world),'hex') || encode(uuid_send(p_channel),'hex'),33));
 FOR c IN SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_timed_states timer USING(item_instance_id)
  JOIN game_house_furniture_lifetimes lifetime ON 'oteryn:item.tibia.i'||lifetime.item_id::text=i.definition_production_key
  JOIN game_item_ground_locations ground USING(item_instance_id,world_id)
  WHERE i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1 AND lifetime.duration_ms IS NOT NULL
   AND timer.deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
   AND timer.charges IS NULL AND ground.scope_kind=1 AND ground.channel_id=p_channel AND ground.runtime_scope_ownership_generation=p_scope
   AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules schedule WHERE schedule.item_instance_id=i.item_instance_id
    AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts done WHERE done.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries done WHERE done.item_instance_id=i.item_instance_id))
  ORDER BY i.item_instance_id LIMIT 16 FOR UPDATE OF i,timer LOOP
  SELECT to_jsonb(i) AS before_item,jsonb_build_object('custody_kind','ground','custody',to_jsonb(g)) AS location,'null'::jsonb AS provenance INTO src
   FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id)
   WHERE i.item_instance_id=c.item_instance_id AND g.scope_kind=1 AND g.world_id=p_world AND g.channel_id=p_channel
    AND g.runtime_scope_ownership_generation=p_scope FOR UPDATE OF i,g;
  IF NOT FOUND THEN CONTINUE; END IF;
  before_item:=src.before_item;
  SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=c.item_instance_id FOR UPDATE;
  SELECT * INTO life FROM game_house_furniture_lifetimes WHERE 'oteryn:item.tibia.i'||item_id::text=before_item->>'definition_production_key';
  SELECT * INTO next_life FROM game_house_furniture_lifetimes WHERE item_id=life.target_id;
  IF t.deadline_at IS NULL OR t.deadline_at>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
   OR t.charges IS NOT NULL OR life.duration_ms IS NULL THEN CONTINUE; END IF;
  tx:=game_character_uuid_v7();burned:=life.target_id IS NULL;
  IF burned AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=c.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=c.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=c.item_instance_id)) THEN CONTINUE; END IF;
  settled:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  cycle_ms:=game_house_furniture_cycle_ms(life.item_id);
  skipped:=CASE WHEN cycle_ms IS NULL THEN 0 ELSE ((settled-t.deadline_at)/cycle_ms)*cycle_ms END;
  w:=NULL;w.item_instance_id:=c.item_instance_id;w.expected_revision:=t.state_revision;w.cause:=4;
  w.transaction_id:=tx;w.holder_character_id:=NULL;w.definition_before_family:='Item';w.definition_after_family:='Item';
  w.definition_before_production_key:=before_item->>'definition_production_key';w.definition_before_revision_ref:=before_item->>'definition_revision_ref';
  w.definition_after_production_key:=CASE WHEN burned THEN w.definition_before_production_key ELSE 'oteryn:item.tibia.i'||life.target_id::text END;
  w.definition_after_revision_ref:=w.definition_before_revision_ref;w.lit_continuous:=next_life.duration_ms IS NOT NULL;
  w.full_remaining_ms:=next_life.duration_ms;w.remaining_ms_before:=t.remaining_ms;w.deadline_before:=t.deadline_at;
  w.remaining_ms_after:=next_life.duration_ms;w.deadline_after:=t.deadline_at+next_life.duration_ms+skipped;
  after_item:=before_item||jsonb_build_object('definition_production_key',w.definition_after_production_key,
   'last_transaction_id',tx,'state_revision',(before_item->>'state_revision')::numeric+1,
   'quantity',CASE WHEN burned THEN 0 ELSE 1 END,'lifecycle',CASE WHEN burned THEN 2 ELSE 1 END);
  after_timer:=to_jsonb(t)||jsonb_build_object('charges',NULL,'remaining_ms',w.remaining_ms_after,
   'deadline_at',w.deadline_after,'state_revision',t.state_revision+1);
  INSERT INTO game_furniture_carried_decay_receipts(transaction_id,character_id,context,before_item,after_item,before_timer,after_timer,location,provenance)
   VALUES(tx,NULL,jsonb_build_object('kind','ground','scope',p_scope,
    'node',p_node,'node_revision',p_node_revision,'world',p_world,'channel',p_channel,'settled_at',settled),before_item,after_item,to_jsonb(t),after_timer,src.location,src.provenance);
  INSERT INTO game_item_timed_state_writes SELECT(w).*;
  UPDATE game_item_instances SET definition_production_key=w.definition_after_production_key,last_transaction_id=tx,
   state_revision=state_revision+1,quantity=CASE WHEN burned THEN 0 ELSE 1 END,lifecycle=CASE WHEN burned THEN 2 ELSE 1 END
   WHERE item_instance_id=c.item_instance_id;
  UPDATE game_item_timed_states SET charges=NULL,remaining_ms=w.remaining_ms_after,deadline_at=w.deadline_after,state_revision=state_revision+1
   WHERE item_instance_id=c.item_instance_id;
  IF burned THEN
   DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_house_interior_locations WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_container_entries WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_nested_entries WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_character_equipment_slots WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_ground_locations WHERE item_instance_id=c.item_instance_id;
  END IF;
  total:=total+1;
 END LOOP;
 RETURN total;
END $$;

-- Existing guards accept only this exact current-XID receipt; every previous
-- guard body remains present and runs for every other mutation.
DO $migration$ DECLARE name TEXT; body TEXT; arm TEXT; begin_at INTEGER; BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_timed_state_write_consistency_guard','game_house_location_proven',
  'game_item_container_entry_removal_proven','game_item_nested_entry_proven','game_item_ground_removal_proven'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'SRV7 missing decoration timer guard: %',name; END IF;
  IF name='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_furniture_carried_decay_receipts srv7_furniture_receipt WHERE
    srv7_furniture_receipt.transaction_id=NEW.last_transaction_id AND srv7_furniture_receipt.before_item=to_jsonb(OLD) AND srv7_furniture_receipt.after_item=to_jsonb(NEW)
    AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;
$arm$;
  ELSIF name='game_item_instance_change_proven' THEN arm:=$arm$
   IF game_furniture_carried_decay_complete(NEW.last_transaction_id) THEN RETURN NULL; END IF;
$arm$;
  ELSIF name='game_item_timed_state_write_consistency_guard' THEN arm:=$arm$
   IF game_furniture_carried_decay_complete(NEW.transaction_id) THEN RETURN NULL; END IF;
$arm$;
  ELSE arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_furniture_carried_decay_receipts srv7_furniture_receipt
    WHERE srv7_furniture_receipt.location->'custody'=to_jsonb(OLD) AND srv7_furniture_receipt.after_item->>'lifecycle'='2'
     AND game_furniture_carried_decay_complete(srv7_furniture_receipt.transaction_id)) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;


DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_furniture_carried_clock_fence(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,uuid)'::regprocedure,
  'game_furniture_carried_clock_source(uuid,uuid,uuid)'::regprocedure,
  'game_furniture_carried_decay_complete(uuid)'::regprocedure,
  'game_furniture_carried_decay_proven()'::regprocedure,
  'game_furniture_carried_decay_expire(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,uuid)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_furniture_carried_decay_expire(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,uuid) TO oteryn_game_runtime;
DO $access$
DECLARE f regprocedure;
BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
  WHERE n.nspname=current_schema() AND p.proname IN('game_furniture_ground_clock_fence','game_furniture_carried_clock_fence','game_furniture_carried_clock_source','game_furniture_carried_decay_complete','game_furniture_carried_decay_proven','game_furniture_carried_decay_expire','game_furniture_ground_decay_expire') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
 GRANT EXECUTE ON FUNCTION game_furniture_carried_decay_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,UUID),game_furniture_ground_decay_expire(UUID,NUMERIC,UUID,UUID,NUMERIC) TO oteryn_game_runtime;
END $access$;

-- Existing deletion proof functions include invoker paths; only the validated
-- read-only completion predicate is callable in addition to the two expiry owners.
GRANT EXECUTE ON FUNCTION game_furniture_carried_decay_complete(UUID) TO oteryn_game_runtime;
