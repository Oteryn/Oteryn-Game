-- Hidden amount is the immutable original source quantity, not mutable attributes.
CREATE FUNCTION game_house_furniture_wrap_quantity(p_transaction UUID) RETURNS NUMERIC
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE wrap_receipt game_house_furniture_wrap_receipts%ROWTYPE;
 source_wrap game_house_furniture_wrap_receipts%ROWTYPE; source_amount NUMERIC;
BEGIN
 SELECT * INTO wrap_receipt FROM game_house_furniture_wrap_receipts WHERE transaction_id=p_transaction;
 IF NOT FOUND THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 source_amount:=(wrap_receipt.before_item->>'quantity')::numeric;
 IF source_amount IS NULL OR source_amount<1 OR source_amount>100 OR source_amount<>trunc(source_amount) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 IF wrap_receipt.action=1 THEN
  IF source_amount<>1 AND NOT coalesce((octet_length(wrap_receipt.use_intent)=13 AND get_byte(wrap_receipt.use_intent,0)=4
   AND wrap_receipt.before_item->>'definition_family'='Item'
   AND wrap_receipt.before_item->>'definition_revision_ref'='definition-r1'
   AND EXISTS(SELECT 1 FROM game_house_furniture_portable_wrap_sources wrap_policy
    WHERE wrap_receipt.before_item->>'definition_production_key'='oteryn:item.tibia.i'||wrap_policy.item_id::text
     AND source_amount BETWEEN 2 AND wrap_policy.stack_max AND source_amount=trunc(source_amount))),FALSE)
   THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
  RETURN 1;
 END IF;
 IF source_amount<>1 THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 IF wrap_receipt.before_item->>'definition_production_key'<>'oteryn:item.tibia.i23398' THEN RETURN 1; END IF;
 SELECT prior_wrap.* INTO source_wrap FROM game_house_furniture_wrap_receipts prior_wrap
 WHERE prior_wrap.item_instance_id=wrap_receipt.item_instance_id AND prior_wrap.world_id=wrap_receipt.world_id
  AND prior_wrap.action=1 AND prior_wrap.after_key=wrap_receipt.before_item->>'definition_production_key'
  AND (prior_wrap.before_item->>'state_revision')::numeric<(wrap_receipt.before_item->>'state_revision')::numeric
 ORDER BY (prior_wrap.before_item->>'state_revision')::numeric DESC LIMIT 1;
 IF NOT FOUND OR (source_wrap.before_item->>'quantity')::numeric=1 THEN RETURN 1; END IF;
 source_amount:=(source_wrap.before_item->>'quantity')::numeric;
 IF octet_length(source_wrap.use_intent) IS DISTINCT FROM 13 OR get_byte(source_wrap.use_intent,0) IS DISTINCT FROM 4
  OR source_wrap.before_item->>'definition_production_key' IS DISTINCT FROM wrap_receipt.after_key
  OR NOT EXISTS(SELECT 1 FROM game_house_furniture_portable_wrap_sources wrap_policy
   WHERE wrap_receipt.after_key='oteryn:item.tibia.i'||wrap_policy.item_id::text
    AND source_amount BETWEEN 2 AND wrap_policy.stack_max AND source_amount=trunc(source_amount))
 THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 RETURN source_amount;
END $$;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_quantity(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_quantity(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Change only the exact immutable Wrap arm of the existing Item guard.
DO $guard$ DECLARE body TEXT; old_arm TEXT; new_arm TEXT; BEGIN
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 old_arm:=$old$IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.item_instance_id=OLD.item_instance_id
     AND r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.state_revision<>OLD.state_revision+1 OR NEW.lifecycle<>1 OR NEW.quantity<>OLD.quantity
      OR (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
       (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
      THEN RAISE EXCEPTION 'wrap preserves item lineage' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;$old$;
 IF position(old_arm IN body)=0 THEN RAISE EXCEPTION 'missing exact immutable Wrap Item arm'; END IF;
 new_arm:=replace(old_arm,'NEW.quantity<>OLD.quantity','NEW.quantity IS DISTINCT FROM game_house_furniture_wrap_quantity(NEW.last_transaction_id)');
 new_arm:=replace(new_arm,'''definition_production_key'',''last_transaction_id'',''state_revision''','''definition_production_key'',''last_transaction_id'',''state_revision'',''quantity''');
 EXECUTE replace(body,old_arm,new_arm);
 body:=pg_get_functiondef('game_house_furniture_wrap_complete(uuid)'::regprocedure);
 old_arm:=$old$OR i.definition_production_key IS DISTINCT FROM r.after_key$old$;
 IF position(old_arm IN body)=0 THEN RAISE EXCEPTION 'missing exact Wrap complete definition proof'; END IF;
 body:=replace(body,old_arm,old_arm||' OR i.quantity IS DISTINCT FROM game_house_furniture_wrap_quantity(p_tx)');
 body:=replace(body,'''definition_production_key'',''last_transaction_id'',''state_revision''','''definition_production_key'',''last_transaction_id'',''state_revision'',''quantity''');
 EXECUTE body;
END $guard$;
