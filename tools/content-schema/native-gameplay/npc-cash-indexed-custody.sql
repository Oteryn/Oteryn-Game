-- Closed NPC coin custody indexes a successful full proof once per statement.
-- All29 existing footprint invalidations delete the parent memo and cascade
-- these tuples. These rows grant no authority without the fresh full-proof call.
CREATE TABLE game_npc_cash_verified_inputs(
 backend_pid INTEGER NOT NULL,fee_transaction_id UUID NOT NULL,item_instance_id UUID NOT NULL,
 transaction_xid XID8 NOT NULL,world_id UUID NOT NULL,parent_item_instance_id UUID,
 placement_ordinal NUMERIC NOT NULL,surface SMALLINT NOT NULL CHECK(surface IN(1,2,3)),
 slot SMALLINT NOT NULL,quantity_before BIGINT NOT NULL CHECK(quantity_before>0),
 burned BIGINT NOT NULL CHECK(burned>=0 AND burned<=quantity_before),
 coin_key TEXT NOT NULL,coin_worth BIGINT NOT NULL CHECK(coin_worth IN(1,100,10000)),
 item_before JSONB NOT NULL,item_after JSONB NOT NULL,
 PRIMARY KEY(backend_pid,fee_transaction_id,item_instance_id),
 FOREIGN KEY(backend_pid,fee_transaction_id) REFERENCES game_npc_cash_verified(backend_pid,fee_transaction_id) ON DELETE CASCADE
);
REVOKE ALL ON game_npc_cash_verified_inputs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

DO $indexed_custody_capture$ DECLARE definition TEXT;marker TEXT;capture TEXT;BEGIN
 definition:=pg_get_functiondef('game_npc_cash_complete(uuid)'::regprocedure);
 marker:='SET transaction_xid=EXCLUDED.transaction_xid,statement_started=EXCLUDED.statement_started;';
 IF strpos(definition,marker)=0 OR strpos(definition,'game_npc_cash_verified_inputs')>0 THEN
  RAISE EXCEPTION 'NPC cash proof indexed-capture boundary changed';END IF;
 capture:=$capture$
 -- Index exact sealed custody facts only after every physical and source proof.
 DELETE FROM game_npc_cash_verified_inputs WHERE backend_pid=pg_backend_pid() AND fee_transaction_id=p_transaction;
 INSERT INTO game_npc_cash_verified_inputs(backend_pid,fee_transaction_id,item_instance_id,transaction_xid,world_id,
  parent_item_instance_id,placement_ordinal,surface,slot,quantity_before,burned,coin_key,coin_worth,item_before,item_after)
 SELECT pg_backend_pid(),p_transaction,(input->>'item')::uuid,pg_current_xact_id(),r.world_id,
  (input->>'parent')::uuid,(input->>'ordinal')::numeric,(input->>'surface')::smallint,(input->>'slot')::smallint,
  (input->'before'->>'quantity')::bigint,(input->>'burned')::bigint,input->'before'->>'definition_production_key',
  game_item_coin_worth(input->'before'->>'definition_production_key'),input->'before',
  (SELECT to_jsonb(actual) FROM game_item_instances actual WHERE actual.item_instance_id=(input->>'item')::uuid)
 FROM jsonb_array_elements(r.inputs) input;
$capture$;
 EXECUTE replace(definition,marker,marker||capture);
END $indexed_custody_capture$;

CREATE FUNCTION game_npc_cash_nested_delete_proven(p_item UUID,p_world UUID,p_parent UUID,p_ordinal NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE fee UUID;
BEGIN
 -- Select by actual closed fee cause, before any nullable proof predicate.
 SELECT f.transaction_id INTO fee FROM game_item_instances i JOIN game_item_fee_burns f ON f.transaction_id=i.last_transaction_id
 WHERE i.item_instance_id=p_item AND f.cause_kind IN(473,474) AND f.created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RAISE EXCEPTION 'NPC nested custody requires its exact closed current fee' USING ERRCODE='23514';END IF;
 -- This ALWAYS reproves the current source/parent/Root; a private memo only
 -- eliminates repeated physical scans after that independently fresh admission.
 PERFORM game_npc_cash_complete(fee);
 IF NOT EXISTS(SELECT 1 FROM game_npc_cash_verified_inputs proof
  JOIN game_npc_cash_verified memo USING(backend_pid,fee_transaction_id)
  JOIN game_item_instances actual ON actual.item_instance_id=proof.item_instance_id
  WHERE proof.backend_pid=pg_backend_pid() AND proof.fee_transaction_id=fee AND proof.item_instance_id=p_item
  AND proof.transaction_xid=pg_current_xact_id() AND memo.transaction_xid=pg_current_xact_id()
  AND memo.statement_started=statement_timestamp() AND proof.world_id=p_world AND actual.world_id=p_world
  AND proof.surface=2 AND proof.parent_item_instance_id=p_parent AND proof.placement_ordinal=p_ordinal
  AND proof.burned=proof.quantity_before AND proof.burned>0
  AND actual.definition_family='Item' AND actual.definition_production_key=proof.coin_key
  AND game_item_coin_worth(actual.definition_production_key)=proof.coin_worth
  AND actual.last_transaction_id=fee AND actual.quantity=0 AND actual.lifecycle=2) THEN
  RAISE EXCEPTION 'NPC nested delete lacks its exact sealed OLD custody, full burn and coin class' USING ERRCODE='23514';END IF;
END $$;
ALTER FUNCTION game_npc_cash_nested_delete_proven(uuid,uuid,uuid,numeric) SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_npc_cash_nested_delete_proven(uuid,uuid,uuid,numeric) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_cash_nested_delete_proven(uuid,uuid,uuid,numeric) TO oteryn_game_runtime;

DO $indexed_custody_guard$ DECLARE definition TEXT;old TEXT;replacement TEXT;BEGIN
 definition:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 old:=$old$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
  WHERE r.created_xact_id=pg_current_xact_id() AND r.world_id=OLD.world_id AND (l->>'surface')::int=2 AND (l->>'item')::uuid=OLD.item_instance_id
   AND (l->>'parent')::uuid=OLD.parent_item_instance_id AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND (l->>'burned')::bigint=(l->'before'->>'quantity')::bigint) THEN
  PERFORM game_npc_cash_complete(r.transaction_id) FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
   WHERE r.created_xact_id=pg_current_xact_id() AND (l->>'item')::uuid=OLD.item_instance_id; RETURN NULL;
 END IF;
$old$;
 replacement:=$replacement$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_fee_burns fee ON fee.transaction_id=i.last_transaction_id
  WHERE i.item_instance_id=OLD.item_instance_id AND fee.cause_kind IN(473,474) AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_nested_delete_proven(OLD.item_instance_id,OLD.world_id,OLD.parent_item_instance_id,OLD.placement_ordinal);
  RETURN NULL;
 END IF;
$replacement$;
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'NPC nested exact custody arm changed';END IF;
 EXECUTE replace(definition,old,replacement);
END $indexed_custody_guard$;

CREATE FUNCTION game_npc_cash_item_change_proven(p_before JSONB,p_after JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE fee UUID;
BEGIN
 SELECT f.transaction_id INTO fee FROM game_item_fee_burns f
 WHERE f.transaction_id=(p_after->>'last_transaction_id')::uuid AND f.cause_kind IN(473,474)
 AND f.created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RAISE EXCEPTION 'NPC item change requires its exact current closed fee' USING ERRCODE='23514';END IF;
 PERFORM game_npc_cash_complete(fee);
 IF NOT EXISTS(SELECT 1 FROM game_npc_cash_verified_inputs proof
  JOIN game_npc_cash_verified memo USING(backend_pid,fee_transaction_id)
  JOIN game_item_instances actual ON actual.item_instance_id=proof.item_instance_id
  WHERE proof.backend_pid=pg_backend_pid() AND proof.fee_transaction_id=fee
  AND proof.item_instance_id=(p_before->>'item_instance_id')::uuid
  AND proof.transaction_xid=pg_current_xact_id() AND memo.transaction_xid=pg_current_xact_id()
  AND memo.statement_started=statement_timestamp() AND proof.burned>0
  AND proof.item_before=p_before AND proof.item_after=p_after AND to_jsonb(actual)=p_after
  AND actual.world_id=proof.world_id AND actual.definition_family='Item'
  AND actual.definition_production_key=proof.coin_key AND game_item_coin_worth(actual.definition_production_key)=proof.coin_worth
  AND actual.last_transaction_id=fee) THEN
  RAISE EXCEPTION 'NPC item change lacks its exact complete sealed OLD/NEW/class/world snapshot' USING ERRCODE='23514';END IF;
END $$;
ALTER FUNCTION game_npc_cash_item_change_proven(jsonb,jsonb) SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_npc_cash_item_change_proven(jsonb,jsonb) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_cash_item_change_proven(jsonb,jsonb) TO oteryn_game_runtime;

-- Replace only the already closed NPC arm. All recovery/exclusivity and all
-- older mutation owners remain untouched; whole Row OLD/NEW equality is retained.
DO $indexed_item_change$ DECLARE definition TEXT;old TEXT;replacement TEXT;BEGIN
 definition:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 old:=$old$
 IF EXISTS(SELECT 1 FROM game_item_fee_burns fee
  WHERE fee.transaction_id=NEW.last_transaction_id AND fee.cause_kind IN(473,474)
   AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_complete(NEW.last_transaction_id);
  IF NOT EXISTS(SELECT 1 FROM game_npc_fee_cash_records cash,
    LATERAL jsonb_array_elements(cash.inputs) line
   WHERE cash.transaction_id=NEW.last_transaction_id AND cash.created_xact_id=pg_current_xact_id()
    AND line->>'item'=OLD.item_instance_id::text AND line->'before'=to_jsonb(OLD)
    AND (line->>'burned')::bigint>0
    AND NEW.item_instance_id=OLD.item_instance_id AND NEW.world_id=OLD.world_id
    AND NEW.quantity=OLD.quantity-(line->>'burned')::bigint
    AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END
    AND NEW.state_revision=OLD.state_revision+1
    AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
       =(to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
    AND EXISTS(SELECT 1 FROM game_item_instances actual
      WHERE actual.item_instance_id=NEW.item_instance_id AND to_jsonb(actual)=to_jsonb(NEW))) THEN
   RAISE EXCEPTION 'NPC coin update lacks its exact sealed before/final successor' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
$old$;
 replacement:=$replacement$
 IF EXISTS(SELECT 1 FROM game_item_fee_burns fee
  WHERE fee.transaction_id=NEW.last_transaction_id AND fee.cause_kind IN(473,474)
   AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_item_change_proven(to_jsonb(OLD),to_jsonb(NEW));
  RETURN NULL;
 END IF;
$replacement$;
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'NPC exact item before/after arm changed';END IF;
 EXECUTE replace(definition,old,replacement);
END $indexed_item_change$;
