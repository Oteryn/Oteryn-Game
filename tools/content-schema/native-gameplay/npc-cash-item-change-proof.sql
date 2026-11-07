-- New closed fee473/474 owns its exact current-XID physical coin mutation.
-- Every older Item/House/Gem proof arm remains byte-for-byte below this arm.
DO $npc_cash_item_change$
DECLARE definition TEXT;at INTEGER;arm TEXT;
BEGIN
 definition:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 IF strpos(definition,'SRV7_CLOSED_NPC_COIN_CHANGE')>0 THEN RAISE EXCEPTION 'NPC coin change proof already installed';END IF;
 arm:=$arm$
 -- SRV7_CLOSED_NPC_COIN_CHANGE: choose by the closed cause, never by a
 -- nullable proof predicate that could fall through to another owner.
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
$arm$;
 at:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'NPC item change boundary changed';END IF;
 EXECUTE overlay(definition placing E'BEGIN\n'||arm from at for 5);
END $npc_cash_item_change$;
