-- W18 extends the existing item-loop receipt/line proof chain for the current
-- named House. The item instance and nested custody writers remain unchanged.
-- Crystal src/items/containers/container.cpp:478-566,617-647
-- (Container::queryAdd/queryRemove) preserves exact parent/slot lineage;
-- source cupboard capacity8: data/items/items.xml:65374-65384.
CREATE FUNCTION game_house_container_reachable(p_position BYTEA,p_actor BYTEA)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE a BIGINT[]:='{}';b BIGINT[]:='{}';v BIGINT;off INTEGER;
BEGIN
 IF octet_length(p_position) IS DISTINCT FROM 12 OR octet_length(p_actor) IS DISTINCT FROM 12 THEN RETURN FALSE; END IF;
 FOREACH off IN ARRAY ARRAY[0,4,8] LOOP
  v:=(get_byte(p_position,off)::bigint<<24)+(get_byte(p_position,off+1)::bigint<<16)
    +(get_byte(p_position,off+2)::bigint<<8)+get_byte(p_position,off+3);
  a:=array_append(a,CASE WHEN v>=2147483648 THEN v-4294967296 ELSE v END);
  v:=(get_byte(p_actor,off)::bigint<<24)+(get_byte(p_actor,off+1)::bigint<<16)
    +(get_byte(p_actor,off+2)::bigint<<8)+get_byte(p_actor,off+3);
  b:=array_append(b,CASE WHEN v>=2147483648 THEN v-4294967296 ELSE v END);
 END LOOP;
 RETURN a[3]=b[3] AND abs(a[1]-b[1])<=1 AND abs(a[2]-b[2])<=1;
END $$;

CREATE FUNCTION game_house_container_tree_valid(p_root UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE tree(item,depth,path,cycle) AS (
  SELECT p_root,0,ARRAY[p_root],false
  UNION ALL
  SELECT n.item_instance_id,t.depth+1,t.path||n.item_instance_id,n.item_instance_id=ANY(t.path)
  FROM tree t JOIN game_item_nested_entries n ON n.parent_item_instance_id=t.item
  WHERE t.depth<=8 AND NOT t.cycle
 ), bounded AS (SELECT * FROM tree LIMIT 501)
 SELECT count(*)<=500 AND NOT coalesce(bool_or(depth>8 OR cycle),false) FROM bounded
$$;

CREATE FUNCTION game_house_container_root_admitted(p_item UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_container_roots a
  JOIN game_item_house_interior_locations h USING(item_instance_id,world_id,house_key)
  JOIN game_item_instances i USING(item_instance_id,world_id)
  JOIN game_house_container_loop_proofs p ON p.transaction_id=a.admitted_transaction_id
  JOIN game_item_loop_move_receipts r ON r.transaction_id=p.transaction_id
  WHERE a.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
    =(a.definition_family,a.definition_production_key,a.definition_revision_ref)
   AND p.world_id=a.world_id AND p.house_key=a.house_key AND p.content_digest=a.content_digest
   AND r.world_id=p.world_id AND r.character_id=p.character_id AND r.content_digest=p.content_digest
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=p_item)
   AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=p_item)
   AND (SELECT count(*) FROM game_item_nested_entries n WHERE n.parent_item_instance_id=p_item)<=a.capacity
   AND game_house_container_tree_valid(p_item))
$$;

CREATE FUNCTION game_house_container_loop_prepare(p_transaction UUID,p_session UUID,p_character UUID,
 p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,
 p_world UUID,p_house TEXT,p_position BYTEA,p_source UUID,p_destination UUID,p_capacities JSONB,p_secret BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_loop_move_receipts%ROWTYPE;h game_item_house_interior_locations%ROWTYPE;
 i game_item_instances%ROWTYPE;root_id UUID;cap INTEGER;
BEGIN
 SELECT * INTO r FROM game_item_loop_move_receipts WHERE transaction_id=p_transaction AND created_xact_id=pg_current_xact_id();
 IF r.transaction_id IS NULL OR r.game_session_id IS DISTINCT FROM p_session OR r.character_id IS DISTINCT FROM p_character
  OR r.connection_generation IS DISTINCT FROM p_connection OR r.lease_generation IS DISTINCT FROM p_lease
  OR r.scope_generation IS DISTINCT FROM p_scope OR r.world_id IS DISTINCT FROM p_world OR r.scope_kind<>2
  OR r.channel_id IS NOT NULL OR r.instance_id IS DISTINCT FROM game_house_scope_instance_id(p_world,p_house)
  OR octet_length(p_position) IS DISTINCT FROM 12
  OR game_node_prove_current_incarnation(p_node,p_node_revision,p_secret) IS DISTINCT FROM TRUE
  OR game_hireling_service_access_lock(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house) IS DISTINCT FROM TRUE
  OR game_house_access_tile(p_world,p_house,p_position) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'House container move requires current named-House receipt' USING ERRCODE='23514'; END IF;
 PERFORM game_house_disposition_fence_check(p_world,p_house,NULL,FALSE);
 INSERT INTO game_house_container_loop_proofs VALUES(p_transaction,p_session,p_character,p_connection,p_lease,p_scope,
  p_world,p_house,p_node,p_node_revision,p_position,p_source,p_destination,r.content_digest,pg_current_xact_id());
 FOR root_id IN SELECT DISTINCT x FROM unnest(ARRAY[p_source,p_destination]) x WHERE x IS NOT NULL ORDER BY x LOOP
  SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=root_id FOR UPDATE;
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=root_id FOR UPDATE;
  cap:=(p_capacities->>replace(root_id::text,'-',''))::integer;
  IF h.item_instance_id IS NULL OR i.item_instance_id IS NULL OR h.world_id<>p_world OR h.house_key<>p_house
   OR i.lifecycle<>1 OR i.quantity<>1 OR cap IS NULL OR cap NOT BETWEEN 1 AND 20
   OR game_house_container_reachable(h.spatial_position,p_position) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'House container root/capacity is not current and reachable' USING ERRCODE='23514'; END IF;
  INSERT INTO game_house_container_roots VALUES(root_id,p_world,p_house,i.definition_family,
   i.definition_production_key,i.definition_revision_ref,r.content_digest,cap,p_transaction)
  ON CONFLICT(item_instance_id) DO UPDATE SET world_id=excluded.world_id,house_key=excluded.house_key,
   definition_family=excluded.definition_family,definition_production_key=excluded.definition_production_key,
   definition_revision_ref=excluded.definition_revision_ref,content_digest=excluded.content_digest,
   capacity=excluded.capacity,admitted_transaction_id=excluded.admitted_transaction_id;
 END LOOP;
END $$;

CREATE TABLE game_house_container_line_sources (
 transaction_id UUID NOT NULL,
 line_ordinal SMALLINT NOT NULL,
 location JSONB,
 PRIMARY KEY(transaction_id,line_ordinal),
 FOREIGN KEY(transaction_id,line_ordinal) REFERENCES game_item_loop_move_lines(transaction_id,line_ordinal)
  DEFERRABLE INITIALLY DEFERRED
);
REVOKE ALL ON game_house_container_line_sources FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_container_line_sources TO oteryn_game_runtime,oteryn_game_control;

-- Accepted HOUSE-CUSTODY-0 §2 (lines59-62) requires each item entering a
-- House to retain its own Character reclaim subject, including bag descendants.
-- HOUSE-RUNTIME-0 §5.2 (lines208-213) preserves that subject across House moves.
CREATE TABLE game_house_nested_reclaim_provenance (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED,
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 reclaim_subject_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 placed_transaction_id UUID NOT NULL REFERENCES game_item_loop_move_receipts(transaction_id)
  DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_house_nested_reclaim_subject ON game_house_nested_reclaim_provenance(reclaim_subject_character_id);
CREATE INDEX game_house_nested_reclaim_property ON game_house_nested_reclaim_provenance(world_id,house_key);
REVOKE ALL ON game_house_nested_reclaim_provenance FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_nested_reclaim_provenance TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER a_house_nested_disposition_provenance_fence BEFORE INSERT OR UPDATE OR DELETE
 ON game_house_nested_reclaim_provenance FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();

CREATE FUNCTION game_house_container_subtree(p_item UUID)
RETURNS TABLE(item UUID) LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE tree(item,depth,path) AS (
  SELECT p_item,0,ARRAY[p_item]
  UNION ALL SELECT n.item_instance_id,t.depth+1,t.path||n.item_instance_id
  FROM tree t JOIN game_item_nested_entries n ON n.parent_item_instance_id=t.item
  WHERE t.depth<8 AND NOT n.item_instance_id=ANY(t.path)
 ) SELECT item FROM tree LIMIT 501
$$;

CREATE FUNCTION game_house_container_line_admit(l game_item_loop_move_lines)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_container_loop_proofs%ROWTYPE;r game_item_loop_move_receipts%ROWTYPE;
 i game_item_instances%ROWTYPE;h RECORD;owner UUID;dep INTEGER;loc JSONB;subject UUID;access_role TEXT;
BEGIN
 SELECT * INTO p FROM game_house_container_loop_proofs WHERE transaction_id=l.transaction_id AND created_xact_id=pg_current_xact_id();
 IF p.transaction_id IS NULL OR (p.source_root IS NULL AND p.destination_root IS NULL) THEN RETURN FALSE; END IF;
 SELECT * INTO r FROM game_item_loop_move_receipts WHERE transaction_id=l.transaction_id AND created_xact_id=pg_current_xact_id();
 IF r.transaction_id IS NULL OR l.created_xact_id<>pg_current_xact_id()
  OR game_hireling_service_access_lock(p.game_session_id,p.character_id,p.connection_generation,p.lease_generation,
    p.scope_generation,p.node_id,p.node_revision,p.world_id,p.house_key) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'House container line lacks current receipt' USING ERRCODE='23514'; END IF;
 IF l.from_kind=0 THEN
  IF EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=l.item_instance_id) THEN
   RAISE EXCEPTION 'House container split identity already exists' USING ERRCODE='23514'; END IF;
  IF p.destination_root IS NOT NULL THEN
   SELECT q.reclaim_subject_character_id INTO subject FROM game_house_nested_reclaim_provenance q
    JOIN game_item_loop_move_lines other ON other.item_instance_id=q.item_instance_id
    WHERE other.transaction_id=l.transaction_id AND other.quantity_after<other.quantity_before LIMIT 1;
   INSERT INTO game_house_nested_reclaim_provenance VALUES(l.item_instance_id,p.world_id,p.house_key,
    coalesce(subject,p.character_id),l.transaction_id);
  END IF;
  INSERT INTO game_house_container_line_sources VALUES(l.transaction_id,l.line_ordinal,NULL);
  RETURN TRUE;
 END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=l.item_instance_id;
 IF i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.world_id<>r.world_id OR i.quantity<>l.quantity_before
  OR i.state_revision<>l.state_revision_before
  OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)
   IS DISTINCT FROM (l.definition_family,l.definition_production_key,l.definition_revision_ref)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=l.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=l.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=l.item_instance_id) THEN
  RAISE EXCEPTION 'House container source is not the exact hosted item' USING ERRCODE='23514'; END IF;
 SELECT o.owner_character,o.depth INTO owner,dep FROM game_item_loop_owner(l.item_instance_id) o;
 IF owner=p.character_id AND dep>=1 AND l.from_kind IN(1,2,9) THEN
  IF l.from_kind=1 THEN SELECT to_jsonb(e) INTO loc FROM game_item_container_entries e WHERE item_instance_id=l.item_instance_id AND character_id=p.character_id;
  ELSE SELECT to_jsonb(n) INTO loc FROM game_item_nested_entries n WHERE item_instance_id=l.item_instance_id;
   IF loc IS NULL THEN SELECT to_jsonb(e) INTO loc FROM game_item_container_entries e WHERE item_instance_id=l.item_instance_id AND character_id=p.character_id; END IF;
  END IF;
 ELSE
  SELECT * INTO h FROM game_house_container_root(l.item_instance_id);
  IF h.root IS NULL OR h.world<>p.world_id OR h.house<>p.house_key
   OR (h.root IS DISTINCT FROM p.source_root AND h.root IS DISTINCT FROM p.destination_root)
   OR h.depth<1 OR l.from_kind NOT IN(7,9)
   OR game_house_container_reachable(h.position,p.actor_position) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'House container source is outside current custody' USING ERRCODE='23514'; END IF;
  SELECT to_jsonb(n) INTO loc FROM game_item_nested_entries n WHERE item_instance_id=l.item_instance_id;
 END IF;
 -- Capture the complete existing carried subtree before its one physical move.
 IF owner=p.character_id AND dep>=1 AND p.destination_root IS NOT NULL AND l.to_kind=2
  AND EXISTS(SELECT 1 FROM game_house_container_root(l.to_parent_item_instance_id)) THEN
  INSERT INTO game_house_nested_reclaim_provenance
   SELECT t.item,p.world_id,p.house_key,p.character_id,l.transaction_id
   FROM game_house_container_subtree(l.item_instance_id) t;
 ELSIF EXISTS(SELECT 1 FROM game_house_container_root(l.item_instance_id)) THEN
  IF EXISTS(SELECT 1 FROM game_house_container_subtree(l.item_instance_id) t
   LEFT JOIN game_house_nested_reclaim_provenance q ON q.item_instance_id=t.item
   WHERE q.item_instance_id IS NULL OR q.world_id<>p.world_id OR q.house_key<>p.house_key) THEN
   RAISE EXCEPTION 'House subtree lacks exact per-item reclaim subject' USING ERRCODE='23514'; END IF;
  SELECT role INTO access_role FROM game_house_access(p.world_id,p.house_key,p.character_id);
  IF access_role='GUEST' AND ((l.to_kind IN(1,2) AND NOT EXISTS(
   SELECT 1 FROM game_house_container_root(l.to_parent_item_instance_id)))
   OR (l.quantity_after<l.quantity_before AND p.destination_root IS NULL)) AND EXISTS(
   SELECT 1 FROM game_house_container_subtree(l.item_instance_id) t
    JOIN game_house_nested_reclaim_provenance q ON q.item_instance_id=t.item
    WHERE q.reclaim_subject_character_id<>p.character_id) THEN
   RAISE EXCEPTION 'House guest cannot take another Characters item' USING ERRCODE='23514'; END IF;
 END IF;
 IF loc IS NULL THEN RAISE EXCEPTION 'House container source location missing' USING ERRCODE='23514'; END IF;
 INSERT INTO game_house_container_line_sources VALUES(l.transaction_id,l.line_ordinal,loc);
 RETURN TRUE;
END $$;

CREATE FUNCTION game_house_container_line_proven(l game_item_loop_move_lines)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_container_loop_proofs%ROWTYPE;i game_item_instances%ROWTYPE;
 n game_item_nested_entries%ROWTYPE;e game_item_container_entries%ROWTYPE;h RECORD;
 owner UUID;dep INTEGER;loc JSONB;old_location JSONB;incoming_subject UUID;
BEGIN
 SELECT * INTO p FROM game_house_container_loop_proofs WHERE transaction_id=l.transaction_id AND created_xact_id=pg_current_xact_id();
 IF p.transaction_id IS NULL OR (p.source_root IS NULL AND p.destination_root IS NULL) THEN RETURN FALSE; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=l.item_instance_id;
 IF i.item_instance_id IS NULL OR i.world_id<>p.world_id
  OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)
   IS DISTINCT FROM(l.definition_family,l.definition_production_key,l.definition_revision_ref)
  OR (l.from_kind=0 AND (i.minted_transaction_id<>l.transaction_id OR i.state_revision<>1 OR i.last_transaction_id IS NOT NULL))
  OR (l.from_kind<>0 AND (i.last_transaction_id IS DISTINCT FROM l.transaction_id OR i.state_revision<>l.state_revision_before+1)) THEN
  RAISE EXCEPTION 'House container item lineage differs from exact line' USING ERRCODE='23514'; END IF;
 IF l.quantity_after>l.quantity_before AND l.from_kind<>0 AND EXISTS(
  SELECT 1 FROM game_house_container_root(l.item_instance_id)) THEN
  SELECT q.reclaim_subject_character_id INTO incoming_subject
   FROM game_item_loop_move_lines other JOIN game_house_nested_reclaim_provenance q USING(item_instance_id)
   WHERE other.transaction_id=l.transaction_id AND other.quantity_after<other.quantity_before LIMIT 1;
  IF NOT EXISTS(SELECT 1 FROM game_house_nested_reclaim_provenance q WHERE q.item_instance_id=l.item_instance_id
   AND q.reclaim_subject_character_id=coalesce(incoming_subject,p.character_id)) THEN
   RAISE EXCEPTION 'House stack merge would change a reclaim subject' USING ERRCODE='23514'; END IF;
 END IF;
 IF l.to_kind=0 THEN
  IF i.lifecycle<>2 OR i.quantity<>0 THEN RAISE EXCEPTION 'House container merge did not retire source' USING ERRCODE='23514'; END IF;
  DELETE FROM game_house_nested_reclaim_provenance WHERE item_instance_id=l.item_instance_id;
  RETURN TRUE;
 END IF;
 IF i.lifecycle<>1 OR i.quantity<>l.quantity_after OR i.quantity>100 THEN
  RAISE EXCEPTION 'House container quantity differs from line' USING ERRCODE='23514'; END IF;
 IF l.to_kind=1 THEN
  SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id=l.item_instance_id;
  IF e.item_instance_id IS NULL OR e.character_id<>p.character_id OR e.world_id<>p.world_id
   OR e.parent_item_instance_id<>l.to_parent_item_instance_id OR e.placed_transaction_id<>l.transaction_id
   OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.character_id=p.character_id AND s.item_instance_id=e.parent_item_instance_id)
   OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=e.parent_item_instance_id)>l.to_capacity
   OR game_item_loop_tree_size(p.character_id)>500 THEN
   RAISE EXCEPTION 'House container destination is not current carried backpack' USING ERRCODE='23514'; END IF;
 ELSIF l.to_kind=2 THEN
  SELECT * INTO n FROM game_item_nested_entries WHERE item_instance_id=l.item_instance_id;
  IF n.item_instance_id IS NULL OR n.world_id<>p.world_id OR n.parent_item_instance_id<>l.to_parent_item_instance_id
   OR n.placed_transaction_id<>l.transaction_id OR l.to_capacity NOT BETWEEN 1 AND 20
   OR (SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=n.parent_item_instance_id)>l.to_capacity THEN
   RAISE EXCEPTION 'House container nested destination differs from exact line' USING ERRCODE='23514'; END IF;
  SELECT o.owner_character,o.depth INTO owner,dep FROM game_item_loop_owner(n.parent_item_instance_id) o;
  IF owner=p.character_id AND dep>=1 THEN
   IF game_item_loop_tree_size(p.character_id)>500 THEN RAISE EXCEPTION 'House pickup exceeds carried tree' USING ERRCODE='23514'; END IF;
  ELSE
   SELECT * INTO h FROM game_house_container_root(n.parent_item_instance_id);
   IF h.root IS NULL OR h.root IS DISTINCT FROM p.destination_root OR h.world<>p.world_id OR h.house<>p.house_key
    OR game_house_container_reachable(h.position,p.actor_position) IS DISTINCT FROM TRUE
    OR NOT game_house_container_root_admitted(h.root) THEN
    RAISE EXCEPTION 'House container destination is outside current House graph' USING ERRCODE='23514'; END IF;
  END IF;
 ELSIF l.to_kind=9 THEN
  SELECT location INTO old_location FROM game_house_container_line_sources WHERE transaction_id=l.transaction_id AND line_ordinal=l.line_ordinal;
  SELECT to_jsonb(n) INTO loc FROM game_item_nested_entries n WHERE item_instance_id=l.item_instance_id;
  IF loc IS NULL THEN SELECT to_jsonb(e) INTO loc FROM game_item_container_entries e WHERE item_instance_id=l.item_instance_id; END IF;
  IF loc IS DISTINCT FROM old_location THEN RAISE EXCEPTION 'House container unchanged location moved' USING ERRCODE='23514'; END IF;
 ELSE RAISE EXCEPTION 'unsupported House container destination kind' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_house_container_root(l.item_instance_id)) THEN
  DELETE FROM game_house_nested_reclaim_provenance q WHERE q.item_instance_id IN(
   SELECT item FROM game_house_container_subtree(l.item_instance_id));
 END IF;
 -- Keep the existing 2000-item disposition census executable after adding
 -- individually reclaimable descendants (20 existing batches of100):
 -- existing0178_house_disposition.sql:225-227,246-248.
 IF (SELECT count(*) FROM game_item_house_interior_locations WHERE world_id=p.world_id AND house_key=p.house_key)
  +(SELECT count(*) FROM game_house_nested_reclaim_provenance WHERE world_id=p.world_id AND house_key=p.house_key)>2000 THEN
  RAISE EXCEPTION 'HOUSE_STORAGE_FULL' USING ERRCODE='23514'; END IF;
 IF NOT game_house_container_tree_valid(l.item_instance_id) THEN RAISE EXCEPTION 'House container tree exceeds depth or item bound' USING ERRCODE='23514'; END IF;
 RETURN TRUE;
END $$;

-- Extend, do not replace, every existing loop guard branch.
DO $patch$
DECLARE body TEXT;old TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 IF position('BEGIN' IN body)=0 THEN RAISE EXCEPTION 'missing existing item line admission'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n IF game_house_container_line_admit(NEW) THEN RETURN NEW; END IF;' from position('BEGIN' IN body) for 5);
 body:=pg_get_functiondef('game_item_loop_line_proven()'::regprocedure);
 IF position('BEGIN' IN body)=0 THEN RAISE EXCEPTION 'missing existing item line proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n IF game_house_container_line_proven(NEW) THEN RETURN NULL; END IF;' from position('BEGIN' IN body) for 5);
 body:=pg_get_functiondef('game_item_location_exclusive(UUID)'::regprocedure);
 old:=(regexp_match(body,'RAISE EXCEPTION ''HouseInterior item cannot have contents'' USING ERRCODE\s*=\s*''23514'';'))[1];
 IF old IS NULL THEN RAISE EXCEPTION 'missing existing House contents guard'; END IF;
 EXECUTE replace(body,old,'IF NOT game_house_container_root_admitted(p_item) THEN '||old||' END IF;');
END $patch$;

DO $$
DECLARE f REGPROCEDURE;
BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_container_subtree(UUID)'::regprocedure,
  'game_house_container_reachable(BYTEA,BYTEA)'::regprocedure,
  'game_house_container_tree_valid(UUID)'::regprocedure,
  'game_house_container_root_admitted(UUID)'::regprocedure,
  'game_house_container_loop_prepare(UUID,UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,UUID,UUID,JSONB,BYTEA)'::regprocedure,
  'game_house_container_line_admit(game_item_loop_move_lines)'::regprocedure,
  'game_house_container_line_proven(game_item_loop_move_lines)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION
 game_house_container_loop_prepare(UUID,UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,UUID,UUID,JSONB,BYTEA)
 TO oteryn_game_runtime;

-- Disposition uses its existing census, steps, Inbox deliveries and audit owner.
-- Descendants are delivered before their parents, each to its retained subject;
-- this also preserves the existing prohibition on populated ordinary Inbox roots.
CREATE FUNCTION game_house_disposition_container_sources(w UUID,h TEXT)
RETURNS TABLE(item_instance_id UUID,reclaim_subject_character_id UUID,source_location JSONB,source_provenance JSONB,source_item JSONB)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT l.item_instance_id,p.reclaim_subject_character_id,to_jsonb(l),to_jsonb(p),to_jsonb(i)
 FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
 JOIN game_item_instances i USING(item_instance_id)
 WHERE l.world_id=w AND l.house_key=h
 UNION ALL
 SELECT n.item_instance_id,p.reclaim_subject_character_id,to_jsonb(n),to_jsonb(p),to_jsonb(i)
 FROM game_house_nested_reclaim_provenance p JOIN game_item_nested_entries n USING(item_instance_id)
 JOIN game_item_instances i USING(item_instance_id)
 WHERE p.world_id=w AND p.house_key=h
$$;
CREATE FUNCTION game_house_disposition_descendant_count(op UUID,i UUID)
RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE descendants(item,depth) AS (
  SELECT i,0
  UNION ALL SELECT c.item_instance_id,d.depth+1 FROM descendants d
   JOIN game_house_disposition_census c ON c.operation_id=op
    AND c.source_location->>'parent_item_instance_id'=d.item::text WHERE d.depth<8
 ) SELECT count(*) FROM descendants
$$;
DO $patch$
DECLARE body TEXT;old TEXT;replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_disposition_advance(UUID,INT,INT,INT)'::regprocedure);
 old:='game_item_house_reclaim_provenance';
 -- Recipient locking also includes independently owned descendants. Keep the
 -- original property/census/item/counter lock order and existing 2000 bound.
 replacement:='(SELECT item_instance_id,world_id,house_key,reclaim_subject_character_id FROM game_item_house_reclaim_provenance UNION ALL SELECT item_instance_id,world_id,house_key,reclaim_subject_character_id FROM game_house_nested_reclaim_provenance)';
 body:=replace(body,'FROM game_item_house_reclaim_provenance p', 'FROM '||replacement||' p');
 body:=replace(body,'IF EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE', 'IF EXISTS(SELECT 1 FROM '||replacement||' AS subjects WHERE');
 old:='SELECT item_instance_id FROM game_item_house_interior_locations
   WHERE world_id=o.world_id AND house_key=o.house_key ORDER BY item_instance_id LIMIT 2001';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing disposition census bound'; END IF;
 body:=replace(body,old,'SELECT item_instance_id FROM game_house_disposition_container_sources(o.world_id,o.house_key) ORDER BY item_instance_id LIMIT 2001');
 old:='SELECT op,l.item_instance_id,p.reclaim_subject_character_id,to_jsonb(l),to_jsonb(p),to_jsonb(i)
   FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE l.world_id=o.world_id AND l.house_key=o.house_key ORDER BY l.item_instance_id LIMIT 2000';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing disposition source census'; END IF;
 body:=replace(body,old,'SELECT op,s.item_instance_id,s.reclaim_subject_character_id,s.source_location,s.source_provenance,s.source_item FROM game_house_disposition_container_sources(o.world_id,o.house_key) s ORDER BY s.item_instance_id LIMIT 2000');
 old:='SELECT 1 FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id) WHERE l.item_instance_id=x.item_instance_id
   AND to_jsonb(l)=x.source_location AND to_jsonb(p)=x.source_provenance AND to_jsonb(i)=x.source_item';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing disposition frozen source'; END IF;
 body:=replace(body,old,'SELECT 1 FROM game_house_disposition_container_sources(o.world_id,o.house_key) s WHERE s.item_instance_id=x.item_instance_id AND s.source_location=x.source_location AND s.source_provenance=x.source_provenance AND s.source_item=x.source_item');
 body:=replace(body,'ORDER BY c.item_instance_id LIMIT 100','ORDER BY game_house_disposition_descendant_count(op,c.item_instance_id),c.item_instance_id LIMIT 100');
 old:='DELETE FROM game_item_house_interior_locations WHERE item_instance_id=x.item_instance_id;';
 body:=replace(body,old,'DELETE FROM game_item_nested_entries WHERE item_instance_id=x.item_instance_id; DELETE FROM game_house_nested_reclaim_provenance WHERE item_instance_id=x.item_instance_id; '||old);
 old:='IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=o.world_id AND house_key=o.house_key) THEN';
 body:=replace(body,old,'IF EXISTS(SELECT 1 FROM game_house_disposition_container_sources(o.world_id,o.house_key)) THEN');
 EXECUTE body;
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 IF strpos(body,'BEGIN')=0 THEN RAISE EXCEPTION 'missing nested custody guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n IF TG_OP=''DELETE'' AND game_house_disposition_native_removal_proven(to_jsonb(OLD)) AND NOT EXISTS(SELECT 1 FROM game_house_nested_reclaim_provenance WHERE item_instance_id=OLD.item_instance_id) THEN RETURN NULL; END IF;' FROM strpos(body,'BEGIN') FOR 5);
END $patch$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOREACH f IN ARRAY ARRAY['game_house_disposition_container_sources(UUID,TEXT)'::regprocedure,
  'game_house_disposition_descendant_count(UUID,UUID)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;

-- Crystal native rotation preserves the same container and its contents;
-- e.g. data/items/items.xml:65380 (34278 -> 34276), both capacity8.
-- The existing native planner attests equal source/target capacities. Refresh
-- only definition metadata after its existing exact before-item admission.
DO $patch$
DECLARE body TEXT;old TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_decoration_apply(UUID)'::regprocedure);
 old:='PERFORM game_house_decoration_admit(r);';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing native furniture admission'; END IF;
 EXECUTE replace(body,old,old||$refresh$
 IF r.effects->>'decoration_family'='native_transform' AND EXISTS(
  SELECT 1 FROM game_house_container_roots WHERE item_instance_id=r.source_item_instance_id) THEN
  IF r.consume_source OR (r.effects->>'house_container_capacity')::integer IS NULL
   OR NOT EXISTS(SELECT 1 FROM game_house_container_roots a
    JOIN game_item_house_interior_locations h USING(item_instance_id,world_id,house_key)
    WHERE a.item_instance_id=r.source_item_instance_id AND a.world_id=r.world_id
     AND a.house_key=r.source_location->'house'->>'house_key'
     AND a.definition_family=r.before_item->>'definition_family'
     AND a.definition_production_key=r.before_item->>'definition_production_key'
     AND a.definition_revision_ref=r.before_item->>'definition_revision_ref'
     AND a.capacity=(r.effects->>'house_container_capacity')::integer) THEN
   RAISE EXCEPTION 'container rotation lacks equal-capacity exact root proof' USING ERRCODE='23514'; END IF;
  UPDATE game_house_container_roots SET definition_production_key=r.effects->'house_after'->>'key',
   definition_revision_ref=r.effects->'house_after'->>'revision'
   WHERE item_instance_id=r.source_item_instance_id;
 END IF;
$refresh$);
END $patch$;

-- Retire nested reclaim metadata only after the existing custody/retirement
-- owner has proved its move. Native USE exhaustion must not leave a deleted
-- Character permanently referenced by a no-longer-hosted item.
CREATE FUNCTION game_house_nested_reclaim_reconcile() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_house_nested_reclaim_provenance WHERE item_instance_id=OLD.item_instance_id) THEN RETURN NULL; END IF;
 IF EXISTS(SELECT 1 FROM game_house_container_root(OLD.item_instance_id)) THEN RETURN NULL; END IF;
 IF EXISTS(SELECT 1 FROM game_house_container_subtree(OLD.item_instance_id) t
  JOIN game_house_nested_reclaim_provenance p ON p.item_instance_id=t.item
  JOIN game_item_instances i ON i.item_instance_id=t.item
  WHERE NOT (i.lifecycle=2 AND i.quantity=0)
   AND NOT EXISTS(SELECT 1 FROM game_item_loop_owner(t.item) o WHERE o.owner_character IS NOT NULL)
   AND NOT EXISTS(SELECT 1 FROM game_item_character_inbox_locations l WHERE l.item_instance_id=t.item)) THEN
  RAISE EXCEPTION 'House nested reclaim source lost without existing custody owner' USING ERRCODE='23514'; END IF;
 DELETE FROM game_house_nested_reclaim_provenance WHERE item_instance_id IN(
  SELECT item FROM game_house_container_subtree(OLD.item_instance_id));
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER z_house_nested_reclaim_reconcile AFTER DELETE ON game_item_nested_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_nested_reclaim_reconcile();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_nested_reclaim_reconcile() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_nested_reclaim_reconcile() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_container_disposition_budget() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 -- Existing root creators (ordinary placement and furniture unwrap) share the
 -- same lifecycle budget as nested inserts;0178_house_disposition.sql:225-227.
 IF (SELECT count(*) FROM game_item_house_interior_locations WHERE world_id=NEW.world_id AND house_key=NEW.house_key)
  +(SELECT count(*) FROM game_house_nested_reclaim_provenance WHERE world_id=NEW.world_id AND house_key=NEW.house_key)>2000 THEN
  RAISE EXCEPTION 'HOUSE_STORAGE_FULL' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER z_house_container_disposition_budget AFTER INSERT ON game_item_house_interior_locations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_container_disposition_budget();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_container_disposition_budget() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_container_disposition_budget() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
