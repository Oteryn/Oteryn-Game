-- T135, Canary 99902524 house.cpp:308-360,440-453; game.cpp:5593-5621.
CREATE TABLE game_house_disposition_wrap_records (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 operation_id UUID NOT NULL REFERENCES game_house_dispositions,
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 before_item JSONB NOT NULL, after_item JSONB NOT NULL,
 after_key TEXT NOT NULL CHECK(after_key='oteryn:item.tibia.i23398'),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(operation_id,item_instance_id)
);
CREATE INDEX game_house_disposition_wrap_item ON game_house_disposition_wrap_records(item_instance_id);
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON game_house_disposition_wrap_records
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER no_truncate BEFORE TRUNCATE ON game_house_disposition_wrap_records
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_house_disposition_wrap_records FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_disposition_wrap_records TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_disposition_wrap_key(i UUID,k TEXT) RETURNS TEXT
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN k<>'oteryn:item.tibia.i23398'
  AND game_house_furniture_wrap_pair(i,k,'oteryn:item.tibia.i23398',1::smallint)
  THEN 'oteryn:item.tibia.i23398' END
$$;
CREATE FUNCTION game_house_disposition_item_unchanged(op UUID,i UUID,old_item JSONB,actual JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT actual=old_item OR EXISTS(SELECT 1 FROM game_house_disposition_wrap_records r
  WHERE r.operation_id=op AND r.item_instance_id=i AND r.before_item=old_item AND r.after_item=actual)
$$;
CREATE FUNCTION game_house_disposition_wrap_complete(tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_disposition_wrap_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_disposition_wrap_records WHERE transaction_id=tx;
 IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id()
  OR r.after_key IS DISTINCT FROM game_house_disposition_wrap_key(r.item_instance_id,r.before_item->>'definition_production_key')
  OR r.before_item->>'definition_family'<>'Item' OR r.before_item->>'definition_revision_ref'<>'definition-r1'
  OR r.before_item->>'quantity'<>'1' OR r.before_item->>'lifecycle'<>'1'
  OR r.after_item IS DISTINCT FROM r.before_item||jsonb_build_object('definition_production_key',r.after_key,
   'last_transaction_id',tx,'state_revision',(r.before_item->>'state_revision')::numeric+1)
  OR NOT EXISTS(SELECT 1 FROM game_item_instances actual WHERE to_jsonb(actual)=r.after_item)
  OR NOT EXISTS(SELECT 1 FROM game_house_disposition_steps s JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
   WHERE s.operation_id=r.operation_id AND s.item_instance_id=r.item_instance_id
    AND s.written_xact_id=pg_current_xact_id() AND c.source_item=r.before_item AND c.tree_count=1
    AND game_house_disposition_delivery_proven(s.operation_id,s.item_instance_id))
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=r.item_instance_id)
  OR NOT EXISTS(SELECT 1 FROM game_house_disposition_audit_outbox a WHERE a.operation_id=r.operation_id
   AND a.written_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'T135 furniture conversion lacks its exact disposition and delivery' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_house_disposition_wrap_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_house_disposition_wrap_complete(NEW.transaction_id);
 IF game_house_timed_decoration_wrap_pair(NEW.before_item->>'definition_production_key',NEW.after_key,1::smallint)
 THEN PERFORM game_house_timed_decoration_wrap_complete(NEW.transaction_id); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_disposition_wrap_proven AFTER INSERT ON game_house_disposition_wrap_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_wrap_proven();

-- This is history, not a Player-session authority. Session/command/intent fields
-- are deliberately NULL for World-job records. Only metadata/timer readers use
-- this union; all player mutation and replay fences remain in their original writer.
CREATE VIEW game_house_furniture_wrap_evidence AS
 SELECT * FROM game_house_furniture_wrap_receipts
 UNION ALL
 SELECT r.transaction_id,NULL::uuid,NULL::numeric,c.reclaim_subject,r.item_instance_id,
  o.world_id,o.house_key,NULL::bytea,1::smallint,r.before_item,r.after_key,c.source_location,
  NULL::jsonb,r.created_xact_id
 FROM game_house_disposition_wrap_records r JOIN game_house_dispositions o USING(operation_id)
 JOIN game_house_disposition_census c USING(operation_id,item_instance_id);
REVOKE ALL ON game_house_furniture_wrap_evidence FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_wrap_evidence TO oteryn_game_runtime;

ALTER FUNCTION game_house_furniture_wrap_complete(UUID) RENAME TO game_house_furniture_wrap_complete_before_t135;
CREATE FUNCTION game_house_furniture_wrap_complete(tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_house_disposition_wrap_records WHERE transaction_id=tx)
 THEN PERFORM game_house_disposition_wrap_complete(tx);
 ELSE PERFORM game_house_furniture_wrap_complete_before_t135(tx); END IF;
END $$;

CREATE FUNCTION game_house_disposition_wrap(op UUID,i UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE original game_item_instances%ROWTYPE; c game_house_disposition_census%ROWTYPE;
 o game_house_dispositions%ROWTYPE; target TEXT;tx UUID;after_value JSONB;
BEGIN
 SELECT * INTO original FROM game_item_instances WHERE item_instance_id=i FOR UPDATE;
 target:=game_house_disposition_wrap_key(i,original.definition_production_key);
 IF target IS NULL THEN RETURN; END IF;
 SELECT * INTO c FROM game_house_disposition_census WHERE operation_id=op AND item_instance_id=i;
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op;
 IF o.state<>2 OR c.source_item IS DISTINCT FROM to_jsonb(original) OR c.tree_count<>1
  OR original.quantity<>1 OR original.lifecycle<>1 OR original.definition_family<>'Item'
  OR original.definition_revision_ref<>'definition-r1'
  OR NOT EXISTS(SELECT 1 FROM game_house_disposition_steps WHERE operation_id=op AND item_instance_id=i
    AND written_xact_id=pg_current_xact_id())
  OR NOT EXISTS(SELECT 1 FROM game_house_ownership a WHERE a.world_id=o.world_id AND a.house_key=o.house_key
    AND a.ownership_revision=o.ownership_revision AND a.owner_character_id IS NOT DISTINCT FROM o.owner_character_id
    AND a.owner_guild_id IS NOT DISTINCT FROM o.owner_guild_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i)
 THEN RAISE EXCEPTION 'T135 wrapping requires the current empty furniture source' USING ERRCODE='23514'; END IF;
 tx:=game_character_uuid_v7();
 after_value:=to_jsonb(original)||jsonb_build_object('definition_production_key',target,'last_transaction_id',tx,
  'state_revision',original.state_revision+1);
 INSERT INTO game_house_disposition_wrap_records VALUES(tx,op,i,to_jsonb(original),after_value,target,pg_current_xact_id());
 UPDATE game_item_instances SET definition_production_key=target,last_transaction_id=tx,state_revision=state_revision+1
  WHERE item_instance_id=i;
 IF game_house_timed_decoration_wrap_pair(original.definition_production_key,target,1::smallint)
 THEN PERFORM game_house_timed_decoration_wrap_apply(tx); END IF;
END $$;

DO $patch$
DECLARE body TEXT;old TEXT;n TEXT;arm TEXT;
BEGIN
 -- A wrapped container's children are their own delivery roots. Ordinary bags
 -- retain their full tree; the existing deepest-first batching empties furniture
 -- before its conversion, preserving every child and explicit item owner.
 body:=pg_get_functiondef('game_house_disposition_capture_container_trees(UUID,UUID,TEXT)'::regprocedure);
 old:='p.reclaim_subject_character_id=walk.subject';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing T135 container split seam'; END IF;
 EXECUTE replace(body,old,old||' AND game_house_disposition_wrap_key(p.item_instance_id,p.source_item->>''definition_production_key'') IS NULL');
 body:=pg_get_functiondef('game_house_disposition_advance(UUID,INT,INT,INT)'::regprocedure);
 old:='INSERT INTO game_house_disposition_steps VALUES(op,x.item_instance_id,x.reclaim_subject,ord,requested_step,pg_current_xact_id());';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing T135 conversion seam'; END IF;
 EXECUTE replace(body,old,old||E'\n  PERFORM game_house_disposition_wrap(op,x.item_instance_id);');
 body:=pg_get_functiondef('game_house_disposition_tree_delivery_complete(UUID,UUID)'::regprocedure);
 old:='to_jsonb(actual) IS DISTINCT FROM m.source_item';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing T135 tree value proof seam'; END IF;
 EXECUTE replace(body,old,'NOT game_house_disposition_item_unchanged(op,t.item,m.source_item,to_jsonb(actual))');

 -- Extend only exact history readers, never INSERT INTO the player receipt table.
 FOREACH n IN ARRAY ARRAY['game_house_furniture_wrap_pair(uuid,text,text,smallint)',
  'game_house_timed_decoration_wrap_complete(uuid)','game_house_timed_decoration_wrap_apply(uuid)',
  'game_house_decoration_timed_deadline_stamp()','game_item_timed_state_write_consistency_guard()',
  'game_house_furniture_catalogue(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)',
  'game_house_furniture_wrap(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,numeric,uuid,numeric,numeric,smallint,bytea,bytea,text,integer,text)'] LOOP
  body:=pg_get_functiondef(n::regprocedure);
  EXECUTE replace(body,'FROM game_house_furniture_wrap_receipts','FROM game_house_furniture_wrap_evidence');
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_house_disposition_item_fence'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_house_disposition_wrap_records r WHERE r.transaction_id=NEW.last_transaction_id
     AND r.item_instance_id=NEW.item_instance_id AND r.after_item=to_jsonb(NEW) AND r.created_xact_id=pg_current_xact_id())
    THEN PERFORM game_house_disposition_wrap_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_disposition_wrap_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
      AND r.before_item=to_jsonb(OLD) AND r.after_item=to_jsonb(NEW) AND r.created_xact_id=pg_current_xact_id())
    THEN RETURN NEW; END IF;
$arm$;
  END IF;
  IF strpos(body,'BEGIN')=0 THEN RAISE EXCEPTION 'missing T135 item proof seam'; END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM strpos(body,'BEGIN') FOR 5);
 END LOOP;
END $patch$;

ALTER FUNCTION game_house_disposition_native_removal_proven(JSONB)
 RENAME TO game_house_disposition_native_removal_proven_before_t135;
CREATE FUNCTION game_house_disposition_native_removal_proven(old_location JSONB) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT game_house_disposition_native_removal_proven_before_t135(old_location) OR EXISTS(
  SELECT 1 FROM game_house_disposition_wrap_records r
  JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
  JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  JOIN game_item_instances actual USING(item_instance_id)
  WHERE c.source_location=old_location AND c.source_item=r.before_item
   AND to_jsonb(actual)=r.after_item AND s.written_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND game_house_disposition_delivery_proven(c.operation_id,c.item_instance_id))
$$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_disposition_wrap_key(UUID,TEXT)'::regprocedure,
  'game_house_disposition_item_unchanged(UUID,UUID,JSONB,JSONB)'::regprocedure,
  'game_house_disposition_wrap_complete(UUID)'::regprocedure,
  'game_house_disposition_wrap_proven()'::regprocedure,
  'game_house_disposition_wrap(UUID,UUID)'::regprocedure,
  'game_house_furniture_wrap_complete(UUID)'::regprocedure,
  'game_house_disposition_native_removal_proven(JSONB)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_complete_before_t135(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_wrap_complete(UUID) TO oteryn_game_runtime;
