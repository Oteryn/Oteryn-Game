-- Crystal enchanting.lua:107-112; paid enchantment preserves clicked Ground/House UUID.
CREATE FUNCTION game_dracoyle_in_place_record(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(r.recipe='oteryn:seasonal.dracoyle' AND r.source_kind IN(1,3,4,5)
  AND r.effects->>'dracoyle_in_place'='true',FALSE)
$$;
CREATE FUNCTION game_dracoyle_in_place_custody(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN r.source_kind=5 THEN game_item_action_house_custody(r)
  WHEN r.source_kind=1 THEN game_item_action_furniture_custody(r)
  WHEN r.source_kind IN(3,4) THEN game_item_action_custody_current(r) ELSE FALSE END
$$;
CREATE FUNCTION game_dracoyle_in_place_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE;e JSONB;node UUID;node_revision NUMERIC;t JSONB;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 SELECT to_jsonb(timer) INTO t FROM game_item_timed_states timer WHERE timer.item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF game_dracoyle_in_place_record(r) IS DISTINCT FROM TRUE OR i.item_instance_id IS NULL
  OR to_jsonb(i) IS DISTINCT FROM r.before_item OR r.before_secondary IS DISTINCT FROM r.before_item
  OR r.secondary_item_instance_id IS DISTINCT FROM r.source_item_instance_id OR r.consume_source OR r.source_count<>0
  OR r.outputs<>'[]'::jsonb OR jsonb_array_length(r.extra_inputs)<>1 OR ((r.source_kind IN(1,4)) IS DISTINCT FROM (r.parent_item_instance_id IS NOT NULL))
  OR i.lifecycle<>1 OR i.quantity<>1 OR i.world_id<>r.world_id OR i.definition_family<>'Item'
  OR i.definition_production_key NOT IN('oteryn:item.tibia.i9035','oteryn:item.tibia.i9040')
  OR i.state_revision>=18446744073709551615 OR r.content_digest IS NULL
  OR game_dracoyle_in_place_custody(r) IS DISTINCT FROM TRUE
  OR r.effects->'house_after' IS DISTINCT FROM jsonb_build_object('key',CASE WHEN i.definition_production_key='oteryn:item.tibia.i9035' THEN 'oteryn:item.tibia.i9034' ELSE 'oteryn:item.tibia.i9039' END,'revision',i.definition_revision_ref)
  OR r.effects->>'kind' IS DISTINCT FROM 'transform' OR r.effects->>'source_item_id' IS DISTINCT FROM substring(i.definition_production_key FROM 20)
  OR r.effects->'cues' IS DISTINCT FROM '[{"binding":"appearance:effect/magic_green","effect_id":18}]'::jsonb
  OR r.effects->'house_temporal' IS DISTINCT FROM jsonb_build_object('before',t,'duration',604800000,'target',i.definition_production_key)
  OR (r.source_kind IN(1,4) AND t IS NULL) OR t->>'charges' IS NOT NULL OR t->>'deadline_at' IS NOT NULL
  OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=i.item_instance_id
   AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts d WHERE d.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries d WHERE d.item_instance_id=i.item_instance_id))
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'Dracoyle requires exact inactive same-custody predecessor and seven-day clock' USING ERRCODE='23514'; END IF;
 e:=r.extra_inputs->0;
 IF e->'before'->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i677'
  OR e->'before'->>'definition_family' IS DISTINCT FROM 'Item' OR e->>'count' IS DISTINCT FROM '1'
 THEN RAISE EXCEPTION 'Dracoyle spends exactly one real carried emerald' USING ERRCODE='23514'; END IF;
 PERFORM game_item_action_extra_admit(r);
 node:=(r.effects->'dracoyle_node'->>'id')::uuid;
 node_revision:=(r.effects->'dracoyle_node'->>'revision')::numeric;
 IF node IS NULL OR node_revision IS NULL OR game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
  OR game_node_lock_current_registration(node,node_revision) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Dracoyle enchant requires current Node registration' USING ERRCODE='23514'; END IF;
 IF r.source_kind=5 THEN
  IF node IS DISTINCT FROM (r.source_location->>'node')::uuid
   OR node_revision IS DISTINCT FROM (r.source_location->>'node_revision')::numeric THEN
   RAISE EXCEPTION 'Dracoyle enchant House Node mismatch' USING ERRCODE='23514'; END IF;
  PERFORM game_house_podium_fence(r.game_session_id,r.character_id,
   (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
   (r.audit_context->>'scope_generation')::numeric,node,node_revision,r.world_id,r.source_location->'house'->>'house_key');
 ELSE
  IF node IS NULL OR node_revision IS NULL
   OR game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
   OR game_node_lock_current_registration(node,node_revision) IS DISTINCT FROM TRUE
   OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
    JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id AND c.account_id=s.account_id
    JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
    JOIN game_runtime_scope_assignments runtime ON runtime.scope_kind=1 AND runtime.world_id=r.world_id
     AND runtime.channel_id=s.runtime_scope_channel_id
    JOIN game_durability_admission_runtime_guards g ON g.scope_key=runtime.scope_key
    WHERE s.game_session_id=r.game_session_id AND s.character_id=r.character_id AND s.world_id=r.world_id
     AND s.session_state IN(1,2) AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=r.world_id
     AND s.current_generation=(r.audit_context->>'connection_generation')::numeric
     AND s.character_lease_generation=(r.audit_context->>'lease_generation')::numeric
     AND s.scope_ownership_generation=(r.audit_context->>'scope_generation')::numeric
     AND c.eligible AND c.lease_generation=s.character_lease_generation AND c.holder_game_session_id=s.game_session_id
     AND a.presence_character_id=c.character_id AND a.holder_game_session_id=s.game_session_id
     AND runtime.state=1 AND runtime.ownership_generation=s.scope_ownership_generation
     AND runtime.holder_node_id=node AND runtime.holder_registration_revision=node_revision
     AND (r.source_kind<>3 OR EXISTS(SELECT 1 FROM game_item_ground_locations ground
      WHERE ground.item_instance_id=r.source_item_instance_id AND ground.channel_id=s.runtime_scope_channel_id
       AND ground.world_id=r.world_id AND ground.runtime_scope_ownership_generation=s.scope_ownership_generation))
     AND g.ready AND g.ownership_generation=runtime.ownership_generation)
  THEN RAISE EXCEPTION 'Dracoyle enchant requires current Character and Node' USING ERRCODE='23514'; END IF;
 END IF;
END $$;
CREATE FUNCTION game_dracoyle_in_place_complete(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;i game_item_instances%ROWTYPE;t game_item_timed_states%ROWTYPE;expected JSONB;w game_item_timed_state_writes%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_tx;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.source_item_instance_id;
 SELECT * INTO w FROM game_item_timed_state_writes WHERE transaction_id=p_tx;
 expected:=r.before_item||jsonb_build_object('definition_production_key',r.effects->'house_after'->>'key','last_transaction_id',p_tx,'state_revision',(r.before_item->>'state_revision')::numeric+1);
 IF game_dracoyle_in_place_record(r) IS DISTINCT FROM TRUE OR r.created_xact_id<>pg_current_xact_id()
  OR to_jsonb(i) IS DISTINCT FROM expected OR game_dracoyle_in_place_custody(r) IS DISTINCT FROM TRUE
  OR t.charges IS NOT NULL OR t.remaining_ms IS DISTINCT FROM 604800000::bigint
  OR t.deadline_at IS DISTINCT FROM r.occurred_at+604800000 OR game_house_furniture_timer_action_complete(w) IS DISTINCT FROM TRUE
 THEN RAISE EXCEPTION 'Dracoyle exact cost, item and clock completion missing' USING ERRCODE='23514'; END IF;
 PERFORM game_item_action_extra_complete(r);
END $$;
CREATE FUNCTION game_dracoyle_in_place_apply(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_tx;
 IF game_dracoyle_in_place_record(r) IS DISTINCT FROM TRUE OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'Dracoyle apply lacks exact current transaction' USING ERRCODE='23514'; END IF;
 UPDATE game_item_instances SET definition_production_key=r.effects->'house_after'->>'key',last_transaction_id=p_tx,state_revision=state_revision+1
  WHERE item_instance_id=r.source_item_instance_id AND to_jsonb(game_item_instances)=r.before_item;
 IF NOT FOUND THEN RAISE EXCEPTION 'Dracoyle predecessor changed' USING ERRCODE='23514'; END IF;
 PERFORM game_house_furniture_timer_apply(p_tx);
END $$;
DO $migration$
DECLARE n TEXT;body TEXT;arm TEXT;begin_at INTEGER;old TEXT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_action_no_consume_shape','game_item_action_source_custody'] LOOP
  SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint WHERE conrelid='game_item_consume_transform_records'::regclass AND conname=n;
  IF body IS NULL OR left(body,7)<>'CHECK (' THEN RAISE EXCEPTION 'Dracoyle source constraint missing: %',n; END IF;
  EXECUTE format('ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT %I',n);
  EXECUTE format('ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT %I CHECK (%s OR (recipe=''oteryn:seasonal.dracoyle'' AND source_kind IN(1,3,4,5) AND effects->>''dracoyle_in_place''=''true'' AND NOT consume_source AND source_count=0 AND outputs=''[]''::jsonb AND jsonb_array_length(extra_inputs)=1 AND secondary_item_instance_id=source_item_instance_id AND ((source_kind IN(1,4))=(parent_item_instance_id IS NOT NULL))))',n,substring(body FROM 8 FOR length(body)-8));
 END LOOP;
 body:=pg_get_functiondef('game_item_action_extra_admit(game_item_consume_transform_records)'::regprocedure);
 old:='IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Dracoyle extra-input admission boundary missing'; END IF;
 EXECUTE replace(body,old,'IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0 AND NOT game_dracoyle_in_place_record(p_record)');
 body:=pg_get_functiondef('game_house_furniture_timer_record(uuid)'::regprocedure);
 old:='(source_kind=5 OR (source_kind IN(1,4) AND (effects->>''kind''=''furniture_rotation'' OR effects->>''portable_use''=''true'')))';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Dracoyle existing timed record owner missing'; END IF;
 EXECUTE replace(body,old,'(source_kind=5 OR (source_kind IN(1,4) AND (effects->>''kind''=''furniture_rotation'' OR effects->>''portable_use''=''true'')) OR (source_kind IN(1,3,4) AND recipe=''oteryn:seasonal.dracoyle'' AND effects->>''dracoyle_in_place''=''true''))');
 FOREACH n IN ARRAY ARRAY['game_item_consume_transform_admit','game_item_consume_transform_complete','game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((n||CASE WHEN n='game_item_consume_transform_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'Dracoyle proof boundary missing: %',n; END IF;
  IF n='game_item_consume_transform_admit' THEN arm:=$arm$
   IF game_dracoyle_in_place_record(NEW) THEN PERFORM game_dracoyle_in_place_admit(NEW);NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;END IF;
$arm$;
  ELSIF n='game_item_consume_transform_complete' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records srv7_furniture_receipt WHERE transaction_id=p_transaction AND game_dracoyle_in_place_record(srv7_furniture_receipt)) THEN PERFORM game_dracoyle_in_place_complete(p_transaction);RETURN;END IF;
$arm$;
  ELSIF n='game_item_instance_guard' THEN arm:=$arm$
   -- The emerald is a separate input. Like the mature extra-input owner,
   -- preserve its predecessor revision while spending exactly one unit.
   IF TG_OP='UPDATE' AND EXISTS(
    SELECT 1 FROM game_item_consume_transform_records srv7_dracoyle_cost,
     LATERAL jsonb_array_elements(srv7_dracoyle_cost.extra_inputs) srv7_dracoyle_extra
    WHERE game_dracoyle_in_place_record(srv7_dracoyle_cost)
     AND srv7_dracoyle_cost.created_xact_id=pg_current_xact_id()
     AND srv7_dracoyle_cost.transaction_id=NEW.last_transaction_id
     AND (srv7_dracoyle_extra->>'item')::uuid=OLD.item_instance_id
     AND srv7_dracoyle_extra->'before'=to_jsonb(OLD)
     AND OLD.definition_family='Item' AND OLD.definition_production_key='oteryn:item.tibia.i677'
     AND NEW.quantity=OLD.quantity-1 AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END
     AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id'])=
      (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])) THEN RETURN NEW; END IF;
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records srv7_furniture_receipt WHERE game_dracoyle_in_place_record(srv7_furniture_receipt)
    AND srv7_furniture_receipt.source_item_instance_id=OLD.item_instance_id AND srv7_furniture_receipt.before_item=to_jsonb(OLD) AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id() AND srv7_furniture_receipt.transaction_id=NEW.last_transaction_id
    AND to_jsonb(NEW)=to_jsonb(OLD)||jsonb_build_object('definition_production_key',srv7_furniture_receipt.effects->'house_after'->>'key','last_transaction_id',srv7_furniture_receipt.transaction_id,'state_revision',OLD.state_revision+1)) THEN RETURN NEW;END IF;
$arm$;
  ELSE arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records srv7_furniture_receipt WHERE game_dracoyle_in_place_record(srv7_furniture_receipt) AND srv7_furniture_receipt.source_item_instance_id=NEW.item_instance_id AND srv7_furniture_receipt.transaction_id=NEW.last_transaction_id AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id()) THEN PERFORM game_dracoyle_in_place_complete(NEW.last_transaction_id);RETURN NULL;END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $access$
DECLARE f regprocedure;
BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
  WHERE n.nspname=current_schema() AND p.proname LIKE 'game_dracoyle_in_place_%' LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
 -- Existing consume/extra-input invoker guards call the closed predicate and
 -- independently validated admission/completion entry points.
 GRANT EXECUTE ON FUNCTION game_dracoyle_in_place_record(game_item_consume_transform_records),
  game_dracoyle_in_place_admit(game_item_consume_transform_records),game_dracoyle_in_place_complete(UUID),
  game_dracoyle_in_place_apply(UUID) TO oteryn_game_runtime;
END $access$;
