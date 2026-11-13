-- M49: direct corpse loot retires child-first in the physical altar transaction.
-- Each entry remains a separate one-item logical transaction and audit envelope.
CREATE TABLE game_elemental_loot_retire_receipts (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances,
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 arrival_transaction_id UUID NOT NULL REFERENCES game_elemental_arrival_receipts(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 item_before JSONB NOT NULL CHECK(jsonb_typeof(item_before)='object' AND octet_length(item_before::text)<=8192),
 entry_before JSONB NOT NULL CHECK(jsonb_typeof(entry_before)='object' AND octet_length(entry_before::text)<=8192),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(item_instance_id<>parent_item_instance_id AND transaction_id<>arrival_transaction_id)
);
CREATE INDEX game_elemental_loot_retire_arrival ON game_elemental_loot_retire_receipts(arrival_transaction_id);
CREATE INDEX game_elemental_loot_retire_parent ON game_elemental_loot_retire_receipts(parent_item_instance_id);
CREATE FUNCTION game_elemental_loot_retire_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; e game_item_corpse_container_entries%ROWTYPE;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 SELECT * INTO e FROM game_item_corpse_container_entries WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR e.item_instance_id IS NULL OR i.lifecycle<>1 OR i.quantity<1
   OR NEW.item_before IS DISTINCT FROM to_jsonb(i) OR NEW.entry_before IS DISTINCT FROM to_jsonb(e)
   OR e.parent_item_instance_id<>NEW.parent_item_instance_id
   OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id OR parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id OR parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_bindings WHERE item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'elemental loot retirement requires an exact plain direct corpse entry' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_elemental_loot_retire_complete(p_item UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_elemental_loot_retire_receipts%ROWTYPE; a game_elemental_arrival_receipts%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_elemental_loot_retire_receipts WHERE item_instance_id=p_item;
 SELECT * INTO a FROM game_elemental_arrival_receipts WHERE transaction_id=r.arrival_transaction_id;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=p_item;
 IF r.item_instance_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
   OR a.transaction_id IS NULL OR a.created_xact_id<>pg_current_xact_id() OR a.corpse_item_instance_id<>r.parent_item_instance_id
   OR r.occurred_at<>a.occurred_at OR i.world_id<>(a.corpse_before->'item'->>'world_id')::uuid
   OR i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id<>r.transaction_id
   OR i.state_revision<>(r.item_before->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
      (r.item_before-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item)
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.transaction_id=r.transaction_id AND o.event_id=r.event_id
      AND o.item_instance_id=p_item AND o.envelope_sha256=r.envelope_sha256 AND o.occurred_at=r.occurred_at
      AND o.transaction_ordinal=1 AND o.transaction_count=1 AND o.created_xact_id=pg_current_xact_id()
      AND o.publication_state=1 AND o.published_at IS NULL)
 THEN RAISE EXCEPTION 'elemental loot retirement requires its completed parent arrival and own audit' USING ERRCODE='23514'; END IF;
 PERFORM game_elemental_arrival_complete(a.transaction_id);
END $$;
CREATE FUNCTION game_elemental_loot_retire_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_elemental_loot_retire_complete(NEW.item_instance_id); RETURN NULL; END $$;
CREATE TRIGGER game_elemental_loot_retire_admit BEFORE INSERT ON game_elemental_loot_retire_receipts
 FOR EACH ROW EXECUTE FUNCTION game_elemental_loot_retire_admit();
CREATE CONSTRAINT TRIGGER game_elemental_loot_retire_consistent AFTER INSERT ON game_elemental_loot_retire_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_loot_retire_consistent();
CREATE TRIGGER game_elemental_loot_retire_immutable BEFORE UPDATE OR DELETE ON game_elemental_loot_retire_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_elemental_loot_retire_no_truncate BEFORE TRUNCATE ON game_elemental_loot_retire_receipts
 EXECUTE FUNCTION game_item_reject_truncate();
-- Preserve the full predecessor guard chains, admitting only this new exact proof.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 -- Item instance retirement admission is composed after accepted audit guards by migration 2815.
 FOREACH name IN ARRAY ARRAY['game_item_instance_change_proven','game_item_corpse_entry_removal_proven'] LOOP
   body:=pg_get_functiondef((name||'()')::regprocedure);
   at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing elemental loot predecessor %',name; END IF;
   IF name='game_item_instance_change_proven' THEN
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_loot_retire_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.item_before=to_jsonb(OLD)) THEN
       PERFORM game_elemental_loot_retire_complete(NEW.item_instance_id); RETURN NULL;
     END IF;
     $arm$;
   ELSE
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_loot_retire_receipts r WHERE r.created_xact_id=pg_current_xact_id()
       AND r.entry_before=to_jsonb(OLD)) THEN
       PERFORM game_elemental_loot_retire_complete(OLD.item_instance_id); RETURN NULL;
     END IF;
     $arm$;
   END IF;
   EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
END $migration$;
CREATE FUNCTION game_elemental_arrival_retire_loot(p_arrival UUID,p_parent UUID,p_item UUID,p_transaction UUID,p_event UUID,
 p_occurred BIGINT,p_envelope BYTEA,p_schema BIGINT,p_retention TEXT) RETURNS void LANGUAGE plpgsql AS $$
DECLARE i JSONB; e JSONB;
BEGIN
 SET CONSTRAINTS ALL DEFERRED;
 SELECT to_jsonb(v),to_jsonb(c) INTO i,e FROM game_item_instances v JOIN game_item_corpse_container_entries c USING(item_instance_id)
   WHERE v.item_instance_id=p_item AND c.parent_item_instance_id=p_parent FOR UPDATE OF v,c;
 IF i IS NULL THEN RAISE EXCEPTION 'elemental loot entry is stale' USING ERRCODE='23514'; END IF;
 INSERT INTO game_elemental_loot_retire_receipts(item_instance_id,transaction_id,event_id,arrival_transaction_id,parent_item_instance_id,
    item_before,entry_before,occurred_at,envelope_sha256)
 VALUES(p_item,p_transaction,p_event,p_arrival,p_parent,i,e,p_occurred,sha256(p_envelope));
 UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=p_transaction WHERE item_instance_id=p_item;
 DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=p_item;
 INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,
   retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state)
 VALUES(p_event,p_transaction,1,1,2,p_schema,p_retention,p_item,p_occurred,p_occurred+7776000000,p_envelope,sha256(p_envelope),1);
END $$;
DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_elemental_loot_retire_admit()','game_elemental_loot_retire_complete(uuid)',
  'game_elemental_loot_retire_consistent()','game_elemental_arrival_retire_loot(uuid,uuid,uuid,uuid,uuid,bigint,bytea,bigint,text)'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
REVOKE ALL ON game_elemental_loot_retire_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_elemental_loot_retire_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_elemental_loot_retire_receipts TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_elemental_loot_retire_complete(UUID),game_elemental_arrival_retire_loot(UUID,UUID,UUID,UUID,UUID,BIGINT,BYTEA,BIGINT,TEXT) TO oteryn_game_runtime;
