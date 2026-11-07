-- Reuse the complete carried-recipe owner for bounded source cosmetic recipes.
-- Crystal9f5a72c mounts.lua:20-36,112-139 (seven-piece Glider).
CREATE FUNCTION game_item_cosmetic_piece_plan(
 p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,p_before JSONB,p_effects JSONB,p_extra JSONB)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 WITH profiles(triggers,ingredients,grant_key,grant_kind) AS (VALUES
  (ARRAY[12551,12552,12553,12554,12555,12556],ARRAY[12551,12552,12553,12554,12555,12556],
   'oteryn:item-grant.outfit.430-431.base','outfit'),
  (ARRAY[21897],ARRAY[21897,21898,21899,21901,21902,21905,21906],
   'oteryn:item-grant.mount.71','mount'))
 SELECT coalesce(EXISTS(SELECT 1 FROM profiles p WHERE
  ((p_kind=3 AND NOT p_consume AND p_count=0)
   OR (p_kind IN(1,4) AND p_consume AND p_count=1))
  AND p_before->>'definition_production_key'=ANY(
    ARRAY(SELECT 'oteryn:item.tibia.i'||n::text FROM unnest(p.triggers) n))
  AND p_effects->>'kind'='cosmetic'
  AND p_effects->'cosmetic'->>'consumed'='true'
  AND p_effects->'cosmetic'->>'provenance'='crystal-item-cosmetics-201189f-v1'
  AND p_effects->'cosmetic'->'progress'='null'::jsonb
  AND jsonb_array_length(p_effects->'cosmetic'->'grants')=1
  AND p_effects->'cosmetic'->'grants'->0->>'kind'=p.grant_kind
  AND p_effects->'cosmetic'->'grants'->0->>'grant_key'=p.grant_key
  AND (p_effects->'cosmetic'->'grants'->0->>'addon')::integer=0
  AND jsonb_array_length(p_extra)=cardinality(p.ingredients)-CASE WHEN p_consume THEN 1 ELSE 0 END
  AND ARRAY(SELECT key FROM (
    SELECT e->'before'->>'definition_production_key' AS key FROM jsonb_array_elements(p_extra) e
    UNION ALL SELECT p_before->>'definition_production_key' WHERE p_consume
   ) actual ORDER BY key)=ARRAY(SELECT 'oteryn:item.tibia.i'||n::text FROM unnest(p.ingredients) n ORDER BY 1)
  AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_extra) e
    WHERE (e->>'count')::integer IS DISTINCT FROM 1
     OR (e->>'source_kind')::integer NOT IN(1,4) OR e->>'source_kind' IS NULL)),FALSE)
$$;
CREATE OR REPLACE FUNCTION game_item_cosmetic_ground_pieces(
 p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,p_before JSONB,p_effects JSONB,p_extra JSONB)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT p_kind=3 AND game_item_cosmetic_piece_plan(p_kind,p_consume,p_count,p_before,p_effects,p_extra)
$$;
DO $migration$
DECLARE definition TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_extra_input_bound';
 IF left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing carried recipe bound'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_extra_input_bound;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_extra_input_bound CHECK ('
  ||substring(definition FROM 7)||' OR game_item_cosmetic_piece_plan(source_kind,consume_source,source_count,before_item,effects,extra_inputs))';
END $migration$;
DO $$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_item_cosmetic_piece_plan(integer,boolean,bigint,jsonb,jsonb,jsonb)',
  'game_item_cosmetic_ground_pieces(integer,boolean,bigint,jsonb,jsonb,jsonb)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_item_cosmetic_piece_plan(integer,boolean,bigint,jsonb,jsonb,jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_cosmetic_piece_plan(integer,boolean,bigint,jsonb,jsonb,jsonb)
 TO oteryn_game_runtime,oteryn_game_control;
