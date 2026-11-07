-- NPC cash keeps all existing purchase-cause shapes. New typed causes have
-- their own exact service/price shape plus the existing reciprocal source guards.
CREATE INDEX game_npc_source_policies_fee_service_price
 ON game_npc_source_policies(fee_service_key,fee_gold_units)
 WHERE fee_service_key IS NOT NULL;
CREATE FUNCTION game_npc_source_fee_purchase_shape(p_service TEXT,p_gold BIGINT)
 RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER
 SET search_path=pg_catalog,public,pg_temp AS $$
 SELECT p_service IS NOT NULL AND p_gold>0 AND EXISTS(
  SELECT 1 FROM game_npc_source_policies source_policy
  WHERE source_policy.fee_service_key=p_service AND source_policy.fee_gold_units=p_gold)
$$;
REVOKE ALL ON FUNCTION game_npc_source_fee_purchase_shape(TEXT,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_source_fee_purchase_shape(TEXT,BIGINT) TO oteryn_game_runtime;
DO $npc_purchase_causes$
DECLARE original_expression TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT original_expression
 FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass
 AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK('
 ||'CASE cause_kind WHEN 473 THEN (charm_key IS NULL AND '
 ||'(purchase_service_key=''oteryn:service.horse-station.rental'') IS TRUE AND fee_gold_units=500) '
 ||'WHEN 474 THEN (charm_key IS NULL AND game_npc_source_fee_purchase_shape(purchase_service_key,fee_gold_units) IS TRUE) '
 ||'ELSE ('||original_expression||') END)';
END $npc_purchase_causes$;
