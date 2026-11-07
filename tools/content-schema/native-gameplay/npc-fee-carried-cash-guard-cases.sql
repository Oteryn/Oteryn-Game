DO $npc_cash$
DECLARE caught BOOLEAN;fee UUID:='01900000-0000-7000-8000-000000000001';
 actor UUID:='01900000-0000-7000-8000-000000000041';world UUID:='01900000-0000-7000-8000-000000000042';
BEGIN
 IF game_npc_cash_context_current(fee,actor,world) IS DISTINCT FROM FALSE THEN
  RAISE EXCEPTION 'no sealed source must not authorize cash';END IF;
 IF game_npc_cash_source_complete(fee,actor,world) IS DISTINCT FROM FALSE THEN
  RAISE EXCEPTION 'no immutable service receipt must not complete cash';END IF;
 caught:=FALSE;BEGIN
  INSERT INTO game_npc_fee_cash_records(transaction_id,character_id,world_id,backpack_item_instance_id,backpack_capacity,backpack_ordinals,inputs)
   VALUES(fee,actor,world,NULL,0,'[]','[]');
 EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'unarmed empty cash proof admitted';END IF;
 caught:=FALSE;BEGIN PERFORM game_npc_cash_complete(fee);
 EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'no fee/service/root proof completed';END IF;
 IF EXISTS(SELECT 1 FROM game_npc_cash_prior_equipment(fee,actor,world)) THEN
  RAISE EXCEPTION 'missing linked cost must not provide a forged original equipment epoch';END IF;
 RAISE NOTICE 'SRV7_NPC_CASH_SECURITY_PASSED cases=5';
END $npc_cash$;
