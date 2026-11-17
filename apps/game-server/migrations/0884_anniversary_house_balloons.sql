-- W19e: ordinary USE on actual HouseInterior balloons. The source
-- instance, placement ordinal and reclaim provenance stay unchanged. This is
-- deliberately not the owner/subowner furniture-WRAP custody operation.
ALTER TABLE game_item_consume_transform_records
 DROP CONSTRAINT game_item_consume_transform_records_source_kind_check,
 ADD CONSTRAINT game_item_consume_transform_records_source_kind_check CHECK(source_kind BETWEEN 1 AND 5);
DO $migration$
DECLARE definition TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing item-action custody shape'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('
  ||substring(definition FROM 7)||' OR (source_kind=5 AND parent_item_instance_id IS NULL AND NOT consume_source'
  ||' AND source_count=0 AND outputs=''[]''::jsonb AND extra_inputs=''[]''::jsonb'
  ||' AND secondary_item_instance_id=source_item_instance_id AND effects->>''kind''=''house_transform''))';
END $migration$;

-- The closed 76 source pairs are generated from anniversary_balloons.lua:1-78.
CREATE FUNCTION game_item_action_house_pair(p_before TEXT,p_after TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM (VALUES
 (37414,37471),
 (37416,37472),
 (37417,37491),
 (37418,37496),
 (37419,37497),
 (37420,37498),
 (37421,37499),
 (37422,37500),
 (37423,37501),
 (37424,37502),
 (37425,37503),
 (37426,37504),
 (37427,37505),
 (37428,37506),
 (37429,37507),
 (37430,37508),
 (37431,37509),
 (37432,37510),
 (37433,37511),
 (37434,37512),
 (37435,37513),
 (37436,37514),
 (37437,37515),
 (37438,37516),
 (37439,37517),
 (37440,37518),
 (37471,37414),
 (37472,37416),
 (37491,37417),
 (37496,37418),
 (37497,37419),
 (37498,37420),
 (37499,37421),
 (37500,37422),
 (37501,37423),
 (37502,37424),
 (37503,37425),
 (37504,37426),
 (37505,37427),
 (37506,37428),
 (37507,37429),
 (37508,37430),
 (37509,37431),
 (37510,37432),
 (37511,37433),
 (37512,37434),
 (37513,37435),
 (37514,37436),
 (37515,37437),
 (37516,37438),
 (37517,37439),
 (37518,37440),
 (39669,39681),
 (39670,39682),
 (39671,39683),
 (39672,39684),
 (39673,39685),
 (39674,39686),
 (39675,39687),
 (39676,39688),
 (39677,39689),
 (39678,39690),
 (39679,39691),
 (39680,39692),
 (39681,39669),
 (39682,39670),
 (39683,39671),
 (39684,39672),
 (39685,39673),
 (39686,39674),
 (39687,39675),
 (39688,39676),
 (39689,39677),
 (39690,39678),
 (39691,39679),
 (39692,39680)
 ) pairs(before_id,after_id) WHERE p_before='oteryn:item.tibia.i'||before_id::text
  AND p_after='oteryn:item.tibia.i'||after_id::text)
$$;

CREATE FUNCTION game_item_action_house_current(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 RETURN TRUE;
EXCEPTION WHEN raise_exception THEN
 IF SQLERRM='PODIUM_STALE_SESSION' THEN RETURN FALSE; END IF;
 RAISE;
END $$;

CREATE FUNCTION game_item_action_house_source(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID)
RETURNS TABLE(before_item JSONB,location JSONB,provenance JSONB,spatial_position BYTEA,stack_ordinal TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 -- Match the established Character -> House -> placement -> Item lock order.
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR SHARE;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 RETURN QUERY SELECT to_jsonb(i),to_jsonb(h),to_jsonb(p),h.spatial_position,h.stack_ordinal::text
  FROM game_item_instances i JOIN game_item_house_interior_locations h USING(item_instance_id,world_id)
  JOIN game_item_house_reclaim_provenance p ON p.item_instance_id=h.item_instance_id
   AND p.world_id=h.world_id AND p.house_key=h.house_key AND p.placement_transaction_id=h.placed_transaction_id
  WHERE i.item_instance_id=p_item AND i.world_id=p_world AND h.house_key=p_house
   AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
   AND game_house_access_tile(h.world_id,h.house_key,h.spatial_position) IS TRUE
  FOR UPDATE OF i,h,p;
END $$;

CREATE FUNCTION game_item_action_house_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; h game_item_house_interior_locations%ROWTYPE;
 p game_item_house_reclaim_provenance%ROWTYPE;
BEGIN
 PERFORM game_house_podium_fence(r.game_session_id,r.character_id,
  (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
  (r.audit_context->>'scope_generation')::numeric,(r.source_location->>'node')::uuid,
  (r.source_location->>'node_revision')::numeric,r.world_id,r.source_location->'house'->>'house_key');
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 SELECT * INTO p FROM game_item_house_reclaim_provenance WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF r.source_kind<>5 OR r.consume_source OR r.source_count<>0 OR r.outputs<>'[]'::jsonb OR r.extra_inputs<>'[]'::jsonb
  OR r.parent_item_instance_id IS NOT NULL OR r.secondary_item_instance_id<>r.source_item_instance_id
  OR i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM r.before_item
  OR r.before_secondary IS DISTINCT FROM r.before_item OR i.lifecycle<>1 OR i.quantity<>1
  OR i.state_revision>=18446744073709551615 OR i.world_id<>r.world_id OR i.definition_family<>'Item'
  OR r.content_digest IS NULL OR r.effects->>'kind' IS DISTINCT FROM 'house_transform'
  OR game_item_action_house_pair(i.definition_production_key,r.effects->'house_after'->>'key') IS DISTINCT FROM TRUE
  OR r.effects->'house_after'->>'revision' IS DISTINCT FROM i.definition_revision_ref
  OR to_jsonb(h) IS DISTINCT FROM r.source_location->'house' OR h.world_id<>r.world_id
  OR h.stack_ordinal<>r.source_ordinal OR to_jsonb(p) IS DISTINCT FROM r.source_location->'provenance'
  OR game_house_access_tile(h.world_id,h.house_key,h.spatial_position) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'House item action requires exact source pair, named House and unchanged plain custody' USING ERRCODE='23514'; END IF;
END $$;

CREATE FUNCTION game_item_action_house_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.transaction_id IS NULL OR r.source_kind<>5 OR r.created_xact_id<>pg_current_xact_id()
  OR i.item_instance_id IS NULL OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
  OR i.state_revision IS DISTINCT FROM (r.before_item->>'state_revision')::numeric+1
  OR i.definition_production_key IS DISTINCT FROM r.effects->'house_after'->>'key'
  OR i.definition_revision_ref IS DISTINCT FROM r.effects->'house_after'->>'revision'
  OR (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
     IS DISTINCT FROM (r.before_item-ARRAY['definition_production_key','last_transaction_id','state_revision'])
  OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=i.item_instance_id
    AND to_jsonb(h)=r.source_location->'house')
  OR NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=i.item_instance_id
    AND to_jsonb(p)=r.source_location->'provenance')
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
    AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
    AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
    AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'House item action changed custody, lineage or lacks exact audit' USING ERRCODE='23514'; END IF;
END $$;

CREATE FUNCTION game_item_action_house_apply(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.source_kind<>5 OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'House item action requires original receipt transaction' USING ERRCODE='23514'; END IF;
 PERFORM game_item_action_house_admit(r);
 UPDATE game_item_instances SET definition_production_key=r.effects->'house_after'->>'key',
   state_revision=state_revision+1,last_transaction_id=r.transaction_id
  WHERE item_instance_id=r.source_item_instance_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'House item action source disappeared' USING ERRCODE='23514'; END IF;
END $$;

-- Extend, never replace, each existing guarded writer chain.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_consume_transform_admit','game_item_consume_transform_complete',
  'game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((n||CASE WHEN n='game_item_consume_transform_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing House item action guard: %',n; END IF;
  IF n='game_item_consume_transform_admit' THEN
   arm:=$arm$
    IF NEW.source_kind=5 THEN PERFORM game_item_action_house_admit(NEW);
     NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END IF;
   $arm$;
  ELSIF n='game_item_consume_transform_complete' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_consume_transform_records WHERE transaction_id=p_transaction AND source_kind=5) THEN
     PERFORM game_item_action_house_complete(p_transaction); RETURN; END IF;
   $arm$;
  ELSIF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r
     WHERE r.source_kind=5 AND r.source_item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
      AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
      AND NEW.definition_production_key=r.effects->'house_after'->>'key') THEN
     IF NEW.state_revision<>OLD.state_revision+1 OR NEW.lifecycle<>1 OR NEW.quantity<>1
      OR (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision']) THEN
      RAISE EXCEPTION 'House item action transforms one instance in place' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
   $arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE r.source_kind=5
     AND r.source_item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
     AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_action_house_complete(NEW.last_transaction_id); RETURN NULL; END IF;
   $arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;

DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_item_action_house_pair(TEXT,TEXT)',
  'game_item_action_house_current(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)',
  'game_item_action_house_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)',
  'game_item_action_house_admit(game_item_consume_transform_records)',
  'game_item_action_house_complete(UUID)','game_item_action_house_apply(UUID)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE 'REVOKE ALL ON FUNCTION '||signature||' FROM PUBLIC,oteryn_game_runtime,oteryn_game_control';
 END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION
 game_item_action_house_current(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT),
 game_item_action_house_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID),
 game_item_action_house_admit(game_item_consume_transform_records),
 game_item_action_house_complete(UUID),game_item_action_house_apply(UUID) TO oteryn_game_runtime;

-- Append after house-item-actions.sql. Crystal anniversary_balloons.lua:88-96
-- resolves item:getPosition through Item::getTile (item.cpp:469-495), including
-- a carried/nested item's current player tile. Preserve each real custody row.
CREATE OR REPLACE FUNCTION game_item_action_house_source(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID)
RETURNS TABLE(before_item JSONB,location JSONB,provenance JSONB,spatial_position BYTEA,stack_ordinal TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; h game_item_house_interior_locations%ROWTYPE;
 p game_item_house_reclaim_provenance%ROWTYPE; e game_item_container_entries%ROWTYPE;
 n game_item_nested_entries%ROWTYPE; s game_character_equipment_slots%ROWTYPE;
 house_identity JSONB:=jsonb_build_object('world_id',p_world,'house_key',p_house);
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR SHARE;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=p_item AND world_id=p_world
  AND lifecycle=1 AND quantity=1 AND definition_family='Item' FOR UPDATE;
 IF NOT FOUND THEN RETURN; END IF;
 SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=p_item
  AND world_id=p_world AND house_key=p_house FOR UPDATE;
 IF FOUND THEN
  SELECT * INTO p FROM game_item_house_reclaim_provenance WHERE item_instance_id=p_item
   AND world_id=p_world AND house_key=p_house AND placement_transaction_id=h.placed_transaction_id FOR UPDATE;
  IF NOT FOUND OR game_house_access_tile(p_world,p_house,h.spatial_position) IS DISTINCT FROM TRUE THEN RETURN; END IF;
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('house',to_jsonb(h),'custody_kind','house','custody',to_jsonb(h)),
   to_jsonb(p),h.spatial_position,h.stack_ordinal::text;
  RETURN;
 END IF;
 SELECT c.* INTO e FROM game_item_container_entries c
  JOIN game_item_container_slots b ON b.character_id=c.character_id AND b.item_instance_id=c.parent_item_instance_id
   AND b.world_id=c.world_id
  WHERE c.item_instance_id=p_item AND c.world_id=p_world AND c.character_id=p_character FOR UPDATE OF c;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('house',house_identity,'custody_kind','container','custody',to_jsonb(e)),
   'null'::jsonb,NULL::bytea,e.placement_ordinal::text;
  RETURN;
 END IF;
 SELECT c.* INTO n FROM game_item_nested_entries c,
  LATERAL game_item_loop_owner(c.parent_item_instance_id) owner
  WHERE c.item_instance_id=p_item AND c.world_id=p_world AND owner.owner_character=p_character
   AND owner.depth BETWEEN 0 AND 7 FOR UPDATE OF c;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('house',house_identity,'custody_kind','nested','custody',to_jsonb(n)),
   'null'::jsonb,NULL::bytea,n.placement_ordinal::text;
  RETURN;
 END IF;
 SELECT * INTO s FROM game_character_equipment_slots WHERE item_instance_id=p_item
  AND world_id=p_world AND character_id=p_character FOR UPDATE;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('house',house_identity,'custody_kind','equipment','custody',to_jsonb(s)),
   'null'::jsonb,NULL::bytea,s.slot::text;
 END IF;
END $$;

CREATE FUNCTION game_item_action_house_custody(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE k TEXT:=coalesce(r.source_location->>'custody_kind','house'); position BYTEA;
BEGIN
 IF r.source_location->'house'->>'world_id' IS DISTINCT FROM r.world_id::text
  OR jsonb_typeof(r.source_location->'actor_position') IS DISTINCT FROM 'array'
  OR jsonb_array_length(r.source_location->'actor_position')<>3 THEN RETURN FALSE; END IF;
 IF k='house' THEN
  RETURN EXISTS(SELECT 1 FROM game_item_house_interior_locations h
   JOIN game_item_house_reclaim_provenance p ON p.item_instance_id=h.item_instance_id
    AND p.world_id=h.world_id AND p.house_key=h.house_key AND p.placement_transaction_id=h.placed_transaction_id
   WHERE h.item_instance_id=r.source_item_instance_id AND h.world_id=r.world_id
    AND h.stack_ordinal=r.source_ordinal AND to_jsonb(h)=r.source_location->'house'
    AND to_jsonb(p)=r.source_location->'provenance'
    AND game_house_access_tile(h.world_id,h.house_key,h.spatial_position) IS TRUE);
 END IF;
 -- A carried item's tile is exactly the current native House actor position.
 position:=int4send((r.source_location->'actor_position'->>0)::integer)
  ||int4send((r.source_location->'actor_position'->>1)::integer)
  ||int4send((r.source_location->'actor_position'->>2)::integer);
 IF r.source_location->'provenance' IS DISTINCT FROM 'null'::jsonb
  OR game_house_access_tile(r.world_id,r.source_location->'house'->>'house_key',position) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=r.source_item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=r.source_item_instance_id)
 THEN RETURN FALSE; END IF;
 RETURN CASE k
  WHEN 'container' THEN EXISTS(SELECT 1 FROM game_item_container_entries c
   JOIN game_item_container_slots b ON b.character_id=c.character_id AND b.item_instance_id=c.parent_item_instance_id
    AND b.world_id=c.world_id
   WHERE c.item_instance_id=r.source_item_instance_id AND c.world_id=r.world_id AND c.character_id=r.character_id
    AND c.placement_ordinal=r.source_ordinal AND to_jsonb(c)=r.source_location->'custody')
  WHEN 'nested' THEN EXISTS(SELECT 1 FROM game_item_nested_entries c,
   LATERAL game_item_loop_owner(c.parent_item_instance_id) owner
   WHERE c.item_instance_id=r.source_item_instance_id AND c.world_id=r.world_id
    AND c.placement_ordinal=r.source_ordinal AND to_jsonb(c)=r.source_location->'custody'
    AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)
  WHEN 'equipment' THEN EXISTS(SELECT 1 FROM game_character_equipment_slots c
   WHERE c.item_instance_id=r.source_item_instance_id AND c.world_id=r.world_id AND c.character_id=r.character_id
    AND c.slot=r.source_ordinal AND to_jsonb(c)=r.source_location->'custody')
  ELSE FALSE END;
END $$;

CREATE OR REPLACE FUNCTION game_item_action_house_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 PERFORM game_house_podium_fence(r.game_session_id,r.character_id,
  (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
  (r.audit_context->>'scope_generation')::numeric,(r.source_location->>'node')::uuid,
  (r.source_location->>'node_revision')::numeric,r.world_id,r.source_location->'house'->>'house_key');
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF r.source_kind<>5 OR r.consume_source OR r.source_count<>0 OR r.outputs<>'[]'::jsonb OR r.extra_inputs<>'[]'::jsonb
  OR r.parent_item_instance_id IS NOT NULL OR r.secondary_item_instance_id<>r.source_item_instance_id
  OR i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM r.before_item
  OR r.before_secondary IS DISTINCT FROM r.before_item OR i.lifecycle<>1 OR i.quantity<>1
  OR i.state_revision>=18446744073709551615 OR i.world_id<>r.world_id OR i.definition_family<>'Item'
  OR r.content_digest IS NULL OR r.effects->>'kind' IS DISTINCT FROM 'house_transform'
  OR game_item_action_house_pair(i.definition_production_key,r.effects->'house_after'->>'key') IS DISTINCT FROM TRUE
  OR r.effects->'house_after'->>'revision' IS DISTINCT FROM i.definition_revision_ref
  OR game_item_action_house_custody(r) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'House item action requires exact source pair and unchanged actor/House custody' USING ERRCODE='23514'; END IF;
END $$;

CREATE OR REPLACE FUNCTION game_item_action_house_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.transaction_id IS NULL OR r.source_kind<>5 OR r.created_xact_id<>pg_current_xact_id()
  OR i.item_instance_id IS NULL OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
  OR i.state_revision IS DISTINCT FROM (r.before_item->>'state_revision')::numeric+1
  OR i.definition_production_key IS DISTINCT FROM r.effects->'house_after'->>'key'
  OR i.definition_revision_ref IS DISTINCT FROM r.effects->'house_after'->>'revision'
  OR (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
     IS DISTINCT FROM (r.before_item-ARRAY['definition_production_key','last_transaction_id','state_revision'])
  OR game_item_action_house_custody(r) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
    AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
    AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
    AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'House item action changed custody, lineage or lacks exact audit' USING ERRCODE='23514'; END IF;
END $$;

DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_item_action_house_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)',
  'game_item_action_house_custody(game_item_consume_transform_records)',
  'game_item_action_house_admit(game_item_consume_transform_records)',
  'game_item_action_house_complete(UUID)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE 'REVOKE ALL ON FUNCTION '||signature||' FROM PUBLIC,oteryn_game_runtime,oteryn_game_control';
 END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION
 game_item_action_house_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID),
 game_item_action_house_admit(game_item_consume_transform_records),game_item_action_house_complete(UUID)
 TO oteryn_game_runtime;


-- Exact Afflicted exception over the existing Ground and nested custody proof.
-- Crystal usable_afflicted_outfit_items.lua:35-49 removes all six pieces from
-- the player's carried tree. Clicking a Ground piece leaves that source intact.
CREATE FUNCTION game_item_cosmetic_ground_pieces(
 p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,p_before JSONB,p_effects JSONB,p_extra JSONB)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(p_kind=3 AND NOT p_consume AND p_count=0
  AND p_before->>'definition_production_key' IN(
   'oteryn:item.tibia.i12551','oteryn:item.tibia.i12552','oteryn:item.tibia.i12553',
   'oteryn:item.tibia.i12554','oteryn:item.tibia.i12555','oteryn:item.tibia.i12556')
  AND p_effects->>'kind'='cosmetic'
  AND p_effects->'cosmetic'->>'consumed'='true'
  AND p_effects->'cosmetic'->>'provenance'='crystal-item-cosmetics-201189f-v1'
  AND p_effects->'cosmetic'->'progress'='null'::jsonb
  AND jsonb_array_length(p_effects->'cosmetic'->'grants')=1
  AND p_effects->'cosmetic'->'grants'->0->>'kind'='outfit'
  AND p_effects->'cosmetic'->'grants'->0->>'grant_key'='oteryn:item-grant.outfit.430-431.base'
  AND (p_effects->'cosmetic'->'grants'->0->>'addon')::integer=0
  AND jsonb_array_length(p_extra)=6
  AND ARRAY(SELECT e->'before'->>'definition_production_key'
       FROM jsonb_array_elements(p_extra) e ORDER BY e->'before'->>'definition_production_key')=
      ARRAY['oteryn:item.tibia.i12551','oteryn:item.tibia.i12552','oteryn:item.tibia.i12553',
       'oteryn:item.tibia.i12554','oteryn:item.tibia.i12555','oteryn:item.tibia.i12556']
  AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_extra) e
       WHERE (e->>'count')::integer IS DISTINCT FROM 1
          OR (e->>'source_kind')::integer NOT IN(1,4)
          OR e->>'source_kind' IS NULL),FALSE)
$$;
DO $migration$
DECLARE definition TEXT; shape TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing item-action custody shape'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 shape:=$shape$
  (source_kind=3 AND parent_item_instance_id IS NULL
   AND secondary_item_instance_id=source_item_instance_id AND source_location ? 'stack_ordinal'
   AND effects->>'kind'='cosmetic' AND jsonb_typeof(effects->'cosmetic')='object'
   AND recipe=('oteryn:item-cosmetic.i'||(effects->'cosmetic'->>'source_item_id')||'.v1')
   AND outputs='[]'::jsonb
   AND (extra_inputs='[]'::jsonb OR game_item_cosmetic_ground_pieces(
      source_kind,consume_source,source_count,before_item,effects,extra_inputs)))
$shape$;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('
  ||substring(definition FROM 7)||' OR coalesce('||shape||',FALSE) OR game_event_planting_ground(source_kind,consume_source,source_count,before_item,effects,extra_inputs,outputs,recipe))';

 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_no_consume_shape';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing item-action no-consume shape'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_no_consume_shape;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_no_consume_shape CHECK ('
  ||substring(definition FROM 7)||' OR game_item_cosmetic_ground_pieces(source_kind,consume_source,source_count,before_item,effects,extra_inputs))';

 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_extra_input_bound';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing bounded item-action extra inputs'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_extra_input_bound;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_extra_input_bound CHECK ('
  ||substring(definition FROM 7)||' OR game_item_cosmetic_ground_pieces(source_kind,consume_source,source_count,before_item,effects,extra_inputs))';
END $migration$;

-- Keep the existing full predecessor, same-character bounded nested custody and
-- deferred successor/audit proofs. Only this exact six-input action may leave its
-- Ground trigger untouched while spending carried inputs.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_action_extra_admit(game_item_consume_transform_records)'::regprocedure);
 original:='IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0 THEN';
 replacement:='IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0
  AND NOT game_item_cosmetic_ground_pieces(p_record.source_kind,p_record.consume_source,p_record.source_count,
   p_record.before_item,p_record.effects,p_record.extra_inputs) THEN';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing strict extra-input admission'; END IF;
 EXECUTE replace(body,original,replacement);

 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 original:='WHERE r.consume_source AND (e->>''item'')::uuid=OLD.item_instance_id AND e->''before''=to_jsonb(OLD)';
 replacement:='WHERE (r.consume_source OR game_item_cosmetic_ground_pieces(r.source_kind,r.consume_source,r.source_count,
   r.before_item,r.effects,r.extra_inputs)) AND (e->>''item'')::uuid=OLD.item_instance_id AND e->''before''=to_jsonb(OLD)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing exact additional input mutation proof'; END IF;
 EXECUTE replace(body,original,replacement);

 body:=pg_get_functiondef('game_item_cosmetic_consistent()'::regprocedure);
 original:='OR (e->>''consumed'')::boolean IS DISTINCT FROM r.consume_source';
 replacement:='OR (e->>''consumed'')::boolean IS DISTINCT FROM (r.consume_source
   OR game_item_cosmetic_ground_pieces(r.source_kind,r.consume_source,r.source_count,r.before_item,r.effects,r.extra_inputs))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing cosmetic consumption proof'; END IF;
 body:=replace(body,original,replacement);
 original:='OR (NOT r.consume_source AND (jsonb_array_length(e->''grants'')<>0 OR p IS DISTINCT FROM ''null''::jsonb))';
 replacement:='OR (NOT r.consume_source AND NOT game_item_cosmetic_ground_pieces(r.source_kind,r.consume_source,r.source_count,
   r.before_item,r.effects,r.extra_inputs) AND (jsonb_array_length(e->''grants'')<>0 OR p IS DISTINCT FROM ''null''::jsonb))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing unchanged cosmetic source grant proof'; END IF;
 EXECUTE replace(body,original,replacement);
 EXECUTE format('ALTER FUNCTION game_item_cosmetic_ground_pieces(integer,boolean,bigint,jsonb,jsonb,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_cosmetic_ground_pieces(integer,boolean,bigint,jsonb,jsonb,jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_cosmetic_ground_pieces(integer,boolean,bigint,jsonb,jsonb,jsonb) TO oteryn_game_runtime;


-- W19b: one carried seed may be spent while the exact14029 plant stays unchanged.
-- This bounded source case reuses all custody, input, audit and achievement proofs.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT; definition TEXT; predicate TEXT;
BEGIN
 predicate:='game_event_seed_preservation(source_kind,consume_source,source_count,before_item,effects,extra_inputs,outputs,recipe)';
 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing preservation source custody'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('
  ||substring(definition FROM 7)||' OR ('||predicate||' AND secondary_item_instance_id=source_item_instance_id'
  ||' AND ((source_kind=3 AND parent_item_instance_id IS NULL AND source_location ? ''stack_ordinal'')'
  ||' OR (source_kind IN(1,4) AND parent_item_instance_id IS NOT NULL))))';
 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_no_consume_shape';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing preservation no-consume shape'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_no_consume_shape;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_no_consume_shape CHECK ('
  ||substring(definition FROM 7)||' OR ('||predicate||' AND secondary_item_instance_id=source_item_instance_id))';

 body:=pg_get_functiondef('game_item_action_extra_admit(game_item_consume_transform_records)'::regprocedure);
 original:='IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0';
 replacement:=original||' AND NOT game_event_seed_preservation(p_record.source_kind,p_record.consume_source,p_record.source_count,'
  ||'p_record.before_item,p_record.effects,p_record.extra_inputs,p_record.outputs,p_record.recipe)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing preservation extra-input admission'; END IF;
 EXECUTE replace(body,original,replacement);

 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 original:='WHERE (r.consume_source OR game_item_cosmetic_ground_pieces(r.source_kind,r.consume_source,r.source_count,';
 replacement:='WHERE (r.consume_source OR game_event_seed_preservation(r.source_kind,r.consume_source,r.source_count,'
  ||'r.before_item,r.effects,r.extra_inputs,r.outputs,r.recipe) OR game_item_cosmetic_ground_pieces(r.source_kind,r.consume_source,r.source_count,';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing preservation exact input mutation'; END IF;
 EXECUTE replace(body,original,replacement);

 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 original:='OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id OR last_transaction_id=r.transaction_id)';
 replacement:='OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)'
  ||' OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>'
  ||' (CASE WHEN game_event_seed_preservation(r.source_kind,r.consume_source,r.source_count,'
  ||'r.before_item,r.effects,r.extra_inputs,r.outputs,r.recipe) THEN 1 ELSE 0 END)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing preserved source completion proof'; END IF;
 EXECUTE replace(body,original,replacement);
END $migration$;
