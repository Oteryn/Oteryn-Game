-- Lazy representation maintenance for a death root picked up before 2806.
-- Current fenced Character custody is authority; old death history only identifies
-- the eligible representation. Named-House actors retain Character-carried custody;
-- physical House custody and immutable history are not rewritten.
CREATE TABLE game_carried_death_reconciliations (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 root_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 death_transaction_id UUID NOT NULL REFERENCES game_player_death_item_records(transaction_id),
 world_id UUID NOT NULL,
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 fence JSONB NOT NULL,
 root_before JSONB NOT NULL,
 placement_before JSONB NOT NULL,
 tree_size SMALLINT NOT NULL CHECK(tree_size BETWEEN 2 AND 500),
 direct_count SMALLINT NOT NULL CHECK(direct_count BETWEEN 1 AND 499),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_carried_death_reconciliations_root ON game_carried_death_reconciliations(root_item_instance_id);
CREATE INDEX game_carried_death_reconciliations_death ON game_carried_death_reconciliations(death_transaction_id);
CREATE INDEX game_carried_death_reconciliations_owner ON game_carried_death_reconciliations(owner_character_id);
CREATE TABLE game_carried_death_edges (
 transaction_id UUID NOT NULL REFERENCES game_carried_death_reconciliations(transaction_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 world_id UUID NOT NULL,
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 placement_ordinal NUMERIC(20,0) NOT NULL,
 placed_transaction_id UUID NOT NULL,
 source_kind SMALLINT NOT NULL CHECK(source_kind IN(1,2)),
 source_character_id UUID,
 item_before JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,item_instance_id),
 UNIQUE(transaction_id,parent_item_instance_id,placement_ordinal),
 CHECK((source_kind=1)=(source_character_id IS NOT NULL))
);
CREATE INDEX game_carried_death_edges_item ON game_carried_death_edges(item_instance_id);
CREATE INDEX game_carried_death_edges_parent ON game_carried_death_edges(parent_item_instance_id);
REVOKE ALL ON game_carried_death_reconciliations,game_carried_death_edges FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT ON game_carried_death_reconciliations,game_carried_death_edges TO oteryn_game_control;

-- Private completion predicate: no actor authority is derived from a journal.
-- The caller's original node possession is proved separately, never persisted.
CREATE FUNCTION game_carried_death_current(f JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql VOLATILE SECURITY DEFINER AS $$
DECLARE
 c UUID:=(f->>'character')::uuid; w UUID:=(f->>'world')::uuid;
 s UUID:=(f->>'session')::uuid; ch UUID:=(f->>'channel')::uuid;
 inst UUID:=(f->>'instance')::uuid; n UUID:=(f->>'node')::uuid;
 house TEXT:=f->>'house';
 kind SMALLINT:=CASE WHEN inst IS NULL THEN 1 ELSE 2 END;
 assignment BYTEA; runtime BYTEA; account UUID;
BEGIN
 IF (ch IS NULL)=(inst IS NULL) OR c IS NULL OR w IS NULL OR s IS NULL
  OR NOT game_node_lock_current_registration(n,(f->>'node_revision')::numeric)
  OR NOT game_runtime_scope_assignment_history_valid() THEN RETURN FALSE; END IF;
 IF house IS NOT NULL AND (kind<>2 OR inst IS DISTINCT FROM game_house_scope_instance_id(w,house)) THEN RETURN FALSE; END IF;
 assignment:=CASE WHEN kind=1 THEN '\x01'::bytea||uuid_send(w)||uuid_send(ch)
  WHEN house IS NOT NULL THEN game_house_scope_key(w,house)
  ELSE '\x03'::bytea||uuid_send(w)||uuid_send(inst) END;
 runtime:=CASE WHEN kind=1 THEN assignment ELSE '\x02'::bytea||uuid_send(w)||uuid_send(inst) END;
 SELECT r.account_id INTO account FROM game_durability_reconnect_sessions r
  WHERE r.game_session_id=s AND r.character_id=c AND r.world_id=w
   AND r.runtime_scope_world_id=w AND r.runtime_scope_kind=kind
   AND r.runtime_scope_channel_id IS NOT DISTINCT FROM ch
   AND r.runtime_scope_instance_id IS NOT DISTINCT FROM inst AND r.runtime_scope_house_key IS NOT DISTINCT FROM house
   AND (house IS NULL OR r.origin_channel_id=(f->>'origin_channel')::uuid)
   AND r.current_generation=(f->>'connection')::numeric
   AND r.character_lease_generation=(f->>'lease')::numeric
   AND r.scope_ownership_generation=(f->>'generation')::numeric AND r.session_state IN(1,2) FOR SHARE;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 IF kind=2 AND house IS NULL AND game_activity_session_origin_valid(s,c) IS DISTINCT FROM TRUE THEN RETURN FALSE; END IF;
 IF house IS NOT NULL THEN
  -- A named House is assignment kind2, never activity kind3 or an origin Channel.
  -- This proves custody only; no House service ACL or House contents are granted.
  PERFORM 1 FROM game_runtime_scope_assignments a WHERE a.scope_key=assignment
   AND a.scope_kind=2 AND a.world_id=w AND a.house_key=house AND a.channel_id IS NULL
   AND a.activity_instance_id IS NULL AND a.state=1
   AND a.ownership_generation=(f->>'generation')::numeric AND a.holder_node_id=n
   AND a.holder_registration_revision=(f->>'node_revision')::numeric FOR SHARE;
 ELSE
 PERFORM 1 FROM game_runtime_scope_assignments a
  JOIN game_durability_admission_runtime_guards g ON g.scope_key=runtime
  WHERE a.scope_key=assignment AND a.world_id=w AND a.state=1
   AND a.scope_kind=CASE WHEN kind=1 THEN 1 ELSE 3 END AND a.house_key IS NULL
   AND a.channel_id IS NOT DISTINCT FROM ch AND a.activity_instance_id IS NOT DISTINCT FROM inst
   AND a.ownership_generation=(f->>'generation')::numeric AND a.holder_node_id=n
   AND a.holder_registration_revision=(f->>'node_revision')::numeric
   AND g.ready AND g.ownership_generation=a.ownership_generation FOR SHARE OF a,g;
 END IF;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 PERFORM 1 FROM game_durability_admission_character_guards cg
  JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id
  WHERE cg.character_id=c AND cg.account_id=account AND cg.world_id=w AND cg.eligible
   AND cg.lease_generation=(f->>'lease')::numeric AND cg.holder_game_session_id=s
   AND ag.presence_character_id=c AND ag.holder_game_session_id=s FOR SHARE OF cg,ag;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 PERFORM 1 FROM game_character_roots r WHERE r.character_id=c AND r.account_id=account
  AND r.world_id=w AND r.lifecycle=1 FOR NO KEY UPDATE;
 RETURN FOUND;
END $$;

CREATE FUNCTION game_carried_death_reconcile(f JSONB,p_secret BYTEA) RETURNS VOID
LANGUAGE plpgsql VOLATILE SECURITY DEFINER AS $$
DECLARE
 c UUID:=(f->>'character')::uuid; w UUID:=(f->>'world')::uuid;
 main UUID; root UUID; death UUID; operation_id UUID; ids UUID[];
 count_nodes BIGINT; direct_nodes BIGINT; distinct_nodes BIGINT; height INT; cycle BOOLEAN;
 before_tree JSONB; after_lock JSONB; root_row JSONB; placement JSONB; changed BIGINT;
BEGIN
 IF game_node_prove_current_incarnation((f->>'node')::uuid,(f->>'node_revision')::numeric,p_secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'carried death reconciliation requires current node possession' USING ERRCODE='23514';
 END IF;
 SELECT item_instance_id INTO main FROM game_item_container_slots WHERE character_id=c AND world_id=w;
 IF NOT FOUND THEN RETURN; END IF;
 -- A bounded indexed hint grants nothing. Ordinary current inventory reads do
 -- not repeat the complete SQL fence when there is no historical layout to write.
 IF NOT EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(main) t WHERE t.depth>0
  AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=t.item_instance_id)
  AND EXISTS(SELECT 1 FROM game_player_death_item_records d WHERE d.item_instance_id=t.item_instance_id AND d.operation=1)) THEN RETURN; END IF;
 IF game_carried_death_current(f) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'carried death reconciliation requires current actor authority' USING ERRCODE='23514';
 END IF;
 -- No historical table scan or global backfill.
 FOR root IN SELECT t.item_instance_id FROM game_item_loop_subtree_edges(main) t
  WHERE t.depth>0 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=t.item_instance_id)
   AND EXISTS(SELECT 1 FROM game_player_death_item_records d WHERE d.item_instance_id=t.item_instance_id AND d.operation=1)
  ORDER BY t.depth,t.item_instance_id LIMIT 500
 LOOP
  SELECT array_agg(t.item_instance_id ORDER BY t.item_instance_id),count(*),count(DISTINCT t.item_instance_id),
   count(*) FILTER(WHERE t.source_kind=1),max(t.depth),bool_or(t.cyclic),
   jsonb_agg(to_jsonb(t) ORDER BY t.item_instance_id)
   INTO ids,count_nodes,distinct_nodes,direct_nodes,height,cycle,before_tree
   FROM game_item_loop_subtree_edges(root) t;
  IF direct_nodes=0 THEN CONTINUE; END IF;
  IF count_nodes>500 OR height>7 OR cycle OR count_nodes<>distinct_nodes
   OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t WHERE t.depth>0 AND t.world_id<>w)
   OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t WHERE t.depth>0
    GROUP BY t.parent_item_instance_id HAVING count(*)>20 OR count(*)<>count(DISTINCT t.placement_ordinal)) THEN
   RAISE EXCEPTION 'carried death reconciliation requires bounded exact tree' USING ERRCODE='23514';
  END IF;
  PERFORM i.item_instance_id FROM game_item_instances i WHERE i.item_instance_id=ANY(ids) ORDER BY i.item_instance_id FOR UPDATE OF i;
  SELECT jsonb_agg(to_jsonb(t) ORDER BY t.item_instance_id) INTO after_lock FROM game_item_loop_subtree_edges(root) t;
  IF before_tree IS DISTINCT FROM after_lock OR game_carried_death_current(f) IS DISTINCT FROM TRUE
   OR NOT EXISTS(SELECT 1 FROM game_item_loop_owner(root) o WHERE o.owner_character=c AND o.depth BETWEEN 1 AND 8)
   OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t JOIN game_item_loop_owner(root) o ON TRUE WHERE t.depth+o.depth>8)
   OR EXISTS(SELECT 1 FROM unnest(ids) x(item) LEFT JOIN game_item_instances i ON i.item_instance_id=x.item
    WHERE i.item_instance_id IS NULL OR i.world_id<>w OR i.lifecycle<>1)
   OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t JOIN game_item_instances i ON i.item_instance_id=t.parent_item_instance_id
    WHERE t.depth>0 AND i.quantity<>1) THEN
   RAISE EXCEPTION 'carried death reconciliation lost current exact custody' USING ERRCODE='23514';
  END IF;
  SELECT d.transaction_id INTO death FROM game_player_death_item_records d
   WHERE d.item_instance_id=root AND d.world_id=w AND d.operation=1 ORDER BY d.occurred_at DESC,d.transaction_id DESC LIMIT 1;
  IF NOT FOUND THEN RAISE EXCEPTION 'carried death reconciliation requires historical death provenance' USING ERRCODE='23514'; END IF;
  SELECT to_jsonb(i) INTO STRICT root_row FROM game_item_instances i WHERE i.item_instance_id=root;
  SELECT p.value INTO STRICT placement FROM (
   SELECT to_jsonb(e) AS value FROM game_item_container_entries e WHERE e.item_instance_id=root
   UNION ALL SELECT to_jsonb(e) FROM game_item_nested_entries e WHERE e.item_instance_id=root
  ) p;
  operation_id:=game_character_uuid_v7();
  INSERT INTO game_carried_death_reconciliations(transaction_id,root_item_instance_id,death_transaction_id,
   world_id,owner_character_id,fence,root_before,placement_before,tree_size,direct_count)
  VALUES(operation_id,root,death,w,c,f,root_row,placement,count_nodes,direct_nodes);
  INSERT INTO game_carried_death_edges(transaction_id,item_instance_id,world_id,parent_item_instance_id,
   placement_ordinal,placed_transaction_id,source_kind,source_character_id,item_before)
  SELECT operation_id,t.item_instance_id,t.world_id,t.parent_item_instance_id,t.placement_ordinal,
   t.placed_transaction_id,t.source_kind,t.character_id,to_jsonb(i)
   FROM game_item_loop_subtree_edges(root) t JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id
   WHERE t.depth>0 ORDER BY t.item_instance_id;
  GET DIAGNOSTICS changed=ROW_COUNT;
  IF changed<>count_nodes-1 THEN RAISE EXCEPTION 'carried reconciliation lost snapshots' USING ERRCODE='23514'; END IF;
  INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
  SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id
   FROM game_carried_death_edges e WHERE e.transaction_id=operation_id AND e.source_kind=1 ORDER BY e.item_instance_id;
  GET DIAGNOSTICS changed=ROW_COUNT;
  IF changed<>direct_nodes THEN RAISE EXCEPTION 'carried reconciliation lost insert' USING ERRCODE='23514'; END IF;
  DELETE FROM game_item_container_entries d USING game_carried_death_edges e WHERE e.transaction_id=operation_id AND e.source_kind=1
   AND (d.item_instance_id,d.world_id,d.parent_item_instance_id,d.placement_ordinal,d.placed_transaction_id,d.character_id)
    =(e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id,e.source_character_id);
  GET DIAGNOSTICS changed=ROW_COUNT;
  IF changed<>direct_nodes THEN RAISE EXCEPTION 'carried reconciliation lost removal' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

CREATE FUNCTION game_carried_death_complete() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE h game_carried_death_reconciliations%ROWTYPE:=NEW;
BEGIN
 IF h.created_xact_id<>pg_current_xact_id() OR game_carried_death_current(h.fence) IS DISTINCT FROM TRUE
  OR (h.fence->>'character')::uuid IS DISTINCT FROM h.owner_character_id
  OR (h.fence->>'world')::uuid IS DISTINCT FROM h.world_id
  OR NOT EXISTS(SELECT 1 FROM game_player_death_item_records d WHERE d.transaction_id=h.death_transaction_id
   AND d.item_instance_id=h.root_item_instance_id AND d.world_id=h.world_id AND d.operation=1)
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=h.root_item_instance_id AND to_jsonb(i)=h.root_before)
  OR NOT EXISTS(SELECT 1 FROM game_item_loop_owner(h.root_item_instance_id) o WHERE o.owner_character=h.owner_character_id AND o.depth BETWEEN 1 AND 8)
  OR NOT EXISTS(SELECT 1 FROM (
   SELECT to_jsonb(e) AS value FROM game_item_container_entries e WHERE e.item_instance_id=h.root_item_instance_id
   UNION ALL SELECT to_jsonb(e) FROM game_item_nested_entries e WHERE e.item_instance_id=h.root_item_instance_id
  ) p WHERE p.value=h.placement_before)
  OR (SELECT count(*) FROM game_carried_death_edges WHERE transaction_id=h.transaction_id AND created_xact_id=h.created_xact_id)<>h.tree_size-1
  OR (SELECT count(*) FROM game_carried_death_edges WHERE transaction_id=h.transaction_id AND source_kind=1)<>h.direct_count
  OR (WITH actual AS MATERIALIZED (SELECT * FROM game_item_loop_subtree_edges(h.root_item_instance_id) WHERE depth>0)
   SELECT (SELECT count(*) FROM actual)<>h.tree_size-1 OR EXISTS(
    SELECT 1 FROM game_carried_death_edges e LEFT JOIN actual a ON a.item_instance_id=e.item_instance_id
     LEFT JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
    WHERE e.transaction_id=h.transaction_id AND (a.item_instance_id IS NULL OR a.source_kind<>2 OR a.cyclic
     OR (a.world_id,a.parent_item_instance_id,a.placement_ordinal,a.placed_transaction_id)
      IS DISTINCT FROM (e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
     OR to_jsonb(i) IS DISTINCT FROM e.item_before))) THEN
  RAISE EXCEPTION 'carried reconciliation must retain exact tree and current authority' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_carried_death_complete AFTER INSERT ON game_carried_death_reconciliations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_carried_death_complete();

-- Narrow same-physical-XID representation arms; all existing owners remain.
DO $migration$
DECLARE body TEXT; at INT;
BEGIN
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing nested guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_carried_death_edges e JOIN game_carried_death_reconciliations h USING(transaction_id)
  WHERE e.source_kind=1 AND (e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
   =(NEW.item_instance_id,NEW.world_id,NEW.parent_item_instance_id,NEW.placement_ordinal,NEW.placed_transaction_id)
   AND e.created_xact_id=pg_current_xact_id() AND h.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$ from at for 5);
 body:=pg_get_functiondef('game_item_container_entry_removal_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing direct guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_carried_death_edges e JOIN game_carried_death_reconciliations h USING(transaction_id)
  WHERE e.source_kind=1 AND (e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id,e.source_character_id)
   =(OLD.item_instance_id,OLD.world_id,OLD.parent_item_instance_id,OLD.placement_ordinal,OLD.placed_transaction_id,OLD.character_id)
   AND e.created_xact_id=pg_current_xact_id() AND h.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
$arm$ from at for 5);
END $migration$;
DO $$ DECLARE name TEXT; signature REGPROCEDURE; BEGIN
 FOREACH name IN ARRAY ARRAY['game_carried_death_reconciliations','game_carried_death_edges'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',name||'_immutable',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',name||'_no_truncate',name);
 END LOOP;
 FOREACH signature IN ARRAY ARRAY['game_carried_death_current(JSONB)'::regprocedure,
  'game_carried_death_reconcile(JSONB,BYTEA)'::regprocedure,'game_carried_death_complete()'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_carried_death_reconcile(JSONB,BYTEA) TO oteryn_game_runtime;
