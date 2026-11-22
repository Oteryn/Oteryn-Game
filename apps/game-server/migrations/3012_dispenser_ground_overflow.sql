-- A dispenser's carried source and its fresh Ground output have separate custody.
-- Keep the existing source, flask count, timed charge, and audit proof chain.
CREATE FUNCTION game_item_dispenser_ground_matches(
    r game_item_consume_transform_records, g game_item_ground_locations
) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT coalesce(r.recipe='oteryn:dispenser.fill' AND r.source_kind IN (1,4)
   AND g.item_instance_id=(r.outputs->0->>'item')::uuid
   AND g.stack_ordinal=(r.outputs->0->>'ordinal')::bigint
   AND (to_jsonb(g)-ARRAY['item_instance_id','stack_ordinal'])=r.effects->'dispenser_ground',false)
$$;

-- Named House floors have their own location, capacity and reclaim provenance.
CREATE FUNCTION game_item_dispenser_house_overflow_ordinal(r game_item_consume_transform_records)
RETURNS NUMERIC LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB := r.effects->'dispenser'; house TEXT := d->'house_context'->>'house';
 pos BYTEA; roots BIGINT; ordinal NUMERIC;
BEGIN
 IF r.effects->'dispenser_house_overflow' IS DISTINCT FROM 'true'::jsonb
    OR r.effects ? 'dispenser_ground' THEN
  RAISE EXCEPTION 'invalid House dispenser overflow' USING ERRCODE='23514'; END IF;
 PERFORM game_item_dispenser_house_admit(r);
 pos:=int4send((d->'house_context'->'actor_position'->>0)::integer)
   ||int4send((d->'house_context'->'actor_position'->>1)::integer)
   ||int4send((d->'house_context'->'actor_position'->>2)::integer);
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||r.world_id::text||':'||house,0));
 IF game_house_access_tile(r.world_id,house,pos) IS DISTINCT FROM true OR NOT EXISTS(
  SELECT 1 FROM game_house_access(r.world_id,house,r.character_id) a
  WHERE a.role IN ('OWNER','SUBOWNER','GUEST') AND a.content_fenced IS FALSE)
 THEN RETURN NULL; END IF;
 SELECT count(*),coalesce(max(h.stack_ordinal) FILTER (WHERE h.spatial_position=pos),0)+1 INTO roots,ordinal
 FROM game_item_house_interior_locations h WHERE h.world_id=r.world_id AND h.house_key=house;
 IF roots - (CASE WHEN d->>'source_kind'='5' AND d->>'after'='0' THEN 1 ELSE 0 END) >=2000
    OR ordinal>18446744073709551615
 THEN RETURN NULL; END IF;
 RETURN ordinal;
END $$;

CREATE FUNCTION game_item_dispenser_house_overflow_matches(
 r game_item_consume_transform_records, h game_item_house_interior_locations
) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT coalesce(r.recipe='oteryn:dispenser.fill' AND r.source_kind IN (1,4)
   AND r.effects->'dispenser_house_overflow'='true'::jsonb
   AND h.item_instance_id=(r.outputs->0->>'item')::uuid AND h.world_id=r.world_id
   AND h.house_key=r.effects->'dispenser'->'house_context'->>'house'
   AND h.spatial_position=int4send((r.effects->'dispenser'->'house_context'->'actor_position'->>0)::integer)
      ||int4send((r.effects->'dispenser'->'house_context'->'actor_position'->>1)::integer)
      ||int4send((r.effects->'dispenser'->'house_context'->'actor_position'->>2)::integer)
   AND h.stack_ordinal=(r.outputs->0->>'ordinal')::numeric AND h.placed_transaction_id=r.transaction_id,false)
$$;

CREATE FUNCTION game_item_dispenser_place_house_overflow(p_transaction UUID)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records; h game_item_house_interior_locations; at JSONB;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 at:=r.effects->'dispenser'->'house_context'->'actor_position';
 h:=ROW((r.outputs->0->>'item')::uuid,r.world_id,r.effects->'dispenser'->'house_context'->>'house',
    int4send((at->>0)::integer)||int4send((at->>1)::integer)||int4send((at->>2)::integer),
    (r.outputs->0->>'ordinal')::numeric,r.transaction_id);
 IF r.created_xact_id IS DISTINCT FROM pg_current_xact_id()
    OR NOT game_item_dispenser_house_overflow_matches(r,h)
    OR (SELECT count(*) FROM game_item_house_interior_locations WHERE world_id=h.world_id AND house_key=h.house_key)>=2000
 THEN RAISE EXCEPTION 'House dispenser placement requires its exact current fill' USING ERRCODE='23514'; END IF;
 INSERT INTO game_item_house_interior_locations SELECT (h).*;
 INSERT INTO game_item_house_reclaim_provenance
    (item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision)
 VALUES (h.item_instance_id,h.world_id,h.house_key,r.character_id,r.transaction_id,1);
END $$;

CREATE FUNCTION game_item_dispenser_overflow_complete(r game_item_consume_transform_records)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT r.created_xact_id=pg_current_xact_id()
 AND EXISTS(SELECT 1 FROM game_item_consume_transform_records admitted
   WHERE admitted.transaction_id=r.transaction_id AND admitted=r)
 AND CASE WHEN r.effects ? 'dispenser_house_overflow' THEN EXISTS(
   SELECT 1 FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance p USING (item_instance_id)
   WHERE h.item_instance_id=(r.outputs->0->>'item')::uuid AND game_item_dispenser_house_overflow_matches(r,h)
     AND p.reclaim_subject_character_id=r.character_id AND p.placement_transaction_id=r.transaction_id
     AND p.written_xact_id=r.created_xact_id)
 ELSE EXISTS(SELECT 1 FROM game_item_ground_locations g
   WHERE g.item_instance_id=(r.outputs->0->>'item')::uuid AND game_item_dispenser_ground_matches(r,g))
 END
 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e
   WHERE e.item_instance_id=(r.outputs->0->>'item')::uuid)
 AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n
   WHERE n.item_instance_id=(r.outputs->0->>'item')::uuid)
$$;

-- A dispenser uses the authored parent capacity (the existing row-wide 500
-- bound still applies). Preserve every other recipe's capacity constraint.
DO $migration$
DECLARE prior TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
 WHERE conrelid='game_item_consume_transform_records'::regclass
   AND conname='game_item_consume_transform_parent_capacity_check';
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_consume_transform_parent_capacity_check;
 EXECUTE format('ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_consume_transform_parent_capacity_check CHECK ((%s) OR recipe=''oteryn:dispenser.fill'')',prior);
END $migration$;

DO $migration$
DECLARE body TEXT; anchor TEXT; entry INTEGER; signature TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_consume_transform_admit()'::regprocedure);
 entry:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF entry=0 THEN RAISE EXCEPTION 'missing dispenser overflow admission boundary'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
  IF NEW.effects ? 'dispenser_house_overflow' THEN
   IF NEW.source_kind NOT IN (1,4)
    OR (NEW.outputs->0->>'ordinal') IS NULL
    OR game_item_dispenser_house_overflow_ordinal(NEW) IS DISTINCT FROM (NEW.outputs->0->>'ordinal')::numeric
   THEN RAISE EXCEPTION 'dispenser overflow lacks exact House output proof' USING ERRCODE='23514'; END IF;
  ELSIF NEW.effects ? 'dispenser_ground' THEN
   IF NEW.recipe<>'oteryn:dispenser.fill' OR NEW.source_kind NOT IN (1,4)
    OR jsonb_typeof(NEW.effects->'dispenser_ground') IS DISTINCT FROM 'object'
    OR coalesce(NEW.effects->'dispenser'->>'source_kind','') NOT IN ('1','3','4')
    OR (NEW.outputs->0->>'ordinal')::bigint IS DISTINCT FROM currval('game_item_ground_stack_ordinal_seq')
    OR NOT EXISTS (
     SELECT 1 FROM game_durability_reconnect_sessions s,
       jsonb_populate_record(NULL::game_item_ground_locations,NEW.effects->'dispenser_ground') g
     WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
       AND s.session_state IN (1,2) AND s.runtime_scope_kind=1
       AND g.scope_kind=1 AND g.instance_id IS NULL
       AND g.world_id=NEW.world_id AND s.runtime_scope_world_id=g.world_id
       AND s.runtime_scope_channel_id=g.channel_id
       AND s.scope_ownership_generation=g.runtime_scope_ownership_generation
       AND g.runtime_scope_ownership_generation=(NEW.audit_context->>'scope_generation')::numeric
       AND s.current_generation=(NEW.audit_context->>'connection_generation')::numeric
       AND s.character_lease_generation=(NEW.audit_context->>'lease_generation')::numeric
       AND g.content_revision='sha256:'||encode(NEW.content_digest,'hex')
       AND octet_length(g.spatial_position)=10
       AND g.corpse_ref=convert_to('oteryn:dispenser.overflow','UTF8'))
   THEN RAISE EXCEPTION 'dispenser overflow lacks exact Channel output proof' USING ERRCODE='23514'; END IF;
  END IF;
 $arm$ from entry for 5);
 body:=pg_get_functiondef('game_item_consume_transform_admit()'::regprocedure);
 anchor:='OR ((o->>''ordinal'')::numeric<=';
 IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing carried output ordinal boundary'; END IF;
 EXECUTE replace(body,anchor,'OR (NOT (NEW.effects ?| array[''dispenser_ground'',''dispenser_house_overflow'']) AND (o->>''ordinal'')::numeric<=');

 body:=pg_get_functiondef('game_item_action_extended_admit(game_item_consume_transform_records)'::regprocedure);
 anchor:='OR (r.source_kind=4 AND ((o->>''ordinal'')::numeric<=highest';
 IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing nested output ordinal boundary'; END IF;
 EXECUTE replace(body,anchor,'OR (r.source_kind=4 AND NOT (r.effects ?| array[''dispenser_ground'',''dispenser_house_overflow'']) AND ((o->>''ordinal'')::numeric<=highest');

 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 anchor:='FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)';
 IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing carried output completion join'; END IF;
 body:=replace(body,anchor,'FROM game_item_instances n LEFT JOIN game_item_container_entries e USING(item_instance_id)');
 anchor:='AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
       AND e.placement_ordinal=(o->>''ordinal'')::numeric AND e.placed_transaction_id=r.transaction_id)';
 IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing carried output custody proof'; END IF;
 EXECUTE replace(body,anchor,$arm$AND CASE WHEN r.effects ?| array['dispenser_ground','dispenser_house_overflow']
       THEN game_item_dispenser_overflow_complete(r)
       ELSE e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
        AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id END)$arm$);

 body:=pg_get_functiondef('game_item_action_extended_complete(game_item_consume_transform_records)'::regprocedure);
 anchor:='OR (r.source_kind=4 AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries';
 IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing nested output custody proof'; END IF;
 EXECUTE replace(body,anchor,$arm$OR (r.source_kind=4 AND r.effects ?| array['dispenser_ground','dispenser_house_overflow'] AND NOT game_item_dispenser_overflow_complete(r))
    OR (r.source_kind=4 AND NOT (r.effects ?| array['dispenser_ground','dispenser_house_overflow']) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries$arm$);

 body:=pg_get_functiondef('game_item_ground_stack_stamp()'::regprocedure);
 entry:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF entry=0 THEN RAISE EXCEPTION 'missing dispenser overflow stack stamp boundary'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
  IF NEW.stack_ordinal IS NOT NULL AND EXISTS(
   SELECT 1 FROM game_item_instances i JOIN game_item_consume_transform_records r
    ON r.transaction_id=i.minted_transaction_id
   WHERE i.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
    AND game_item_dispenser_ground_matches(r,NEW))
   AND NEW.stack_ordinal=currval('game_item_ground_stack_ordinal_seq')
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE stack_ordinal=NEW.stack_ordinal)
  THEN RETURN NEW; END IF;
 $arm$ from entry for 5);

 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 entry:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF entry=0 THEN RAISE EXCEPTION 'missing dispenser overflow Ground custody boundary'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
  IF EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_consume_transform_records r
    ON r.transaction_id=i.minted_transaction_id
   WHERE i.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
    AND game_item_dispenser_ground_matches(r,NEW)) THEN RETURN NULL; END IF;
 $arm$ from entry for 5);

 body:=pg_get_functiondef('game_item_action_ground_source(uuid)'::regprocedure);
 anchor:='r.source_kind=3 AND (o->>''item'')::uuid=p_item';
 IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing dispenser overflow pickup boundary'; END IF;
 EXECUTE replace(body,anchor,$arm$(r.source_kind=3 OR (r.recipe='oteryn:dispenser.fill' AND r.effects ? 'dispenser_ground'))
    AND (o->>'item')::uuid=p_item$arm$);

 body:=pg_get_functiondef('game_house_location_proven()'::regprocedure);
 entry:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF entry=0 THEN RAISE EXCEPTION 'missing dispenser overflow House custody boundary'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
  IF TG_OP='INSERT' AND EXISTS(
   SELECT 1 FROM game_item_consume_transform_records r WHERE r.transaction_id=NEW.placed_transaction_id
    AND r.created_xact_id=pg_current_xact_id() AND game_item_dispenser_house_overflow_matches(r,NEW))
  THEN RETURN NULL; END IF;
 $arm$ from entry for 5);

 FOREACH signature IN ARRAY ARRAY[
  'game_item_dispenser_ground_matches(game_item_consume_transform_records,game_item_ground_locations)',
  'game_item_dispenser_house_overflow_matches(game_item_consume_transform_records,game_item_house_interior_locations)',
  'game_item_dispenser_house_overflow_ordinal(game_item_consume_transform_records)',
  'game_item_dispenser_place_house_overflow(uuid)',
  'game_item_dispenser_overflow_complete(game_item_consume_transform_records)'
 ] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_control',signature);
 END LOOP;
END $migration$;
