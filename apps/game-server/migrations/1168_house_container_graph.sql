-- W18 House cupboards retain the existing item/nested custody graph.
-- Crystal src/items/containers/container.cpp queryAdd/queryRemove and
-- data/items/items.xml:65374-65384 qualify the cupboard's existing capacity8.
-- This helper is a bounded historical path, never current House authority.
CREATE FUNCTION game_house_container_root(p_item UUID)
RETURNS TABLE(root UUID,world UUID,house TEXT,depth INTEGER,"position" BYTEA)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE path(item,hops,seen,cycle) AS (
  SELECT p_item,0,ARRAY[p_item],false
  UNION ALL
  SELECT n.parent_item_instance_id,p.hops+1,p.seen||n.parent_item_instance_id,
   n.parent_item_instance_id=ANY(p.seen)
  FROM path p JOIN game_item_nested_entries n ON n.item_instance_id=p.item
  WHERE p.hops<9 AND NOT p.cycle
 )
 SELECT h.item_instance_id,h.world_id,h.house_key,p.hops,h.spatial_position
 FROM path p JOIN game_item_house_interior_locations h ON h.item_instance_id=p.item
 WHERE p.hops<=8 AND NOT p.cycle
  AND NOT EXISTS(SELECT 1 FROM path q LEFT JOIN game_item_instances i ON i.item_instance_id=q.item
   WHERE q.cycle OR q.hops>8 OR i.lifecycle IS DISTINCT FROM 1 OR i.world_id IS DISTINCT FROM h.world_id
    OR (q.hops>0 AND i.quantity IS DISTINCT FROM 1))
  AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=h.item_instance_id)
 LIMIT 1
$$;

-- A House root's capacity is admitted only by a fenced move receipt. The row is
-- provenance, not a second item location or an authority grant for a later turn.
CREATE TABLE game_house_container_roots (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 definition_family TEXT NOT NULL,
 definition_production_key TEXT NOT NULL,
 definition_revision_ref TEXT NOT NULL,
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 capacity SMALLINT NOT NULL CHECK(capacity BETWEEN 1 AND 25),
 admitted_transaction_id UUID NOT NULL,
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id)
);
CREATE INDEX game_house_container_roots_house ON game_house_container_roots(world_id,house_key);

CREATE TABLE game_house_container_loop_proofs (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_loop_move_receipts(transaction_id)
  DEFERRABLE INITIALLY DEFERRED,
 game_session_id UUID NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 lease_generation NUMERIC(20,0) NOT NULL CHECK(lease_generation>0),
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 node_id UUID NOT NULL,
 node_revision NUMERIC(20,0) NOT NULL CHECK(node_revision>0),
 actor_position BYTEA NOT NULL CHECK(octet_length(actor_position)=12),
 source_root UUID REFERENCES game_item_instances(item_instance_id),
 destination_root UUID REFERENCES game_item_instances(item_instance_id),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 capacity_limit SMALLINT NOT NULL DEFAULT 20 CHECK(capacity_limit IN(20,25)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_house_container_loop_proofs_character ON game_house_container_loop_proofs(character_id);
CREATE INDEX game_house_container_loop_proofs_source ON game_house_container_loop_proofs(source_root);
CREATE INDEX game_house_container_loop_proofs_destination ON game_house_container_loop_proofs(destination_root);
ALTER TABLE game_house_container_roots ADD FOREIGN KEY(admitted_transaction_id)
 REFERENCES game_house_container_loop_proofs(transaction_id) DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX game_house_container_roots_admission ON game_house_container_roots(admitted_transaction_id);

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_container_root(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_container_root(UUID) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_container_root(UUID) TO oteryn_game_runtime;
REVOKE ALL ON game_house_container_roots,game_house_container_loop_proofs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_container_roots,game_house_container_loop_proofs TO oteryn_game_runtime,oteryn_game_control;
