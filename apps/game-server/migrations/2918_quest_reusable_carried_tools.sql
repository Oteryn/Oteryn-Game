-- Crystal 201189f items.xml:22845-22861, lord_of_the_elements.lua:28.
-- The Lord's ashes use the same four source deadlines as the existing corpse owner.
DO $migration$
DECLARE body TEXT; anchor TEXT := 'SELECT CASE p_key';
BEGIN
 body:=pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected source corpse chain registry';
 END IF;
 EXECUTE replace(body,anchor,anchor||E'\n WHEN ''oteryn:item.tibia.i8181'' THEN ARRAY[8181,8182,8183,8184]');
 IF game_source_corpse_durations_v1('oteryn:item.tibia.i8181') IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[] THEN
  RAISE EXCEPTION 'Lord ashes deadlines differ from Crystal';
 END IF;
END $migration$;

-- The Spike detector and the obsidian knife remain in the user's custody on
-- a reusable use. The ordinary quest receipt retains the exact ingress bytes.
CREATE FUNCTION game_quest_reusable_tool_complete(p game_quest_reward_sources)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
DECLARE observation JSONB; selected_item UUID; owned_world UUID;
BEGIN
 observation:=p.plan->'carried_observation';
 IF p.created_xact_id<>pg_current_xact_id() OR p.plan->'take' IS DISTINCT FROM '[]'::JSONB
  OR jsonb_typeof(observation->'source') IS DISTINCT FROM 'array'
  OR jsonb_typeof(observation->'intent') IS DISTINCT FROM 'array'
  OR NOT ((jsonb_typeof(p.plan->'carried_tracker') IS NOT DISTINCT FROM 'object' AND p.plan->>'give' IS NULL)
   OR (p.plan #>> '{corpse_target,reusable_tool}' IS NOT DISTINCT FROM 'true'
    AND p.plan->'per_occurrence_mint' IS NOT DISTINCT FROM 'true'::JSONB AND p.mint_transaction_id IS NOT NULL))
 THEN RETURN false; END IF;
 IF jsonb_array_length(observation->'source')<>16 OR jsonb_array_length(observation->'intent') NOT BETWEEN 1 AND 4096
  OR EXISTS (SELECT 1 FROM jsonb_array_elements((observation->'source')||(observation->'intent')) bytes(b)
   WHERE CASE WHEN b::TEXT ~ '^[0-9]{1,3}$' THEN b::TEXT::INTEGER>255 ELSE true END)
 THEN RETURN false; END IF;
 SELECT string_agg(lpad(to_hex(b::INTEGER),2,'0'),'' ORDER BY n)::UUID INTO selected_item
 FROM jsonb_array_elements_text(observation->'source') WITH ORDINALITY bytes(b,n);
 SELECT i.world_id INTO owned_world FROM game_item_instances i
 WHERE i.item_instance_id=selected_item AND i.lifecycle=1 AND i.quantity>0
  AND i.definition_family='Item' AND i.definition_production_key=p.plan->>'carried_item';
 RETURN FOUND AND EXISTS(SELECT 1 FROM game_quest_callback_carried(p.character_id,owned_world,selected_item));
END $$;

DO $migration$
DECLARE body TEXT; anchor TEXT; replacement TEXT;
BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_reusable_tool_complete(game_quest_reward_sources) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 body:=pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
 anchor:='    IF jsonb_typeof(NEW.plan->''take'') IS DISTINCT FROM ''array''';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected carried USE guard';
 END IF;
 replacement:=$arm$    IF NEW.plan ? 'carried_observation' THEN
        IF NOT game_quest_reusable_tool_complete(NEW) THEN
            RAISE EXCEPTION 'quest USE lacks current reusable carried tool' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
$arm$||anchor;
 EXECUTE replace(body,anchor,replacement);

 -- Crystal skinning.lua:259-263 permits a new use on fresh ashes after the
 -- sampling storage has reached 1. The per-command mint and corpse/tool
 -- proof still require their own same-transaction receipts.
 body:=pg_get_functiondef('game_quest_reward_occurrence_effects_valid(jsonb,text[],bigint[],bigint[])'::regprocedure);
 anchor:='AND v_before IS DISTINCT FROM v_after';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected reward occurrence changed-state guard';
 END IF;
 EXECUTE replace(body,anchor,$arm$AND (v_before IS DISTINCT FROM v_after OR
  v_plan #>> '{corpse_target,reusable_tool}'='true')$arm$);

 body:=pg_get_functiondef('game_quest_corpse_sample_complete()'::regprocedure);
 anchor:='-r.occurred_at BETWEEN 0 AND sample_row.maximum_age_ms-1';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected corpse sampling clock guard';
 END IF;
 replacement:=$arm$-CASE WHEN source_row.plan #>> '{corpse_target,reusable_tool}'='true'
       THEN r.materialized_at ELSE r.occurred_at END
   BETWEEN coalesce((source_row.plan #>> '{corpse_target,minimum_age_ms}')::BIGINT,0) AND sample_row.maximum_age_ms-1$arm$;
 body:=replace(body,anchor,replacement);
 anchor:=' SELECT * INTO input_row FROM game_quest_callback_consumptions c';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected corpse sampling tool guard';
 END IF;
 replacement:=$arm$ IF source_row.plan #>> '{corpse_target,reusable_tool}'='true' THEN
  IF NOT game_quest_reusable_tool_complete(source_row) THEN
   RAISE EXCEPTION 'corpse sample lacks current reusable tool' USING ERRCODE='23514';
  END IF;
  IF (SELECT jsonb_agg(get_byte(uuid_send(sample_row.source_item),n) ORDER BY n) FROM generate_series(0,15) bytes(n))
        IS DISTINCT FROM source_row.plan #> '{carried_observation,source}'
    OR (SELECT jsonb_agg(get_byte(sample_row.ingress,n) ORDER BY n)
         FROM generate_series(0,octet_length(sample_row.ingress)-1) bytes(n))
        IS DISTINCT FROM source_row.plan #> '{carried_observation,intent}'
    OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=sample_row.source_item AND i.world_id=sample_row.world_id) THEN
   RAISE EXCEPTION 'corpse sample differs from reusable tool observation' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
$arm$||anchor;
 EXECUTE replace(body,anchor,replacement);
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reusable_tool_complete(game_quest_reward_sources) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reusable_tool_complete(game_quest_reward_sources) TO oteryn_game_runtime;
