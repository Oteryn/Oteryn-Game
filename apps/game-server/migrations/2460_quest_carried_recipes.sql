-- M20: recipe plans use their own two-selected-input proof. Legacy one-input
-- tools keep their existing guard, including the later creature-tool extension.
CREATE FUNCTION game_quest_recipe_use_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE s RECORD; t RECORD; p JSONB; source_id JSONB; target_id JSONB; n BIGINT;
BEGIN
 IF NOT (NEW.plan ? 'recipe_source' OR NEW.plan ? 'carried_target') THEN RETURN NULL; END IF;
 IF NEW.created_xact_id IS DISTINCT FROM pg_current_xact_id()
   OR NEW.plan ? 'carried_item'
   OR jsonb_typeof(NEW.plan->'recipe_source') IS DISTINCT FROM 'string'
   OR jsonb_typeof(NEW.plan->'carried_target') IS DISTINCT FROM 'string'
   OR NEW.plan->>'recipe_source'=NEW.plan->>'carried_target'
   OR jsonb_typeof(NEW.plan->'take') IS DISTINCT FROM 'array'
   OR jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object' THEN
  RAISE EXCEPTION 'recipe lacks its selected-input plan' USING ERRCODE='23514';
 END IF;
 IF jsonb_array_length(NEW.plan->'take') NOT BETWEEN 2 AND 16
   OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') i
     WHERE i->>'item'=NEW.plan->>'recipe_source' AND i->>'count'='1')
   OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') i
     WHERE i->>'item'=NEW.plan->>'carried_target' AND i->>'count'='1') THEN
  RAISE EXCEPTION 'recipe lacks both declared operands' USING ERRCODE='23514';
 END IF;
 SELECT count(*) INTO n FROM (SELECT 1 FROM game_quest_callback_consumptions c
   WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
     AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key LIMIT 501) bounded;
 IF n NOT BETWEEN 2 AND 500 THEN
  RAISE EXCEPTION 'recipe input bound' USING ERRCODE='23514';
 END IF;
 SELECT count(*) INTO n FROM game_quest_callback_consumptions c
   WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
     AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
     AND c.before_item->>'definition_production_key'=NEW.plan->>'recipe_source';
 IF n<>1 THEN RAISE EXCEPTION 'recipe selected source is ambiguous' USING ERRCODE='23514'; END IF;
 SELECT c.* INTO s FROM game_quest_callback_consumptions c
   WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
     AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
     AND c.before_item->>'definition_production_key'=NEW.plan->>'recipe_source';
 SELECT count(*) INTO n FROM game_quest_callback_consumptions c
   WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
     AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
     AND c.before_item->>'definition_production_key'=NEW.plan->>'carried_target';
 IF n<>1 THEN RAISE EXCEPTION 'recipe selected target is ambiguous' USING ERRCODE='23514'; END IF;
 SELECT c.* INTO t FROM game_quest_callback_consumptions c
   WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
     AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
     AND c.before_item->>'definition_production_key'=NEW.plan->>'carried_target';
 IF s.item_instance_id=t.item_instance_id OR s.quantity<>1 OR t.quantity<>1
   OR s.replacement IS NOT NULL OR t.replacement IS NOT NULL
   OR s.location_kind='ground' OR t.location_kind='ground'
   OR s.created_xact_id IS DISTINCT FROM pg_current_xact_id()
   OR t.created_xact_id IS DISTINCT FROM pg_current_xact_id() THEN
  RAISE EXCEPTION 'recipe requires two exact carried operands' USING ERRCODE='23514';
 END IF;
 p:=convert_from(s.proof,'UTF8')::jsonb;
 SELECT jsonb_agg(get_byte(uuid_send(s.item_instance_id),i) ORDER BY i) INTO source_id FROM generate_series(0,15) i;
 SELECT jsonb_agg(get_byte(uuid_send(t.item_instance_id),i) ORDER BY i) INTO target_id FROM generate_series(0,15) i;
 IF p->'reward_callback' IS DISTINCT FROM NEW.plan
   OR p #> '{carried_use,source}' IS DISTINCT FROM source_id
   OR p #> '{carried_use,recipe_target}' IS DISTINCT FROM target_id
   OR jsonb_typeof(p #> '{carried_use,intent}') IS DISTINCT FROM 'array' THEN
  RAISE EXCEPTION 'recipe ingress differs from selected operands' USING ERRCODE='23514';
 END IF;
 IF jsonb_array_length(p #> '{carried_use,intent}') NOT BETWEEN 1 AND 4096
   OR EXISTS(SELECT 1 FROM jsonb_array_elements(p #> '{carried_use,intent}') value
     WHERE CASE WHEN value::text ~ '^[0-9]{1,3}$' THEN value::text::integer NOT BETWEEN 0 AND 255 ELSE true END)
   OR EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
     WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
       AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
       AND (convert_from(c.proof,'UTF8')::jsonb->'reward_callback' IS DISTINCT FROM NEW.plan
         OR convert_from(c.proof,'UTF8')::jsonb->'carried_use' IS DISTINCT FROM p->'carried_use')) THEN
  RAISE EXCEPTION 'recipe input proofs disagree' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_recipe_use_consistent
 AFTER INSERT ON game_quest_reward_sources DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_quest_recipe_use_complete();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_recipe_use_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_recipe_use_complete() FROM PUBLIC;
