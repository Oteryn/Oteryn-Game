-- A death-dropped main backpack keeps the same logical container tree. Its old
-- direct-entry representation must become the ordinary nested representation so
-- later pickup, ownership, capacity and container views see the same contents.
-- This is custody REPRESENTATION maintenance, not a child transfer/effect: every
-- child ItemInstance and logical parent/world/ordinal/placement transaction stays
-- identical. DROP_LIMITS, audit lines and legacy receipt bytes remain unchanged.

-- One bounded logical read covers both historical direct edges and nested edges.
-- Include the first invalid depth/size/cycle instead of silently truncating it to
-- an apparently legal tree. A main root plus 500 descendants is legal; 502 rows
-- are sufficient to prove that ceiling was exceeded. No write capability here.
CREATE FUNCTION game_item_loop_subtree_edges(p_item UUID)
RETURNS TABLE(item_instance_id UUID,world_id UUID,parent_item_instance_id UUID,
 placement_ordinal NUMERIC,placed_transaction_id UUID,character_id UUID,
 source_kind SMALLINT,depth INT,cyclic BOOLEAN)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE edges AS NOT MATERIALIZED (
  SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,
   e.placed_transaction_id,e.character_id,1::SMALLINT AS source_kind
   FROM game_item_container_entries e
  UNION ALL
  SELECT n.item_instance_id,n.world_id,n.parent_item_instance_id,n.placement_ordinal,
   n.placed_transaction_id,NULL::UUID,2::SMALLINT FROM game_item_nested_entries n
 ), tree(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,
         placed_transaction_id,character_id,source_kind,depth,path,cyclic) AS (
  SELECT p_item,NULL::UUID,NULL::UUID,NULL::NUMERIC,NULL::UUID,NULL::UUID,
   0::SMALLINT,0,ARRAY[p_item],FALSE
  UNION ALL
  SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,
   e.placed_transaction_id,e.character_id,e.source_kind,t.depth+1,
   t.path || e.item_instance_id,e.item_instance_id = ANY(t.path)
   FROM tree t JOIN edges e ON e.parent_item_instance_id=t.item_instance_id
   WHERE t.depth<=8 AND NOT t.cyclic
 )
 SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,
  placed_transaction_id,character_id,source_kind,depth,cyclic FROM tree LIMIT 502
$$;

CREATE OR REPLACE FUNCTION game_item_loop_subtree(p_item UUID,OUT height INT,OUT size BIGINT)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH t AS MATERIALIZED (SELECT * FROM game_item_loop_subtree_edges(p_item))
 SELECT CASE WHEN bool_or(cyclic) OR count(*)<>count(DISTINCT item_instance_id)
   OR EXISTS(SELECT 1 FROM t WHERE depth>0 GROUP BY parent_item_instance_id,placement_ordinal HAVING count(*)>1)
  THEN 9 ELSE max(depth) END,count(*) FROM t
$$;

-- The main root is not one of the 500 carried descendants. Historical roots
-- already inside that tree must not hide their retained direct children either.
CREATE OR REPLACE FUNCTION game_item_loop_tree_size(p_character UUID) RETURNS BIGINT
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT COALESCE((SELECT CASE WHEN t.height>8 THEN 501 ELSE t.size-1 END
  FROM game_item_container_slots s CROSS JOIN LATERAL game_item_loop_subtree(s.item_instance_id) t
  WHERE s.character_id=p_character),0)
$$;

-- Runtime cannot author, alter or replay these source snapshots. The operation
-- is part of the key: a child may legally return to a main entry and be captured
-- again by a later death. Only this physical transaction can use either proof.
CREATE TABLE game_player_death_container_normalizations (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 root_item_instance_id UUID NOT NULL,
 world_id UUID NOT NULL,
 operation SMALLINT NOT NULL CHECK(operation IN(1,2)), -- death, historical Ground pickup
 death_transaction_id UUID NOT NULL REFERENCES game_player_death_item_records(transaction_id),
 loop_transaction_id UUID REFERENCES game_item_loop_move_receipts(transaction_id),
 ground_before JSONB,
 tree_size SMALLINT NOT NULL CHECK(tree_size BETWEEN 2 AND 501),
 direct_count SMALLINT NOT NULL CHECK(direct_count BETWEEN 1 AND 500),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(root_item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id),
 CHECK((operation=1 AND transaction_id=death_transaction_id AND loop_transaction_id IS NULL AND ground_before IS NULL)
    OR (operation=2 AND loop_transaction_id IS NOT NULL AND transaction_id=loop_transaction_id AND ground_before IS NOT NULL)),
 CHECK(direct_count<tree_size)
);
CREATE INDEX game_player_death_container_normalizations_root ON game_player_death_container_normalizations(root_item_instance_id);
CREATE INDEX game_player_death_container_normalizations_death ON game_player_death_container_normalizations(death_transaction_id);
CREATE INDEX game_player_death_container_normalizations_loop ON game_player_death_container_normalizations(loop_transaction_id);

CREATE TABLE game_player_death_container_edges (
 transaction_id UUID NOT NULL REFERENCES game_player_death_container_normalizations(transaction_id),
 item_instance_id UUID NOT NULL,
 world_id UUID NOT NULL,
 parent_item_instance_id UUID NOT NULL,
 placement_ordinal NUMERIC(20,0) NOT NULL CHECK(placement_ordinal BETWEEN 1 AND 18446744073709551615),
 placed_transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(placed_transaction_id)),
 source_kind SMALLINT NOT NULL CHECK(source_kind IN(1,2)),
 source_character_id UUID,
 item_before JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,item_instance_id),
 UNIQUE(transaction_id,parent_item_instance_id,placement_ordinal),
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id),
 FOREIGN KEY(parent_item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id),
 CHECK(item_instance_id<>parent_item_instance_id),
 CHECK((source_kind=1)=(source_character_id IS NOT NULL))
);
CREATE INDEX game_player_death_container_edges_item ON game_player_death_container_edges(item_instance_id);
CREATE INDEX game_player_death_container_edges_parent ON game_player_death_container_edges(parent_item_instance_id);
REVOKE ALL ON game_player_death_container_normalizations,game_player_death_container_edges FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT ON game_player_death_container_normalizations,game_player_death_container_edges TO oteryn_game_control;

CREATE FUNCTION game_player_death_container_normalize() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 death game_player_death_item_records%ROWTYPE;
 receipt game_item_loop_move_receipts%ROWTYPE;
 ground game_item_ground_locations%ROWTYPE;
 root UUID; world UUID; operation_id UUID; operation_kind SMALLINT;
 items UUID[]; node_count BIGINT; distinct_count BIGINT; direct_count BIGINT;
 height INT; cycle BOOLEAN; changed BIGINT; max_nodes INT; max_height INT;
BEGIN
 IF TG_TABLE_NAME='game_player_death_item_records' THEN
  IF NEW.operation<>1 THEN RETURN NULL; END IF;
  death:=NEW;
  root:=NEW.item_instance_id; world:=NEW.world_id; operation_id:=NEW.transaction_id; operation_kind:=1;
  max_nodes:=CASE WHEN NEW.source_slot=9 THEN 501 ELSE 500 END;
  max_height:=CASE WHEN NEW.source_slot=9 THEN 8 ELSE 7 END;
 ELSE
  IF NEW.from_kind<>3 OR NEW.to_kind NOT IN(1,2) THEN RETURN NULL; END IF;
  SELECT * INTO STRICT receipt FROM game_item_loop_move_receipts
   WHERE transaction_id=NEW.transaction_id AND created_xact_id=pg_current_xact_id();
  root:=NEW.item_instance_id; world:=receipt.world_id; operation_id:=NEW.transaction_id; operation_kind:=2;
  max_nodes:=500; max_height:=7;
 END IF;

 SELECT array_agg(t.item_instance_id),count(*),count(DISTINCT t.item_instance_id),
  count(*) FILTER(WHERE t.source_kind=1),max(t.depth),bool_or(t.cyclic)
  INTO items,node_count,distinct_count,direct_count,height,cycle
  FROM game_item_loop_subtree_edges(root) t;
 IF operation_kind=2 AND direct_count=0 THEN RETURN NULL; END IF;
 IF node_count>max_nodes OR height>max_height OR cycle OR node_count<>distinct_count
  OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t WHERE t.depth>0 AND t.world_id<>world)
  OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t WHERE t.depth>0
    GROUP BY t.parent_item_instance_id HAVING count(*)>20 OR count(*)<>count(DISTINCT t.placement_ordinal)) THEN
  RAISE EXCEPTION 'death container representation requires its bounded collision-free logical tree' USING ERRCODE='23514';
 END IF;

 -- Lock the exact existing ItemInstances in stable order; no child row is updated.
 PERFORM i.item_instance_id FROM game_item_instances i WHERE i.item_instance_id=ANY(items)
  ORDER BY i.item_instance_id FOR UPDATE OF i;
 IF EXISTS(SELECT 1 FROM unnest(items) ids(item) LEFT JOIN game_item_instances i ON i.item_instance_id=ids.item
   WHERE i.item_instance_id IS NULL OR i.world_id<>world OR i.lifecycle<>1)
  OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root) t JOIN game_item_instances i ON i.item_instance_id=t.parent_item_instance_id
   WHERE t.depth>0 AND i.quantity<>1) THEN
  RAISE EXCEPTION 'death container representation requires exact live same-world items' USING ERRCODE='23514';
 END IF;
 IF direct_count=0 THEN RETURN NULL; END IF;

 IF operation_kind=2 THEN
  -- A current admitted root line supplies authority. Old death evidence explains
  -- the representation only; it never substitutes for the current source/fence.
  SELECT * INTO STRICT ground FROM game_item_ground_locations WHERE item_instance_id=root FOR UPDATE;
  SELECT * INTO death FROM game_player_death_item_records d WHERE d.item_instance_id=root AND d.world_id=world AND d.operation=1
   AND d.created_xact_id<>pg_current_xact_id() ORDER BY d.occurred_at DESC,d.transaction_id DESC LIMIT 1;
  IF NOT FOUND OR NEW.created_xact_id<>pg_current_xact_id() OR receipt.operation<>5 OR receipt.line_count<>1
   OR NEW.quantity_before<>NEW.quantity_after OR ground.world_id<>world
   OR (ground.scope_kind,ground.channel_id,ground.instance_id) IS DISTINCT FROM (receipt.scope_kind,receipt.channel_id,receipt.instance_id)
   OR NOT game_item_loop_drop_ref(ground.corpse_ref) THEN
   RAISE EXCEPTION 'historical death container requires an exact current whole-root Ground pickup' USING ERRCODE='23514';
  END IF;
 END IF;

 INSERT INTO game_player_death_container_normalizations(transaction_id,root_item_instance_id,world_id,operation,
  death_transaction_id,loop_transaction_id,ground_before,tree_size,direct_count)
 VALUES(operation_id,root,world,operation_kind,death.transaction_id,
  CASE WHEN operation_kind=2 THEN operation_id END,CASE WHEN operation_kind=2 THEN to_jsonb(ground) END,node_count,direct_count);
 INSERT INTO game_player_death_container_edges(transaction_id,item_instance_id,world_id,parent_item_instance_id,
  placement_ordinal,placed_transaction_id,source_kind,source_character_id,item_before)
 SELECT operation_id,t.item_instance_id,t.world_id,t.parent_item_instance_id,t.placement_ordinal,
  t.placed_transaction_id,t.source_kind,t.character_id,to_jsonb(i)
  FROM game_item_loop_subtree_edges(root) t JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id
  WHERE t.depth>0 ORDER BY t.item_instance_id;

 -- Set-based conversion: at most 500 edges, with the exact existing parent,
 -- ordinal and original placement transaction. Never flatten or re-number.
 INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
 SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id
  FROM game_player_death_container_edges e WHERE e.transaction_id=operation_id AND e.source_kind=1 ORDER BY e.item_instance_id;
 GET DIAGNOSTICS changed=ROW_COUNT;
 IF changed<>direct_count THEN RAISE EXCEPTION 'death container conversion lost a source edge' USING ERRCODE='23514'; END IF;
 DELETE FROM game_item_container_entries prior_entry USING game_player_death_container_edges e
  WHERE e.transaction_id=operation_id AND e.source_kind=1 AND prior_entry.item_instance_id=e.item_instance_id
   AND prior_entry.world_id=e.world_id AND prior_entry.parent_item_instance_id=e.parent_item_instance_id
   AND prior_entry.placement_ordinal=e.placement_ordinal AND prior_entry.placed_transaction_id=e.placed_transaction_id
   AND prior_entry.character_id=e.source_character_id;
 GET DIAGNOSTICS changed=ROW_COUNT;
 IF changed<>direct_count THEN RAISE EXCEPTION 'death container conversion changed a source edge' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;

-- Check the complete logical before/after once per root, rather than rescanning
-- 500 descendants in every deferred child guard. Native root terminal guards
-- remain installed and independently validate the original receipt and audit.
CREATE FUNCTION game_player_death_container_normalization_complete(p_tx UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE h game_player_death_container_normalizations%ROWTYPE;
BEGIN
 SELECT * INTO STRICT h FROM game_player_death_container_normalizations
  WHERE transaction_id=p_tx AND created_xact_id=pg_current_xact_id();
 IF (SELECT count(*) FROM game_player_death_container_edges WHERE transaction_id=p_tx AND created_xact_id=pg_current_xact_id())<>h.tree_size-1
  OR (SELECT count(*) FROM game_player_death_container_edges WHERE transaction_id=p_tx AND source_kind=1 AND created_xact_id=pg_current_xact_id())<>h.direct_count
  OR (WITH actual AS MATERIALIZED (SELECT * FROM game_item_loop_subtree_edges(h.root_item_instance_id) WHERE depth>0)
   SELECT (SELECT count(*) FROM actual)<>h.tree_size-1 OR EXISTS(
    SELECT 1 FROM game_player_death_container_edges e LEFT JOIN actual a ON a.item_instance_id=e.item_instance_id
     LEFT JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
     WHERE e.transaction_id=p_tx AND (a.item_instance_id IS NULL OR a.source_kind<>2 OR a.cyclic
      OR (a.world_id,a.parent_item_instance_id,a.placement_ordinal,a.placed_transaction_id)
       IS DISTINCT FROM (e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
      OR to_jsonb(i) IS DISTINCT FROM e.item_before))) THEN
  RAISE EXCEPTION 'death container conversion must preserve every child and logical edge exactly' USING ERRCODE='23514';
 END IF;
 IF h.operation=1 THEN
  IF NOT EXISTS(SELECT 1 FROM game_player_death_item_records r WHERE r.transaction_id=p_tx
    AND r.item_instance_id=h.root_item_instance_id AND r.world_id=h.world_id AND r.operation=1
    AND r.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'death container conversion lacks its current root record' USING ERRCODE='23514';
  END IF;
  PERFORM game_player_death_item_complete(p_tx);
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_item_loop_move_lines l JOIN game_item_loop_move_receipts r USING(transaction_id)
    JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
    CROSS JOIN LATERAL game_item_loop_owner(i.item_instance_id) o
    WHERE l.transaction_id=p_tx AND l.item_instance_id=h.root_item_instance_id
     AND l.from_kind=3 AND l.to_kind IN(1,2) AND l.quantity_before=l.quantity_after
     AND r.world_id=h.world_id AND r.operation=5 AND r.line_count=1
     AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
     AND i.world_id=h.world_id AND i.lifecycle=1 AND i.quantity=l.quantity_after
     AND i.state_revision=l.state_revision_before+1 AND i.last_transaction_id=p_tx
     AND o.owner_character=r.character_id AND o.depth BETWEEN 1 AND 8
     AND ((l.to_kind=1 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
       AND e.world_id=h.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=l.to_parent_item_instance_id AND e.placed_transaction_id=p_tx))
      OR (l.to_kind=2 AND EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id
       AND n.world_id=h.world_id AND n.parent_item_instance_id=l.to_parent_item_instance_id AND n.placed_transaction_id=p_tx))))
   OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=h.root_item_instance_id) THEN
   RAISE EXCEPTION 'death container conversion must commit its exact current pickup root' USING ERRCODE='23514';
  END IF;
  PERFORM game_item_location_exclusive(h.root_item_instance_id);
 END IF;
END $$;

CREATE FUNCTION game_player_death_container_normalization_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_player_death_container_normalization_complete(NEW.transaction_id); RETURN NULL; END $$;

-- Extend installed guard chains; all prior owners/proofs remain intact. A proof
-- grants only the captured INSERT-nested / DELETE-direct pair, never a later
-- nested DELETE, UPDATE, re-parent, child state write or replay in another xact.
DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing installed nested custody proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_player_death_container_edges e
   JOIN game_player_death_container_normalizations h USING(transaction_id)
   WHERE e.source_kind=1 AND e.item_instance_id=NEW.item_instance_id AND e.world_id=NEW.world_id
    AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.placement_ordinal
    AND e.placed_transaction_id=NEW.placed_transaction_id
    AND e.created_xact_id=pg_current_xact_id() AND h.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
 body:=pg_get_functiondef('game_item_container_entry_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing installed direct custody proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_player_death_container_edges e
   JOIN game_player_death_container_normalizations h USING(transaction_id)
   WHERE e.source_kind=1 AND e.item_instance_id=OLD.item_instance_id AND e.world_id=OLD.world_id
    AND e.source_character_id=OLD.character_id AND e.parent_item_instance_id=OLD.parent_item_instance_id
    AND e.placement_ordinal=OLD.placement_ordinal AND e.placed_transaction_id=OLD.placed_transaction_id
    AND e.created_xact_id=pg_current_xact_id() AND h.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
$arm$ from begin_at for 5);
END $migration$;

CREATE TRIGGER game_player_death_container_normalize AFTER INSERT ON game_player_death_item_records
 FOR EACH ROW EXECUTE FUNCTION game_player_death_container_normalize();
CREATE TRIGGER game_player_death_container_normalize AFTER INSERT ON game_item_loop_move_lines
 FOR EACH ROW EXECUTE FUNCTION game_player_death_container_normalize();
CREATE CONSTRAINT TRIGGER game_player_death_container_normalization_proven
 AFTER INSERT ON game_player_death_container_normalizations DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_player_death_container_normalization_proven();

DO $$ DECLARE name TEXT; signature REGPROCEDURE; BEGIN
 FOREACH name IN ARRAY ARRAY['game_player_death_container_normalizations','game_player_death_container_edges'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',name||'_immutable',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',name||'_no_truncate',name);
 END LOOP;
 FOREACH signature IN ARRAY ARRAY[
  'game_player_death_container_normalize()'::regprocedure,
  'game_player_death_container_normalization_complete(UUID)'::regprocedure,
  'game_player_death_container_normalization_proven()'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
 FOREACH signature IN ARRAY ARRAY[
  'game_item_loop_subtree_edges(UUID)'::regprocedure,
  'game_item_loop_subtree(UUID)'::regprocedure,
  'game_item_loop_tree_size(UUID)'::regprocedure,
  'game_item_nested_entry_proven()'::regprocedure,
  'game_item_container_entry_removal_proven()'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_item_loop_subtree_edges(UUID) TO oteryn_game_runtime;
