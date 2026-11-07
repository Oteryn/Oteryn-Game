-- Current Crystal/Canary wrap code emits the generic decoration kit23398.
-- XML wrapableto qualifies source furniture; dedicated legacy wrapper profiles
-- and immutable prior receipts remain usable for genuine historical unwraps.
DO $shape$ BEGIN
 IF (SELECT count(*) FROM game_house_furniture_rules WHERE action=1)<>1774
  OR (SELECT count(DISTINCT first_item) FROM game_house_furniture_rules WHERE action=1)<>1774 THEN
 RAISE EXCEPTION 'canonical wrap source rule shape changed';END IF;
 UPDATE game_house_furniture_rules SET result_item=23398 WHERE action=1 AND result_item<>23398;
END $shape$;
ALTER FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT)
 RENAME TO game_house_furniture_wrap_pair_before_canonical_kit;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_pair_before_canonical_kit(UUID,TEXT,TEXT,SMALLINT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_house_furniture_wrap_pair(p_item UUID,p_before TEXT,p_after TEXT,p_action SMALLINT)
 RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER STRICT AS $$
 SELECT (p_action<>1 OR p_after='oteryn:item.tibia.i23398')
  AND game_house_furniture_wrap_pair_before_canonical_kit(p_item,p_before,p_after,p_action)
$$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_pair(uuid,text,text,smallint) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
