-- Canonical all-nested trees entering an empty native ContainerSlot become MAIN:
-- only depth-one custody changes representation. No historical direct recovery.
-- Receipt names deliberately exclude this immutable journal from the existing
-- account-closure custody hash. Existing root TRANSFER proofs remain installed.
CREATE TABLE game_item_transfer_container_promotion_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_transfer_receipts(transaction_id),
 root_item_instance_id UUID NOT NULL,
 world_id UUID NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 tree_size SMALLINT NOT NULL CHECK(tree_size BETWEEN 2 AND 500),
 height SMALLINT NOT NULL CHECK(height BETWEEN 1 AND 8),
 immediate_count SMALLINT NOT NULL CHECK(immediate_count BETWEEN 1 AND 20),
 tree_sha256 BYTEA NOT NULL CHECK(octet_length(tree_sha256)=32),
 root_after JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(root_item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id)
);
CREATE INDEX game_item_transfer_container_promotion_receipts_root ON game_item_transfer_container_promotion_receipts(root_item_instance_id);
CREATE INDEX game_item_transfer_container_promotion_receipts_character ON game_item_transfer_container_promotion_receipts(character_id);
CREATE TABLE game_item_transfer_container_promotion_edge_receipts (
 transaction_id UUID NOT NULL REFERENCES game_item_transfer_container_promotion_receipts(transaction_id),
 item_instance_id UUID NOT NULL,
 world_id UUID NOT NULL,
 parent_item_instance_id UUID NOT NULL,
 placement_ordinal NUMERIC(20,0) NOT NULL CHECK(placement_ordinal BETWEEN 1 AND 18446744073709551615),
 placed_transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(placed_transaction_id)),
 depth SMALLINT NOT NULL CHECK(depth BETWEEN 1 AND 8),
 item_before JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,item_instance_id),
 UNIQUE(transaction_id,parent_item_instance_id,placement_ordinal),
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id),
 FOREIGN KEY(parent_item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id),
 CHECK(item_instance_id<>parent_item_instance_id)
);
CREATE INDEX game_item_transfer_container_promotion_edge_receipts_item ON game_item_transfer_container_promotion_edge_receipts(item_instance_id);
CREATE INDEX game_item_transfer_container_promotion_edge_receipts_parent ON game_item_transfer_container_promotion_edge_receipts(parent_item_instance_id);
REVOKE ALL ON game_item_transfer_container_promotion_receipts,game_item_transfer_container_promotion_edge_receipts
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Constant-size binding used by both exact-row guard arms and completion. A
-- historical header/receipt cannot authorize another physical transaction.
CREATE FUNCTION game_item_transfer_container_current(p_tx UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_transfer_container_promotion_receipts h
  JOIN game_item_transfer_receipts r USING(transaction_id)
  JOIN game_item_transfer_reservations v USING(game_session_id,command_id)
  JOIN game_item_instances i ON i.item_instance_id=h.root_item_instance_id
  JOIN game_item_container_slots s ON s.character_id=h.character_id
  WHERE h.transaction_id=p_tx AND h.created_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND r.shape=1
   AND r.source_quantity_before=1 AND r.source_quantity_after=1
   AND r.receiver_item_instance_id IS NULL AND r.destination_parent_item_instance_id IS NULL
   AND r.destination_ordinal IS NULL
   AND (r.source_item_instance_id,r.character_id)=(h.root_item_instance_id,h.character_id)
   AND (v.character_id,v.source_item_instance_id,v.transaction_id,v.event_id,v.intent_binding,v.occurred_at,v.world_id,v.destination_kind)
    =(r.character_id,r.source_item_instance_id,r.transaction_id,r.event_id,r.intent_binding,r.occurred_at,h.world_id,1::SMALLINT)
   AND (s.item_instance_id,s.world_id,s.placed_transaction_id)=(h.root_item_instance_id,h.world_id,p_tx)
   AND i.world_id=h.world_id AND i.quantity=1 AND i.lifecycle=1 AND i.last_transaction_id=p_tx
   AND to_jsonb(i)=h.root_after)
$$;

CREATE FUNCTION game_item_transfer_container_promote() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 root_row game_item_instances%ROWTYPE;
 nodes BIGINT; distinct_nodes BIGINT; height INT; immediate BIGINT; cycle BOOLEAN;
 binding BYTEA; changed BIGINT;
BEGIN
 IF NEW.shape<>1 THEN RETURN NULL; END IF;
 SELECT * INTO STRICT root_row FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id;
 IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.source_quantity_before<>1 OR NEW.source_quantity_after<>1
  OR NEW.receiver_item_instance_id IS NOT NULL OR NEW.destination_parent_item_instance_id IS NOT NULL
  OR NEW.destination_ordinal IS NOT NULL OR root_row.lifecycle<>1 OR root_row.quantity<>1
  OR root_row.last_transaction_id IS DISTINCT FROM NEW.transaction_id
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_reservations v
   WHERE (v.game_session_id,v.command_id,v.character_id,v.source_item_instance_id,v.intent_binding,v.transaction_id,v.event_id,v.occurred_at,v.world_id,v.destination_kind)
    =(NEW.game_session_id,NEW.command_id,NEW.character_id,NEW.source_item_instance_id,NEW.intent_binding,NEW.transaction_id,NEW.event_id,NEW.occurred_at,root_row.world_id,1::SMALLINT))
  OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.character_id=NEW.character_id
   AND (s.item_instance_id,s.world_id,s.placed_transaction_id)=(root_row.item_instance_id,root_row.world_id,NEW.transaction_id)) THEN
  RAISE EXCEPTION 'container promotion requires exact current native slot TRANSFER' USING ERRCODE='23514';
 END IF;
 SELECT count(*),count(DISTINCT t.item_instance_id),max(t.depth),bool_or(t.cyclic),count(*) FILTER(WHERE t.depth=1),
  sha256(string_agg(uuid_send(t.item_instance_id),''::BYTEA ORDER BY t.item_instance_id))
  INTO nodes,distinct_nodes,height,cycle,immediate,binding FROM game_item_loop_subtree_edges(root_row.item_instance_id) t;
 IF nodes>500 OR height>8 OR cycle OR nodes<>distinct_nodes
  OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root_row.item_instance_id) t
   LEFT JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id
   WHERE i.item_instance_id IS NULL OR i.world_id<>root_row.world_id OR i.lifecycle<>1 OR i.quantity<1
    OR (t.depth>0 AND (t.source_kind<>2 OR t.world_id<>root_row.world_id)))
  OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root_row.item_instance_id) t WHERE t.depth>0
   GROUP BY t.parent_item_instance_id HAVING count(*)>20 OR count(*)<>count(DISTINCT t.placement_ordinal))
  OR EXISTS(SELECT 1 FROM game_item_loop_subtree_edges(root_row.item_instance_id) t
   JOIN game_item_instances i ON i.item_instance_id=t.parent_item_instance_id WHERE t.depth>0 AND i.quantity<>1) THEN
  RAISE EXCEPTION 'container promotion requires a canonical bounded live tree' USING ERRCODE='23514';
 END IF;
 IF nodes=1 THEN RETURN NULL; END IF;
 INSERT INTO game_item_transfer_container_promotion_receipts(transaction_id,root_item_instance_id,world_id,character_id,
  tree_size,height,immediate_count,tree_sha256,root_after)
 VALUES(NEW.transaction_id,root_row.item_instance_id,root_row.world_id,NEW.character_id,nodes,height,immediate,binding,to_jsonb(root_row));
 -- Capture ALL descendants before conversion, including untouched deeper rows.
 INSERT INTO game_item_transfer_container_promotion_edge_receipts(transaction_id,item_instance_id,world_id,
  parent_item_instance_id,placement_ordinal,placed_transaction_id,depth,item_before)
 SELECT NEW.transaction_id,t.item_instance_id,t.world_id,t.parent_item_instance_id,t.placement_ordinal,
  t.placed_transaction_id,t.depth,to_jsonb(i) FROM game_item_loop_subtree_edges(root_row.item_instance_id) t
  JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id WHERE t.depth>0 ORDER BY t.item_instance_id;
 GET DIAGNOSTICS changed=ROW_COUNT;
 IF changed<>nodes-1 THEN RAISE EXCEPTION 'container promotion lost captured children' USING ERRCODE='23514'; END IF;
 INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
 SELECT e.item_instance_id,e.world_id,NEW.character_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id
  FROM game_item_transfer_container_promotion_edge_receipts e WHERE e.transaction_id=NEW.transaction_id AND e.depth=1 ORDER BY e.item_instance_id;
 GET DIAGNOSTICS changed=ROW_COUNT;
 IF changed<>immediate THEN RAISE EXCEPTION 'container promotion lost direct insert' USING ERRCODE='23514'; END IF;
 DELETE FROM game_item_nested_entries n USING game_item_transfer_container_promotion_edge_receipts e
  WHERE e.transaction_id=NEW.transaction_id AND e.depth=1
   AND (n.item_instance_id,n.world_id,n.parent_item_instance_id,n.placement_ordinal,n.placed_transaction_id)
    =(e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id);
 GET DIAGNOSTICS changed=ROW_COUNT;
 IF changed<>immediate THEN RAISE EXCEPTION 'container promotion lost nested removal' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;

-- Historical replay verifies only immutable members and their real receipt,
-- never mutable present custody. Singleton/older receipts have no journal.
CREATE FUNCTION game_item_transfer_container_binding(p_tx UUID)
RETURNS TABLE(tree_size SMALLINT,tree_sha256 BYTEA)
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE h game_item_transfer_container_promotion_receipts%ROWTYPE; nodes BIGINT; binding BYTEA;
BEGIN
 SELECT * INTO h FROM game_item_transfer_container_promotion_receipts WHERE transaction_id=p_tx;
 IF NOT FOUND THEN RETURN; END IF;
 SELECT count(*),sha256(string_agg(uuid_send(m.item),''::BYTEA ORDER BY m.item)) INTO nodes,binding
  FROM (SELECT h.root_item_instance_id AS item UNION ALL
   SELECT e.item_instance_id FROM game_item_transfer_container_promotion_edge_receipts e WHERE e.transaction_id=p_tx) m;
 IF nodes<>h.tree_size OR binding<>h.tree_sha256
  OR EXISTS(SELECT 1 FROM game_item_transfer_container_promotion_edge_receipts e WHERE e.transaction_id=p_tx
   AND (e.world_id<>h.world_id OR e.created_xact_id<>h.created_xact_id))
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_receipts r JOIN game_item_transfer_reservations v USING(game_session_id,command_id)
   WHERE r.transaction_id=p_tx AND r.created_xact_id=h.created_xact_id AND r.shape=1
    AND r.source_item_instance_id=h.root_item_instance_id AND r.character_id=h.character_id
    AND v.transaction_id=p_tx AND v.world_id=h.world_id AND v.intent_binding=r.intent_binding
    AND v.event_id=r.event_id AND v.destination_kind=1) THEN
  RAISE EXCEPTION 'container promotion historical binding differs' USING ERRCODE='23514';
 END IF;
 tree_size:=h.tree_size; tree_sha256:=h.tree_sha256; RETURN NEXT;
END $$;

CREATE FUNCTION game_item_transfer_container_complete() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE h game_item_transfer_container_promotion_receipts%ROWTYPE;
BEGIN
 h:=NEW;
 IF NOT game_item_transfer_container_current(h.transaction_id)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=h.root_item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=h.root_item_instance_id)
  OR (SELECT count(*) FROM game_item_transfer_container_promotion_edge_receipts e WHERE e.transaction_id=h.transaction_id AND e.depth=1)<>h.immediate_count
  OR (WITH actual AS MATERIALIZED (SELECT * FROM game_item_loop_subtree_edges(h.root_item_instance_id))
   SELECT (SELECT count(*) FROM actual)<>h.tree_size
    OR (SELECT max(depth) FROM actual)<>h.height
    OR (SELECT sha256(string_agg(uuid_send(item_instance_id),''::BYTEA ORDER BY item_instance_id)) FROM actual)<>h.tree_sha256
    OR EXISTS(SELECT 1 FROM game_item_transfer_container_promotion_edge_receipts e
     LEFT JOIN actual a ON a.item_instance_id=e.item_instance_id
     LEFT JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
     WHERE e.transaction_id=h.transaction_id AND (a.item_instance_id IS NULL OR a.cyclic
      OR (a.world_id,a.parent_item_instance_id,a.placement_ordinal,a.placed_transaction_id,a.depth)
       IS DISTINCT FROM (e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id,e.depth::INT)
      OR a.source_kind<>CASE WHEN e.depth=1 THEN 1 ELSE 2 END
      OR (e.depth=1 AND a.character_id IS DISTINCT FROM h.character_id)
      OR to_jsonb(i) IS DISTINCT FROM e.item_before))) THEN
  RAISE EXCEPTION 'container promotion must preserve every child and exact logical edge' USING ERRCODE='23514';
 END IF;
 PERFORM * FROM game_item_transfer_container_binding(h.transaction_id);
 RETURN NULL;
END $$;

-- Preserve the installed guard bodies verbatim after one verified outer BEGIN.
-- The table test must precede entry-only NEW.character_id field access because
-- game_item_placement_proven also receives ContainerSlot rows.
DO $migration$
DECLARE body TEXT; anchor TEXT; begin_at INT;
BEGIN
 FOREACH anchor IN ARRAY ARRAY['game_item_placement_proven()','game_item_nested_entry_proven()'] LOOP
  body:=pg_get_functiondef(anchor::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 OR substr(body,begin_at,5) NOT IN('BEGIN','begin') THEN
   RAISE EXCEPTION 'missing installed container proof anchor: %',anchor;
  END IF;
  IF anchor='game_item_placement_proven()' THEN
   EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_TABLE_NAME='game_item_container_entries' THEN
  IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_item_transfer_container_promotion_edge_receipts e
   JOIN game_item_transfer_container_promotion_receipts h USING(transaction_id)
   WHERE e.depth=1 AND e.created_xact_id=pg_current_xact_id()
    AND h.created_xact_id=pg_current_xact_id() AND game_item_transfer_container_current(h.transaction_id)
    AND (NEW.item_instance_id,NEW.world_id,NEW.character_id,NEW.parent_item_instance_id,NEW.placement_ordinal,NEW.placed_transaction_id)
     =(e.item_instance_id,e.world_id,h.character_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)) THEN RETURN NULL; END IF;
 END IF;
$arm$ from begin_at for 5);
  ELSE
   EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_transfer_container_promotion_edge_receipts e
  JOIN game_item_transfer_container_promotion_receipts h USING(transaction_id)
  WHERE e.depth=1 AND e.created_xact_id=pg_current_xact_id()
   AND h.created_xact_id=pg_current_xact_id() AND game_item_transfer_container_current(h.transaction_id)
   AND (OLD.item_instance_id,OLD.world_id,OLD.parent_item_instance_id,OLD.placement_ordinal,OLD.placed_transaction_id)
    =(e.item_instance_id,e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
  END IF;
 END LOOP;
END $migration$;

CREATE TRIGGER game_item_transfer_container_promote AFTER INSERT ON game_item_transfer_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_transfer_container_promote();
CREATE CONSTRAINT TRIGGER game_item_transfer_container_complete
 AFTER INSERT ON game_item_transfer_container_promotion_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_item_transfer_container_complete();
DO $$ DECLARE name TEXT; signature REGPROCEDURE; BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_transfer_container_promotion_receipts','game_item_transfer_container_promotion_edge_receipts'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',name||'_immutable',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',name||'_no_truncate',name);
 END LOOP;
 FOREACH signature IN ARRAY ARRAY[
  'game_item_transfer_container_current(UUID)'::regprocedure,
  'game_item_transfer_container_promote()'::regprocedure,
  'game_item_transfer_container_binding(UUID)'::regprocedure,
  'game_item_transfer_container_complete()'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_item_transfer_container_binding(UUID) TO oteryn_game_runtime;
