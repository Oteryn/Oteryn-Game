-- Rotate retargets the original absolute Item schedule; its source and deadline
-- remain unchanged. The complete old row is retained in the immutable action.
CREATE FUNCTION game_house_furniture_absolute_proposal(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_spell_item_temporal_schedules%ROWTYPE; life game_house_furniture_lifetimes%ROWTYPE;
BEGIN
 SELECT * INTO p FROM game_spell_item_temporal_schedules WHERE item_instance_id=r.source_item_instance_id;
 IF NOT (r.effects ? 'house_absolute') THEN RETURN p.item_instance_id IS NULL
  OR EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts done WHERE done.item_instance_id=p.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries done WHERE done.item_instance_id=p.item_instance_id); END IF;
 SELECT * INTO life FROM game_house_furniture_lifetimes
  WHERE 'oteryn:item.tibia.i'||item_id::text=coalesce(r.effects->'rotate_after',r.effects->'house_after')->>'key';
 RETURN r.effects->>'house_rotate'='true' AND p.item_instance_id IS NOT NULL AND to_jsonb(p)=r.effects->'house_absolute'->'before'
  AND NOT (r.effects ? 'house_temporal')
  AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=p.item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts WHERE item_instance_id=p.item_instance_id)
  AND p.expires_at_unix_ms>r.occurred_at AND p.duration_millis=life.duration_ms
  AND (p.definition_family,p.definition_production_key,p.definition_revision)=
   ('Item',r.before_item->>'definition_production_key',r.before_item->>'definition_revision_ref')
  AND life.target_id IS NOT NULL AND p.target_family='Item'
  AND p.target_revision=p.definition_revision
  AND r.effects->'house_absolute'->>'target'='oteryn:item.tibia.i'||life.target_id::text;
END $$;
CREATE FUNCTION game_house_furniture_absolute_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 IF TG_OP<>'UPDATE' THEN RAISE EXCEPTION 'absolute Item source history is immutable' USING ERRCODE='23514'; END IF;
 SELECT * INTO r FROM game_item_consume_transform_records
  WHERE source_item_instance_id=OLD.item_instance_id AND created_xact_id=pg_current_xact_id()
   AND ((source_kind IN(1,4) AND effects->>'kind'='furniture_rotation')
    OR (source_kind=5 AND effects->>'kind'='house_transform' AND effects->>'house_rotate'='true'))
   AND effects->'house_absolute'->'before'=to_jsonb(OLD);
 IF r.transaction_id IS NULL OR r.effects ? 'house_temporal'
  OR to_jsonb(NEW) IS DISTINCT FROM to_jsonb(OLD)||jsonb_build_object(
   'definition_production_key',coalesce(r.effects->'rotate_after',r.effects->'house_after')->>'key',
   'target_production_key',r.effects->'house_absolute'->>'target')
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=OLD.item_instance_id
    AND i.last_transaction_id=r.transaction_id AND i.definition_production_key=NEW.definition_production_key
    AND i.state_revision=(r.before_item->>'state_revision')::numeric+1)
 THEN RAISE EXCEPTION 'absolute Rotate must preserve original source, deadline and exact receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
DROP TRIGGER game_spell_item_temporal_schedules_immutable ON game_spell_item_temporal_schedules;
CREATE TRIGGER game_spell_item_temporal_schedules_immutable BEFORE UPDATE OR DELETE ON game_spell_item_temporal_schedules
 FOR EACH ROW EXECUTE FUNCTION game_house_furniture_absolute_guard();
CREATE FUNCTION game_house_furniture_absolute_apply(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_tx;
 IF NOT (r.effects ? 'house_absolute') THEN RETURN; END IF;
 IF r.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'absolute Rotate requires original action transaction' USING ERRCODE='23514'; END IF;
 UPDATE game_spell_item_temporal_schedules SET definition_production_key=coalesce(r.effects->'rotate_after',r.effects->'house_after')->>'key',
  target_production_key=r.effects->'house_absolute'->>'target'
  WHERE item_instance_id=r.source_item_instance_id AND to_jsonb(game_spell_item_temporal_schedules)=r.effects->'house_absolute'->'before';
 IF NOT FOUND THEN RAISE EXCEPTION 'absolute Rotate source schedule changed' USING ERRCODE='23514'; END IF;
END $$;
DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY['game_house_furniture_absolute_proposal(game_item_consume_transform_records)'::regprocedure,
  'game_house_furniture_absolute_guard()'::regprocedure,'game_house_furniture_absolute_apply(uuid)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;

CREATE FUNCTION game_house_furniture_absolute_complete(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT NOT(r.effects ? 'house_absolute') OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
  WHERE p.item_instance_id=r.source_item_instance_id AND to_jsonb(p)=
   (r.effects->'house_absolute'->'before')||jsonb_build_object(
    'definition_production_key',coalesce(r.effects->'rotate_after',r.effects->'house_after')->>'key',
    'target_production_key',r.effects->'house_absolute'->>'target'))
$$;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_item_action_house_admit(game_item_consume_transform_records)'::regprocedure);
 needle:='OR game_house_furniture_timer_proposal(r) IS DISTINCT FROM TRUE';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing House timer proposal'; END IF;
 EXECUTE replace(body,needle,needle||' OR game_house_furniture_absolute_proposal(r) IS DISTINCT FROM TRUE');
 body:=pg_get_functiondef('game_item_action_house_complete(UUID)'::regprocedure);
 needle:='OR game_item_action_house_custody(r) IS DISTINCT FROM TRUE';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing House custody completion'; END IF;
 EXECUTE replace(body,needle,needle||' OR game_house_furniture_absolute_complete(r) IS DISTINCT FROM TRUE');
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_absolute_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_absolute_complete(game_item_consume_transform_records) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_absolute_complete(game_item_consume_transform_records) TO oteryn_game_runtime;
