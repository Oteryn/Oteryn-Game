-- Native fluid ownership remains in 0560 and QBASE1200. Quest selectors add
-- exact subtype predicates; source rewards bind the existing MINT fluid plan.
CREATE FUNCTION game_quest_reward_input_fluid_matches(p_item UUID,p_expected JSONB)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT NOT (p_expected ? 'fluid_subtype') OR coalesce((
  SELECT fluid_subtype FROM game_item_fluid_states WHERE item_instance_id=p_item
 ),0)=(p_expected->>'fluid_subtype')::smallint
$$;
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
 original:=$old$AND c.before_item->>'definition_production_key'=expected->>'item'$old$;
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'quest fluid input sum anchor changed'; END IF;
 body:=replace(body,original,original||' AND game_quest_reward_input_fluid_matches(c.item_instance_id,expected)');
 original:=$old$WHERE e->>'item'=c.before_item->>'definition_production_key'$old$;
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'quest unauthored fluid input anchor changed'; END IF;
 EXECUTE replace(body,original,original||' AND game_quest_reward_input_fluid_matches(c.item_instance_id,e)');
END $migration$;
CREATE FUNCTION game_quest_reward_fluids_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual JSONB; part JSONB; ordinal BIGINT; outputs JSONB; field TEXT; plan_column TEXT; value_key TEXT;
BEGIN
 IF NEW.mint_transaction_id IS NULL THEN RETURN NULL; END IF;
 -- Both existing native instance-initialization owners remain source-bound.
 -- Missing authored metadata requires an empty native plan, not an implicit grant.
 FOR field,plan_column,value_key IN VALUES ('fluid_subtype','fluid_plan','subtype'),('charges_remaining','charges_plan','charges_remaining') LOOP
  expected:='[]';
  IF NEW.plan->'give' ? field THEN
   expected:=expected||jsonb_build_array(jsonb_build_object('ordinal',0,value_key,(NEW.plan->'give'->>field)::integer));
  END IF;
  outputs:=CASE WHEN jsonb_array_length(coalesce(NEW.plan #> '{give,backpack_outputs}','[]'))>0
   THEN NEW.plan #> '{give,backpack_outputs}' ELSE coalesce(NEW.plan #> '{give,contents}','[]') END;
  FOR part,ordinal IN SELECT value,number FROM jsonb_array_elements(outputs) WITH ORDINALITY AS output(value,number) LOOP
   IF part ? field THEN expected:=expected||jsonb_build_array(jsonb_build_object('ordinal',ordinal,value_key,(part->>field)::integer)); END IF;
  END LOOP;
  SELECT to_jsonb(m)->plan_column INTO actual FROM game_reward_claim_mint_receipts m
   WHERE transaction_id=NEW.mint_transaction_id AND character_id=NEW.character_id
    AND game_session_id=NEW.cause_id AND command_id=NEW.cause_ordinal AND created_xact_id=pg_current_xact_id();
  IF NOT FOUND OR actual IS DISTINCT FROM expected THEN
   RAISE EXCEPTION 'quest reward instance plan differs from exact native MINT: %',field USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_fluids_complete AFTER INSERT ON game_quest_reward_sources
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_fluids_complete();
DO $migration$ BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_reward_input_fluid_matches(UUID,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_quest_reward_fluids_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_input_fluid_matches(UUID,JSONB),game_quest_reward_fluids_complete() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_input_fluid_matches(UUID,JSONB) TO oteryn_game_runtime;
