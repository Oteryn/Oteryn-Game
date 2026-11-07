-- Pinned Crystal mounts.lua:20-36,111-140 consumes blueprint21897 and six
-- separate parts. The general five-extra bound remains unchanged for all other recipes.
CREATE FUNCTION game_srv7_glider_six_inputs(p JSONB) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT CASE WHEN jsonb_typeof(p) IS DISTINCT FROM 'array' OR jsonb_array_length(p)<>6 THEN FALSE ELSE
  (SELECT count(DISTINCT value->>'item')=6
   AND bool_and(((value->>'count')::numeric=1 AND (value->>'source_kind')::numeric IN(1,4)
    AND value->'before'->>'definition_family'='Item') IS TRUE)
   AND array_agg(value->'before'->>'definition_production_key' ORDER BY value->'before'->>'definition_production_key')=
    ARRAY['oteryn:item.tibia.i21898','oteryn:item.tibia.i21899','oteryn:item.tibia.i21901',
     'oteryn:item.tibia.i21902','oteryn:item.tibia.i21905','oteryn:item.tibia.i21906']::TEXT[]
   FROM jsonb_array_elements(p)) IS TRUE END
$$;
REVOKE ALL ON FUNCTION game_srv7_glider_six_inputs(JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_srv7_glider_six_inputs(JSONB) TO oteryn_game_runtime,oteryn_game_control;
ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_extra_input_bound;
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_extra_input_bound
 CHECK(jsonb_typeof(extra_inputs)='array' AND (jsonb_array_length(extra_inputs)<=5 OR
  (recipe='oteryn:item-cosmetic.i21897.v1' AND consume_source AND source_count=1 AND source_kind IN(1,4)
   AND before_item->>'definition_family'='Item' AND before_item->>'definition_production_key'='oteryn:item.tibia.i21897'
   AND effects->>'kind'='cosmetic' AND outputs='[]'::jsonb AND game_srv7_glider_six_inputs(extra_inputs)) IS TRUE));
