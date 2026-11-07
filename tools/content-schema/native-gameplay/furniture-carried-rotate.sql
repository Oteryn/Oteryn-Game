-- Source-captured portable USE subset; carpets and construction kits are excluded.
CREATE TABLE game_house_furniture_portable_use(first_item INTEGER PRIMARY KEY,result_item INTEGER NOT NULL);
INSERT INTO game_house_furniture_portable_use VALUES
__PORTABLE_USE_ROWS__;
REVOKE ALL ON game_house_furniture_portable_use FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_portable_use TO oteryn_game_runtime,oteryn_game_control;
-- Portable native Rotate shares the existing economic Type-2 action receipt.
-- A source pair grants no House authority and never creates House metadata.
CREATE FUNCTION game_item_action_furniture_custody(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE r.source_kind
 WHEN 1 THEN EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
  ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id AND b.world_id=e.world_id
  WHERE e.item_instance_id=r.source_item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id
   AND e.parent_item_instance_id=r.parent_item_instance_id AND e.placement_ordinal=r.source_ordinal AND to_jsonb(e)=r.source_location)
 WHEN 4 THEN game_item_action_custody_current(r)
 ELSE FALSE END
$$;
CREATE FUNCTION game_item_action_furniture_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF (r.effects->>'kind' IS DISTINCT FROM 'furniture_rotation' AND r.effects->>'portable_use' IS DISTINCT FROM 'true') OR r.source_kind NOT IN(1,4)
  OR r.consume_source OR r.source_count<>0 OR r.outputs<>'[]'::jsonb OR r.extra_inputs<>'[]'::jsonb
  OR r.secondary_item_instance_id<>r.source_item_instance_id OR r.before_secondary IS DISTINCT FROM r.before_item
  OR i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM r.before_item OR i.lifecycle<>1 OR i.quantity<>1
  OR i.definition_family<>'Item' OR i.world_id<>r.world_id OR i.state_revision>=18446744073709551615
  OR r.content_digest IS NULL
  OR (r.effects->>'kind'='furniture_rotation' AND (octet_length(r.intent)<6 OR get_byte(r.intent,0)<>255))
  OR (r.effects->>'portable_use'='true' AND (r.effects->>'kind' IS DISTINCT FROM 'transform'
   OR r.recipe IS DISTINCT FROM 'oteryn:novelty-item.furniture-use.'||i.definition_production_key||'.v1'
   OR r.effects ? 'house_absolute' OR r.effects->>'house_rotate'='true'
   OR NOT EXISTS(SELECT 1 FROM game_house_furniture_portable_use c
    WHERE i.definition_production_key='oteryn:item.tibia.i'||c.first_item::text
     AND r.effects->'rotate_after'->>'key'='oteryn:item.tibia.i'||c.result_item::text)))
  OR r.effects->'rotate_after'->>'revision' IS DISTINCT FROM i.definition_revision_ref
  OR NOT EXISTS(SELECT 1 FROM game_house_furniture_rules c WHERE c.action=CASE WHEN r.effects->>'portable_use'='true' THEN 3 ELSE 4 END
   AND i.definition_production_key='oteryn:item.tibia.i'||c.first_item::text
   AND r.effects->'rotate_after'->>'key'='oteryn:item.tibia.i'||c.result_item::text)
  OR game_item_action_furniture_custody(r) IS DISTINCT FROM TRUE
  OR (r.effects ? 'house_temporal' AND (r.effects->'house_after' IS DISTINCT FROM r.effects->'rotate_after' OR (r.effects->>'kind'='furniture_rotation' AND r.effects->>'house_rotate' IS DISTINCT FROM 'true')))
  OR game_house_furniture_timer_proposal(r) IS DISTINCT FROM TRUE
  OR game_house_furniture_absolute_proposal(r) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=i.item_instance_id)
  OR (SELECT count(*) FROM (SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id
    UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id
    UNION ALL SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id) child)
   >coalesce((SELECT capacity FROM game_house_furniture_capacities
    WHERE 'oteryn:item.tibia.i'||item_id::text=r.effects->'rotate_after'->>'key'),0)
  OR game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
  OR game_node_lock_current_registration((r.effects->>'node_id')::uuid,(r.effects->>'node_revision')::numeric) IS DISTINCT FROM TRUE
  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
   JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id AND c.account_id=s.account_id
   JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
   WHERE s.game_session_id=r.game_session_id AND s.character_id=r.character_id AND s.world_id=r.world_id
    AND s.session_state IN(1,2) AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=r.world_id
    AND s.current_generation=(r.audit_context->>'connection_generation')::numeric
    AND s.character_lease_generation=(r.audit_context->>'lease_generation')::numeric
    AND s.scope_ownership_generation=(r.audit_context->>'scope_generation')::numeric
    AND c.eligible AND c.lease_generation=s.character_lease_generation AND c.holder_game_session_id=s.game_session_id
    AND a.presence_character_id=c.character_id AND a.holder_game_session_id=s.game_session_id
    AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments runtime
     JOIN game_durability_admission_runtime_guards g ON g.scope_key=runtime.scope_key
     WHERE runtime.scope_kind=1 AND runtime.world_id=r.world_id AND runtime.channel_id=s.runtime_scope_channel_id
      AND runtime.state=1 AND runtime.ownership_generation=s.scope_ownership_generation
      AND runtime.holder_node_id=(r.effects->>'node_id')::uuid
      AND runtime.holder_registration_revision=(r.effects->>'node_revision')::numeric
      AND g.ready AND g.ownership_generation=runtime.ownership_generation))
 THEN RAISE EXCEPTION 'portable Rotate requires exact native pair, live Character and unchanged carried custody' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_item_action_furniture_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.transaction_id IS NULL OR (r.effects->>'kind' IS DISTINCT FROM 'furniture_rotation' AND r.effects->>'portable_use' IS DISTINCT FROM 'true')
  OR r.created_xact_id<>pg_current_xact_id() OR r.source_kind NOT IN(1,4)
  OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
  OR i.state_revision IS DISTINCT FROM (r.before_item->>'state_revision')::numeric+1
  OR i.definition_production_key IS DISTINCT FROM r.effects->'rotate_after'->>'key'
  OR i.definition_revision_ref IS DISTINCT FROM r.effects->'rotate_after'->>'revision'
  OR (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
   IS DISTINCT FROM (r.before_item-ARRAY['definition_production_key','last_transaction_id','state_revision'])
  OR game_item_action_furniture_custody(r) IS DISTINCT FROM TRUE
  OR ((r.effects ? 'house_absolute') AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
    WHERE p.item_instance_id=r.source_item_instance_id AND to_jsonb(p)=
     (r.effects->'house_absolute'->'before')||jsonb_build_object(
      'definition_production_key',r.effects->'rotate_after'->>'key',
      'target_production_key',r.effects->'house_absolute'->>'target')))
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
    AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
    AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
    AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'portable Rotate changed instance, custody, lineage or exact audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_item_action_furniture_apply(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'portable Rotate requires original receipt transaction' USING ERRCODE='23514'; END IF;
 PERFORM game_item_action_furniture_admit(r);
 UPDATE game_item_instances SET definition_production_key=r.effects->'rotate_after'->>'key',
  state_revision=state_revision+1,last_transaction_id=r.transaction_id WHERE item_instance_id=r.source_item_instance_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'portable Rotate source disappeared' USING ERRCODE='23514'; END IF;
END $$;
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_consume_transform_admit','game_item_consume_transform_complete',
  'game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((n||CASE WHEN n='game_item_consume_transform_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing portable Rotate proof chain: %',n; END IF;
  IF n='game_item_consume_transform_admit' THEN arm:=$arm$
   IF (NEW.effects->>'kind'='furniture_rotation' OR NEW.effects->>'portable_use'='true') THEN
    PERFORM game_item_action_furniture_admit(NEW); NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
   END IF;
  $arm$;
  ELSIF n='game_item_consume_transform_complete' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records WHERE transaction_id=p_transaction AND (effects->>'kind'='furniture_rotation' OR effects->>'portable_use'='true')) THEN
    PERFORM game_item_action_furniture_complete(p_transaction); RETURN;
   END IF;
  $arm$;
  ELSIF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records srv7_furniture_receipt
    WHERE (srv7_furniture_receipt.effects->>'kind'='furniture_rotation' OR srv7_furniture_receipt.effects->>'portable_use'='true') AND srv7_furniture_receipt.source_kind IN(1,4)
     AND srv7_furniture_receipt.source_item_instance_id=OLD.item_instance_id AND srv7_furniture_receipt.before_item=to_jsonb(OLD)
     AND srv7_furniture_receipt.transaction_id=NEW.last_transaction_id AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id()
     AND NEW.definition_production_key=srv7_furniture_receipt.effects->'rotate_after'->>'key') THEN
    IF NEW.state_revision<>OLD.state_revision+1 OR NEW.lifecycle<>1 OR NEW.quantity<>1
     OR (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision']) THEN
     RAISE EXCEPTION 'portable Rotate transforms one instance in place' USING ERRCODE='23514'; END IF;
    RETURN NEW;
   END IF;
  $arm$;
  ELSE arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records srv7_furniture_receipt WHERE (srv7_furniture_receipt.effects->>'kind'='furniture_rotation' OR srv7_furniture_receipt.effects->>'portable_use'='true')
    AND srv7_furniture_receipt.source_item_instance_id=NEW.item_instance_id AND srv7_furniture_receipt.transaction_id=NEW.last_transaction_id
    AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_item_action_furniture_complete(NEW.last_transaction_id); RETURN NULL;
   END IF;
  $arm$;
  END IF;
  EXECUTE substring(body FROM 1 FOR begin_at+4)||E'\n'||arm||substring(body FROM begin_at+5);
 END LOOP;
END $migration$;
DO $access$
DECLARE f regprocedure;
BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_item_action_furniture_custody(game_item_consume_transform_records)'::regprocedure,
  'game_item_action_furniture_admit(game_item_consume_transform_records)'::regprocedure,
  'game_item_action_furniture_complete(uuid)'::regprocedure,
  'game_item_action_furniture_apply(uuid)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
 -- The merged0589 admission/deferred completion chain runs as invoker.
 -- These validated entry points therefore need EXECUTE; custody stays private.
 GRANT EXECUTE ON FUNCTION game_item_action_furniture_admit(game_item_consume_transform_records),
  game_item_action_furniture_complete(UUID),game_item_action_furniture_apply(UUID) TO oteryn_game_runtime;
END $access$;
