-- COIN-EXCHANGE-1 / owner decision 27. The sole new table is the immutable command receipt.
-- A pending receipt retains acceptance across process loss; finalization and all custody changes
-- share one physical transaction. Before/after lineage is the durable conversion evidence.
CREATE TABLE IF NOT EXISTS game_coin_exchange_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 exchange SMALLINT NOT NULL CHECK(exchange BETWEEN 1 AND 4),
 amount NUMERIC(20,0) NOT NULL CHECK(amount BETWEEN 0 AND 18446744073709551615),
 npc_identity BYTEA NOT NULL CHECK(octet_length(npc_identity)=16),
 npc_generation NUMERIC(20,0) NOT NULL CHECK(npc_generation BETWEEN 1 AND 18446744073709551615),
 npc_key TEXT NOT NULL CHECK(octet_length(npc_key) BETWEEN 1 AND 512),
 catalogue_revision NUMERIC(20,0) NOT NULL CHECK(catalogue_revision BETWEEN 1 AND 18446744073709551615),
 conversation_revision NUMERIC(20,0) NOT NULL CHECK(conversation_revision BETWEEN 1 AND 18446744073709551615),
 result SMALLINT NOT NULL DEFAULT 0 CHECK(result BETWEEN 0 AND 4),
 inputs JSONB NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs)<=500),
 outputs JSONB NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs)<=500),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(game_session_id,command_id),
 CHECK(result=1 OR (inputs='[]' AND outputs='[]'))
);
CREATE OR REPLACE FUNCTION game_coin_exchange_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE l JSONB; i game_item_instances%ROWTYPE; input_key TEXT; output_key TEXT; parent UUID;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'immutable coin exchange receipt'; END IF;
 IF TG_OP='INSERT' THEN
  IF NEW.result<>0 OR NEW.inputs<>'[]' OR NEW.outputs<>'[]' THEN RAISE EXCEPTION 'exchange must be accepted first'; END IF;
 ELSE
  IF OLD.result<>0 OR NEW.result=0 OR (to_jsonb(NEW)-ARRAY['result','inputs','outputs','created_xact_id']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['result','inputs','outputs','created_xact_id']) THEN
   RAISE EXCEPTION 'immutable coin exchange request';
  END IF;
  IF NEW.result=1 THEN
   input_key:=CASE NEW.exchange WHEN 1 THEN 'oteryn:item.tibia.i3031' WHEN 4 THEN 'oteryn:item.tibia.i3043' ELSE 'oteryn:item.tibia.i3035' END;
   output_key:=CASE NEW.exchange WHEN 2 THEN 'oteryn:item.tibia.i3031' WHEN 3 THEN 'oteryn:item.tibia.i3043' ELSE 'oteryn:item.tibia.i3035' END;
   IF NEW.amount<1 OR NEW.amount>4294967295 OR jsonb_array_length(NEW.inputs)=0 OR jsonb_array_length(NEW.outputs)=0
    OR (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.inputs||NEW.outputs))<>jsonb_array_length(NEW.inputs||NEW.outputs)
    OR (SELECT sum((value->>'count')::numeric) FROM jsonb_array_elements(NEW.inputs))<>NEW.amount*(CASE WHEN NEW.exchange IN (1,3) THEN 100 ELSE 1 END)
    OR (SELECT sum((value->>'count')::numeric) FROM jsonb_array_elements(NEW.outputs))<>NEW.amount*(CASE WHEN NEW.exchange IN (2,4) THEN 100 ELSE 1 END) THEN
    RAISE EXCEPTION 'invalid coin exchange conservation';
   END IF;
   FOR l IN SELECT value FROM jsonb_array_elements(NEW.inputs||NEW.outputs) LOOP
    parent:=(l->>'parent')::uuid;
    IF NOT EXISTS(WITH RECURSIVE carried AS (
      SELECT item_instance_id,0 AS depth FROM game_item_container_slots WHERE character_id=NEW.character_id AND world_id=NEW.world_id
      UNION ALL SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN (
       SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries WHERE character_id=NEW.character_id AND world_id=NEW.world_id
       UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries WHERE world_id=NEW.world_id
      ) e ON e.parent_item_instance_id=c.item_instance_id WHERE c.depth<7)
     SELECT 1 FROM carried WHERE item_instance_id=parent AND (depth>0)=(l->>'nested')::boolean)
     OR (l->>'count')::int NOT BETWEEN 1 AND 100 OR (l->>'ordinal')::numeric NOT BETWEEN 1 AND 18446744073709551615 THEN
     RAISE EXCEPTION 'exchange requires carried container';
    END IF;
    IF l ? 'before' THEN
     SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid FOR UPDATE;
     IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM l->'before' OR i.lifecycle<>1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
      OR i.quantity NOT BETWEEN 1 AND 100
      OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
      OR NOT EXISTS(SELECT 1 FROM (SELECT item_instance_id,parent_item_instance_id,placement_ordinal,false AS nested FROM game_item_container_entries WHERE character_id=NEW.character_id AND world_id=NEW.world_id
       UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries WHERE world_id=NEW.world_id) e WHERE e.item_instance_id=i.item_instance_id AND e.parent_item_instance_id=parent AND e.placement_ordinal=(l->>'ordinal')::numeric AND e.nested=(l->>'nested')::boolean) THEN
      RAISE EXCEPTION 'exchange requires unchanged plain coin';
     END IF;
     IF NEW.inputs @> jsonb_build_array(l) THEN
      IF i.definition_production_key<>input_key OR (l->>'count')::int>i.quantity THEN RAISE EXCEPTION 'wrong exchange input'; END IF;
     ELSE
      IF i.definition_production_key<>output_key OR i.definition_production_key IS DISTINCT FROM l->>'key' OR i.definition_revision_ref IS DISTINCT FROM l->>'revision' OR i.quantity+(l->>'count')::int>100 THEN RAISE EXCEPTION 'wrong exchange receiver'; END IF;
     END IF;
    ELSE
     IF NEW.inputs @> jsonb_build_array(l) OR l->>'key' IS DISTINCT FROM output_key OR NOT game_character_is_uuid_v7((l->>'item')::uuid)
      OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid) OR octet_length(l->>'revision') NOT BETWEEN 1 AND 512 THEN RAISE EXCEPTION 'wrong exchange mint'; END IF;
    END IF;
   END LOOP;
  END IF;
 END IF;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_coin_exchange_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_coin_exchange_receipts%ROWTYPE; l JSONB; i game_item_instances%ROWTYPE; expected BIGINT;
BEGIN
 SELECT * INTO STRICT r FROM game_coin_exchange_receipts WHERE transaction_id=p_transaction;
 IF r.result<>1 THEN RETURN; END IF;
 IF r.created_xact_id<>pg_current_xact_id()
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>(SELECT count(*) FROM jsonb_array_elements(r.outputs) AS output(value) WHERE NOT output.value ? 'before')
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>jsonb_array_length(r.inputs)+(SELECT count(*) FROM jsonb_array_elements(r.outputs) AS output(value) WHERE output.value ? 'before') THEN
  RAISE EXCEPTION 'incomplete coin exchange lineage';
 END IF;
 -- Independent custody ceilings match the item loop; Content may impose a smaller capacity.
 IF EXISTS(SELECT 1 FROM (SELECT DISTINCT (value->>'parent')::uuid AS parent FROM jsonb_array_elements(r.outputs)) p
  WHERE (SELECT count(*) FROM (SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=p.parent
   UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=p.parent) e)>20)
  OR (WITH RECURSIVE carried AS (
   SELECT item_instance_id,0 AS depth FROM game_item_container_slots WHERE character_id=r.character_id AND world_id=r.world_id
   UNION ALL SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN (
    SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries WHERE character_id=r.character_id AND world_id=r.world_id
    UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries WHERE world_id=r.world_id
   ) e ON e.parent_item_instance_id=c.item_instance_id WHERE c.depth<8)
   SELECT count(*)>501 FROM (SELECT item_instance_id FROM carried LIMIT 502) bounded) THEN
  RAISE EXCEPTION 'exchange exceeds carried custody ceiling';
 END IF;
 FOR l IN SELECT value FROM jsonb_array_elements(r.inputs||r.outputs) LOOP
  SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid;
  IF r.inputs @> jsonb_build_array(l) THEN
   expected:=(l->'before'->>'quantity')::bigint-(l->>'count')::bigint;
  ELSE expected:=coalesce((l->'before'->>'quantity')::bigint,0)+(l->>'count')::bigint; END IF;
  IF i.quantity<>expected OR i.lifecycle<>(CASE WHEN expected=0 THEN 2 ELSE 1 END)
   OR (l ? 'before' AND (i.last_transaction_id IS DISTINCT FROM r.transaction_id OR i.state_revision<>(l->'before'->>'state_revision')::numeric+1
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM ((l->'before')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])))
   OR (NOT l ? 'before' AND (i.minted_transaction_id IS DISTINCT FROM r.transaction_id OR i.last_transaction_id IS NOT NULL OR i.state_revision<>1 OR i.definition_family<>'Item' OR i.world_id<>r.world_id OR i.definition_production_key IS DISTINCT FROM l->>'key' OR i.definition_revision_ref IS DISTINCT FROM l->>'revision')) THEN
   RAISE EXCEPTION 'exchange item after-state mismatch';
  END IF;
  IF expected=0 THEN
   IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id) THEN RAISE EXCEPTION 'retired exchange coin still placed'; END IF;
  ELSIF NOT EXISTS(SELECT 1 FROM (SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id,false AS nested FROM game_item_container_entries WHERE character_id=r.character_id
    UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id,true FROM game_item_nested_entries) e
    WHERE e.item_instance_id=i.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=(l->>'parent')::uuid AND e.placement_ordinal=(l->>'ordinal')::numeric AND e.nested=(l->>'nested')::boolean AND (l ? 'before' OR e.placed_transaction_id=r.transaction_id)) THEN
   RAISE EXCEPTION 'exchange location mismatch';
  END IF;
 END LOOP;
END $$;
CREATE OR REPLACE FUNCTION game_coin_exchange_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_coin_exchange_complete(NEW.transaction_id); RETURN NULL; END $$;
DROP TRIGGER IF EXISTS game_coin_exchange_admit ON game_coin_exchange_receipts;
CREATE TRIGGER game_coin_exchange_admit BEFORE INSERT OR UPDATE OR DELETE ON game_coin_exchange_receipts FOR EACH ROW EXECUTE FUNCTION game_coin_exchange_admit();
DROP TRIGGER IF EXISTS game_coin_exchange_consistent ON game_coin_exchange_receipts;
CREATE CONSTRAINT TRIGGER game_coin_exchange_consistent AFTER UPDATE ON game_coin_exchange_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_coin_exchange_consistent();
-- Add this closed conversion proof to the existing custody owners; retain every other arm.
DO $migration$ DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_change_proven','game_item_container_entry_removal_proven','game_item_nested_entry_proven','game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('game_coin_exchange_receipts' IN body)>0 THEN CONTINUE; END IF;
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected custody proof body: %',n; END IF;
  IF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_coin_exchange_receipts r, LATERAL jsonb_array_elements(r.inputs||r.outputs) l WHERE r.result=1 AND (l->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_coin_exchange_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSIF n IN ('game_item_container_entry_removal_proven','game_item_nested_entry_proven') THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_coin_exchange_receipts r, LATERAL jsonb_array_elements(r.inputs) l WHERE r.result=1 AND (l->>'item')::uuid=OLD.item_instance_id AND r.world_id=OLD.world_id AND (l->>'parent')::uuid=OLD.parent_item_instance_id AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND ((l->>'nested')::boolean)=(TG_TABLE_NAME='game_item_nested_entries') AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_coin_exchange_complete(r.transaction_id) FROM game_coin_exchange_receipts r WHERE r.created_xact_id=pg_current_xact_id() AND r.result=1 AND r.inputs @> jsonb_build_array(jsonb_build_object('item',OLD.item_instance_id::text)); RETURN NULL;
    END IF;
$arm$;
   IF n='game_item_nested_entry_proven' THEN
    arm:=arm||$arm$
    IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_coin_exchange_receipts r, LATERAL jsonb_array_elements(r.outputs) l WHERE r.result=1 AND r.transaction_id=NEW.placed_transaction_id AND (l->>'item')::uuid=NEW.item_instance_id AND (l->>'parent')::uuid=NEW.parent_item_instance_id AND (l->>'ordinal')::numeric=NEW.placement_ordinal AND l->>'nested'='true' AND r.world_id=NEW.world_id AND r.created_xact_id=pg_current_xact_id()) THEN PERFORM game_coin_exchange_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
$arm$;
   END IF;
  ELSIF n='game_item_placement_proven' THEN
   arm:=$arm$
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_coin_exchange_receipts r, LATERAL jsonb_array_elements(r.outputs) l WHERE r.result=1 AND r.transaction_id=NEW.placed_transaction_id AND (l->>'item')::uuid=NEW.item_instance_id AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id AND (l->>'parent')::uuid=NEW.parent_item_instance_id AND (l->>'ordinal')::numeric=NEW.placement_ordinal AND l->>'nested'='false' AND r.created_xact_id=pg_current_xact_id()) THEN PERFORM game_coin_exchange_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_coin_exchange_receipts r, LATERAL jsonb_array_elements(r.outputs) l WHERE r.result=1 AND (l->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.minted_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN PERFORM game_coin_exchange_complete(NEW.minted_transaction_id); RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_coin_exchange_admit','game_coin_exchange_consistent'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_coin_exchange_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_coin_exchange_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_coin_exchange_admit(),game_coin_exchange_consistent(),game_coin_exchange_complete(UUID) FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_coin_exchange_receipts TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_coin_exchange_complete(UUID) TO oteryn_game_runtime;
