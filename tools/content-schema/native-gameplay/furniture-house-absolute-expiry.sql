-- The Channel absolute Item clock already exists. A named-House resident needs
-- its current House fence when settling that same schedule; no new clock store.
CREATE TABLE game_house_furniture_absolute_expiries(
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_spell_item_temporal_schedules(item_instance_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 context JSONB NOT NULL,before_item JSONB NOT NULL,after_item JSONB NOT NULL,
 schedule JSONB NOT NULL,location JSONB NOT NULL,provenance JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_house_furniture_absolute_expiries FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_absolute_expiries TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_house_furniture_absolute_expiries_immutable BEFORE UPDATE OR DELETE ON game_house_furniture_absolute_expiries
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_house_furniture_absolute_expiries_no_truncate BEFORE TRUNCATE ON game_house_furniture_absolute_expiries
 EXECUTE FUNCTION game_item_reject_truncate();
CREATE FUNCTION game_house_furniture_absolute_expiry_complete(p_tx UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_absolute_expiries%ROWTYPE; p game_spell_item_temporal_schedules%ROWTYPE;
 expected JSONB; k TEXT;
BEGIN
 SELECT * INTO r FROM game_house_furniture_absolute_expiries WHERE transaction_id=p_tx;
 IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id() THEN RETURN FALSE; END IF;
 PERFORM game_house_podium_fence((r.context->>'session')::uuid,r.character_id,
  (r.context->>'connection')::numeric,(r.context->>'lease')::numeric,(r.context->>'scope')::numeric,
  (r.context->>'node')::uuid,(r.context->>'node_revision')::numeric,(r.context->>'world')::uuid,r.context->>'house');
 SELECT * INTO p FROM game_spell_item_temporal_schedules WHERE item_instance_id=r.item_instance_id;
 IF NOT FOUND OR to_jsonb(p)<>r.schedule OR p.expires_at_unix_ms>(r.context->>'settled_at')::bigint
  OR p.definition_production_key NOT IN('oteryn:item.tibia.i9034','oteryn:item.tibia.i9039',
   'oteryn:item.tibia.i35949','oteryn:item.tibia.i35951','oteryn:item.tibia.i35969','oteryn:item.tibia.i35971')
  OR p.target_family IS DISTINCT FROM 'Item' OR p.target_production_key IS NULL
  OR (p.definition_family,p.definition_production_key,p.definition_revision) IS DISTINCT FROM
   (r.before_item->>'definition_family',r.before_item->>'definition_production_key',r.before_item->>'definition_revision_ref')
  OR r.before_item->>'lifecycle'<>'1' OR r.before_item->>'quantity'<>'1'
  OR r.before_item->>'world_id' IS DISTINCT FROM r.context->>'world'
  OR EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=r.item_instance_id)
 THEN RETURN FALSE; END IF;
 expected:=r.before_item||jsonb_build_object('definition_family',p.target_family,
  'definition_production_key',p.target_production_key,'definition_revision_ref',p.target_revision,
  'last_transaction_id',r.transaction_id,'state_revision',(r.before_item->>'state_revision')::numeric+1);
 IF r.after_item<>expected OR NOT EXISTS(SELECT 1 FROM game_item_instances i
  WHERE i.item_instance_id=r.item_instance_id AND to_jsonb(i)=r.after_item) THEN RETURN FALSE; END IF;
 k:=r.location->>'custody_kind';
 RETURN CASE k
 WHEN 'house' THEN EXISTS(SELECT 1 FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance provenance USING(item_instance_id)
  WHERE h.item_instance_id=r.item_instance_id AND to_jsonb(h)=r.location->'custody' AND to_jsonb(provenance)=r.provenance)
 WHEN 'container' THEN EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=r.item_instance_id
  AND to_jsonb(e)=r.location->'custody' AND e.character_id=r.character_id)
 WHEN 'nested' THEN EXISTS(SELECT 1 FROM game_item_nested_entries e,LATERAL game_item_loop_owner(e.parent_item_instance_id) owner
  WHERE e.item_instance_id=r.item_instance_id AND to_jsonb(e)=r.location->'custody'
   AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)
 WHEN 'equipment' THEN EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=r.item_instance_id
  AND to_jsonb(e)=r.location->'custody' AND e.character_id=r.character_id)
 ELSE FALSE END;
END $$;
CREATE FUNCTION game_house_furniture_absolute_expiry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_house_furniture_absolute_expiry_complete(NEW.transaction_id) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'House absolute furniture expiry lacks current fence and exact successor' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_furniture_absolute_expiry_proven AFTER INSERT ON game_house_furniture_absolute_expiries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_furniture_absolute_expiry_proven();
CREATE FUNCTION game_house_furniture_absolute_expire(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
 RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE candidate RECORD; src RECORD; p game_spell_item_temporal_schedules%ROWTYPE;
 total INTEGER:=0; tx UUID; settled BIGINT; after_item JSONB;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 settled:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 FOR candidate IN SELECT i.item_instance_id FROM game_item_instances i JOIN game_spell_item_temporal_schedules schedule USING(item_instance_id)
  LEFT JOIN game_item_house_interior_locations h USING(item_instance_id)
  LEFT JOIN LATERAL game_item_loop_owner(i.item_instance_id) owner ON TRUE
  WHERE i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1
   AND i.definition_production_key IN('oteryn:item.tibia.i9034','oteryn:item.tibia.i9039',
    'oteryn:item.tibia.i35949','oteryn:item.tibia.i35951','oteryn:item.tibia.i35969','oteryn:item.tibia.i35971')
   AND schedule.expires_at_unix_ms<=settled
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
    (schedule.definition_family,schedule.definition_production_key,schedule.definition_revision)
   AND ((h.world_id=p_world AND h.house_key=p_house) OR owner.owner_character=p_character)
   AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries WHERE item_instance_id=i.item_instance_id)
  ORDER BY i.item_instance_id LIMIT 16 FOR UPDATE OF i,schedule LOOP
  SELECT * INTO src FROM game_item_action_house_source(p_session,p_character,p_connection,p_lease,p_scope,
   p_node,p_node_revision,p_world,p_house,candidate.item_instance_id);
  IF NOT FOUND THEN CONTINUE; END IF;
  SELECT * INTO p FROM game_spell_item_temporal_schedules WHERE item_instance_id=candidate.item_instance_id FOR UPDATE;
  IF p.target_family IS DISTINCT FROM 'Item' OR p.target_production_key IS NULL THEN CONTINUE; END IF;
  tx:=game_character_uuid_v7();
  after_item:=src.before_item||jsonb_build_object('definition_family',p.target_family,
   'definition_production_key',p.target_production_key,'definition_revision_ref',p.target_revision,
   'last_transaction_id',tx,'state_revision',(src.before_item->>'state_revision')::numeric+1);
  INSERT INTO game_house_furniture_absolute_expiries(transaction_id,item_instance_id,character_id,context,
   before_item,after_item,schedule,location,provenance)
  VALUES(tx,candidate.item_instance_id,p_character,jsonb_build_object('session',p_session,'connection',p_connection,
   'lease',p_lease,'scope',p_scope,'node',p_node,'node_revision',p_node_revision,'world',p_world,
   'house',p_house,'settled_at',settled),src.before_item,after_item,to_jsonb(p),src.location,src.provenance);
  UPDATE game_item_instances SET definition_family=p.target_family,definition_production_key=p.target_production_key,
   definition_revision_ref=p.target_revision,last_transaction_id=tx,state_revision=state_revision+1
   WHERE item_instance_id=candidate.item_instance_id;
  total:=total+1;
 END LOOP;
 RETURN total;
END $$;
DO $migration$ DECLARE name TEXT; body TEXT; arm TEXT; begin_at INTEGER; BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing House absolute item guard'; END IF;
  IF name='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries srv7_furniture_receipt
    WHERE srv7_furniture_receipt.transaction_id=NEW.last_transaction_id AND srv7_furniture_receipt.before_item=to_jsonb(OLD)
     AND srv7_furniture_receipt.after_item=to_jsonb(NEW) AND srv7_furniture_receipt.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;
  $arm$;
  ELSE arm:=$arm$
   IF game_house_furniture_absolute_expiry_complete(NEW.last_transaction_id) THEN RETURN NULL; END IF;
  $arm$;
  END IF;
  EXECUTE substring(body FROM 1 FOR begin_at+4)||E'\n'||arm||substring(body FROM begin_at+5);
 END LOOP;
END $migration$;
ALTER FUNCTION game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 RENAME TO game_house_furniture_destroy_expire_before_srv7_absolute;
CREATE FUNCTION game_house_furniture_destroy_expire(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
 RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 RETURN game_house_furniture_destroy_expire_before_srv7_absolute(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house)+game_house_furniture_absolute_expire(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house);
END $$;
DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY['game_house_furniture_absolute_expiry_complete(uuid)'::regprocedure,
  'game_house_furniture_absolute_expiry_proven()'::regprocedure,
  'game_house_furniture_absolute_expire(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)'::regprocedure,
  'game_house_furniture_destroy_expire(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_house_furniture_destroy_expire(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text) TO oteryn_game_runtime;
