-- CRAFT-1 basic fixed cooking. Reuse existing ItemInstance and main-backpack rows;
-- one immutable CommandRef receipt proves the decrement and every fresh output.
CREATE TABLE game_item_craft_records (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 529),
 recipe TEXT NOT NULL CHECK(octet_length(recipe) BETWEEN 1 AND 512),
 source_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 source_ordinal NUMERIC(20,0) NOT NULL CHECK(source_ordinal>0),
 before_item JSONB NOT NULL,
 outputs JSONB NOT NULL CHECK(jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 1 AND 4),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);

CREATE FUNCTION game_item_craft_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; o JSONB;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR to_jsonb(i)<>NEW.before_item OR i.lifecycle<>1
    OR i.quantity<1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                    ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.source_ordinal)
 THEN RAISE EXCEPTION 'craft requires unchanged plain carried input' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.outputs))<>jsonb_array_length(NEW.outputs)
    OR (SELECT count(DISTINCT value->>'ordinal') FROM jsonb_array_elements(NEW.outputs))<>jsonb_array_length(NEW.outputs)
 THEN RAISE EXCEPTION 'duplicate craft output' USING ERRCODE='23514'; END IF;
 FOR o IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP
  IF NOT game_character_is_uuid_v7((o->>'item')::uuid)
     OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
     OR (o->>'count')::int NOT BETWEEN 1 AND 100
     OR octet_length(o->>'key') NOT BETWEEN 1 AND 512
     OR octet_length(o->>'revision') NOT BETWEEN 1 AND 512
     OR ((o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)
         AND NOT (i.quantity=1 AND o=NEW.outputs->0 AND (o->>'ordinal')::numeric=NEW.source_ordinal))
  THEN RAISE EXCEPTION 'invalid fresh craft output' USING ERRCODE='23514'; END IF;
 END LOOP;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE FUNCTION game_item_craft_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; i game_item_instances%ROWTYPE; o JSONB;
BEGIN
 SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR i.quantity<>(r.before_item->>'quantity')::bigint-1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=r.source_ordinal))
    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
    OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20
    OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id AND item_instance_id=r.source_item_instance_id
                   AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256 AND created_xact_id=pg_current_xact_id()
                   AND transaction_ordinal=1 AND transaction_count=1 AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'incomplete atomic craft' USING ERRCODE='23514'; END IF;
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)
     WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
       AND n.quantity=(o->>'count')::bigint AND n.definition_family='Item' AND n.definition_production_key=o->>'key'
       AND n.definition_revision_ref=o->>'revision' AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL
       AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
       AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id)
  THEN RAISE EXCEPTION 'craft output or custody mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_item_craft_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_item_craft_complete(NEW.transaction_id); RETURN NULL; END $$;

-- Add only craft proof arms; preserve every prior mechanic's guard body verbatim.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
     'game_item_container_entry_removal_proven','game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_craft_records r WHERE r.source_item_instance_id=OLD.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>OLD.quantity-1 OR NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
       OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'craft decrements exactly one input unit' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_craft_records r WHERE r.source_item_instance_id=NEW.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_craft_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i
       ON i.item_instance_id=r.source_item_instance_id WHERE r.source_item_instance_id=OLD.item_instance_id
       AND r.character_id=OLD.character_id AND r.parent_item_instance_id=OLD.parent_item_instance_id
       AND r.source_ordinal=OLD.placement_ordinal AND r.world_id=OLD.world_id
       AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
       AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_placement_proven' THEN
   arm:=$arm$
    IF TG_TABLE_NAME='game_item_container_entries' AND EXISTS(SELECT 1 FROM game_item_craft_records r,
       LATERAL jsonb_array_elements(r.outputs) o WHERE (o->>'item')::uuid=NEW.item_instance_id
       AND (o->>'ordinal')::numeric=NEW.placement_ordinal AND r.transaction_id=NEW.placed_transaction_id
       AND r.world_id=NEW.world_id AND r.character_id=NEW.character_id AND r.parent_item_instance_id=NEW.parent_item_instance_id
       AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_craft_records r, LATERAL jsonb_array_elements(r.outputs) o
       WHERE (o->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.minted_transaction_id
         AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_craft_complete(NEW.minted_transaction_id); RETURN NULL;
    END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
CREATE TRIGGER game_item_craft_admit BEFORE INSERT ON game_item_craft_records FOR EACH ROW EXECUTE FUNCTION game_item_craft_admit();
CREATE CONSTRAINT TRIGGER game_item_craft_consistent AFTER INSERT ON game_item_craft_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_craft_consistent();
CREATE TRIGGER game_item_craft_immutable BEFORE UPDATE OR DELETE ON game_item_craft_records FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_craft_no_truncate BEFORE TRUNCATE ON game_item_craft_records EXECUTE FUNCTION game_item_reject_truncate();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_craft_admit','game_item_craft_consistent'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_item_craft_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_item_craft_records FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_craft_admit(),game_item_craft_complete(UUID),game_item_craft_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_craft_records TO oteryn_game_runtime;
GRANT SELECT ON game_item_craft_records TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_item_craft_complete(UUID) TO oteryn_game_runtime;
